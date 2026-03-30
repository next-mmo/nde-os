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
        CommandBuilder::new("sh")
    };
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
