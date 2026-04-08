use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

pub struct PtyState {
    pub writers: Mutex<HashMap<String, Box<dyn Write + Send>>>,
}

impl PtyState {
    pub fn new() -> Self {
        Self {
            writers: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub async fn spawn_pty(
    id: String,
    cwd: String,
    app: AppHandle,
    state: State<'_, PtyState>,
    app_state: State<'_, crate::state::AppState>,
) -> Result<(), String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let mut cmd = if cfg!(windows) {
        CommandBuilder::new("cmd.exe")
    } else {
        let mut b = CommandBuilder::new("bash");

        if let Ok(sandbox) = ai_launcher_core::sandbox::Sandbox::new(&app_state.base_dir) {
            let sandbox_root = sandbox.root().to_string_lossy().to_string();
            let rc_content = format!(
                r#"
export PS1="nde-os:\w\$ "

cd() {{
    local target="${{1:-$HOME}}"
    # Use python to resolve the path explicitly 
    local abs_target
    abs_target=$(python3 -c "import os; print(os.path.abspath('$target'))" 2>/dev/null)
    
    # Deny jumping outside the sandbox or common temp dirs
    if [[ -n "$abs_target" && "$abs_target" != "{sandbox_root}"* && "$abs_target" != "/var/folders"* && "$abs_target" != "/tmp"* ]]; then
        echo "nde-os: cd: $target: Permission denied (sandbox workspace strictly enforced)"
        return 1
    fi
    builtin cd "$target"
}}

ls() {{
    for arg in "$@"; do
        if [[ ! "$arg" == -* && "$arg" == /* ]]; then
            if [[ "$arg" != "{sandbox_root}"* && "$arg" != "/tmp"* && "$arg" != "/var/folders"* ]]; then
                 echo "nde-os: ls: $arg: Permission denied"
                 return 1
            fi
        fi
    done
    command ls "$@"
}}
"#
            );
            let rc_path = sandbox.root().join(".nde_rc");
            let _ = std::fs::write(&rc_path, rc_content);
            b.args(["--noprofile", "--rcfile", &rc_path.to_string_lossy()]);
        }
        b
    };

    // Inject the NDE-OS strict jail environment variables.
    // This forcibly remaps HOME, TMPDIR, and configs directly to the sandbox root!
    if let Ok(sandbox) = ai_launcher_core::sandbox::Sandbox::new(&app_state.base_dir) {
        let _ = sandbox.init_workspace();
        for (k, v) in sandbox.env_vars() {
            cmd.env(k, v);
        }

        // Auto-activate the sandbox Python virtual environment
        let current_path = std::env::var("PATH").unwrap_or_default();
        let venv_bin = if cfg!(windows) {
            sandbox.root().join(".venv").join("Scripts")
        } else {
            sandbox.root().join(".venv").join("bin")
        };
        let sys_path_sep = if cfg!(windows) { ";" } else { ":" };
        let isolated_path = format!("{}{}{}", venv_bin.display(), sys_path_sep, current_path);

        cmd.env("PATH", isolated_path);
    }

    cmd.cwd(cwd);

    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    state.writers.lock().unwrap().insert(id.clone(), writer);

    let id_clone = id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            let bytes = buf[..n].to_vec();
            let _ = app.emit(&format!("pty_read_{}", id_clone), bytes);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn write_pty(id: String, data: String, state: State<'_, PtyState>) -> Result<(), String> {
    if let Some(writer) = state.writers.lock().unwrap().get_mut(&id) {
        writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
