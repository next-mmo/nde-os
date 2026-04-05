//! RVC (Retrieval-based Voice Conversion) service.
//!
//! Two-step pipeline: TTS → RVC post-process.
//! Wraps the RVC CLI invocation with proper argument handling.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::runtime::{resolve_python, run_checked_command, VoiceRuntime};
use super::types::{RvcConvertRequest, RvcConvertResult, VoiceModel};

/// Run RVC voice conversion on an input audio file.
pub fn convert(request: &RvcConvertRequest, runtime: &VoiceRuntime) -> Result<RvcConvertResult> {
    runtime.with_runtime_path(|| {
        let python = request
            .python_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(resolve_python)
            .ok_or_else(|| anyhow!("Python not found for RVC CLI invocation"))?;

        let cli_path = request
            .cli_path
            .as_ref()
            .ok_or_else(|| anyhow!("RVC CLI path is required"))?;

        let out_path = match &request.output_path {
            Some(p) => PathBuf::from(p),
            None => {
                let input = Path::new(&request.input_audio);
                input.with_extension("rvc.mp3")
            }
        };

        // Ensure parent dir exists
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let mut command = Command::new(&python);
        command.arg(cli_path);
        command.arg("tts_infer");
        // RVC uses the input audio as the TTS output path
        command.arg("--output_tts_path").arg(&request.input_audio);
        command.arg("--output_rvc_path").arg(&out_path);
        command.arg("--pth_path").arg(&request.model_path);
        command.arg("--index_path").arg(&request.index_path);
        command.arg("--export_format").arg(&request.output_format);

        if let Some(pitch_shift) = request.pitch_shift {
            command.arg("--pitch").arg(pitch_shift.to_string());
        }

        run_checked_command(command, "RVC voice conversion")?;

        Ok(RvcConvertResult {
            output_path: out_path.to_string_lossy().to_string(),
            model_path: request.model_path.clone(),
        })
    })
}

/// Scan a directory for RVC models (`.pth` files with optional `.index` siblings).
pub fn list_models(models_dir: &Path) -> Result<Vec<VoiceModel>> {
    if !models_dir.exists() {
        return Ok(Vec::new());
    }

    let mut models = Vec::new();
    for entry in std::fs::read_dir(models_dir)
        .with_context(|| format!("failed to read RVC models dir: {}", models_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("pth") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let index_path = path.with_extension("index");
            models.push(VoiceModel {
                name,
                model_path: path.to_string_lossy().to_string(),
                index_path: if index_path.exists() {
                    Some(index_path.to_string_lossy().to_string())
                } else {
                    None
                },
            });
        }
    }

    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_models_empty_dir() {
        let dir = std::env::temp_dir().join("nde-rvc-test-empty");
        std::fs::create_dir_all(&dir).ok();
        let result = list_models(&dir).unwrap();
        assert!(result.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_models_nonexistent_dir() {
        let result = list_models(Path::new("/tmp/nde-rvc-nonexistent-dir-xyz"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
