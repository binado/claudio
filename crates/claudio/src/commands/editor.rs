use anyhow::{Context, Result};
use claudio_core::preset::loader;
use std::env;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Opens the specified preset file in the user's preferred editor.
///
/// The editor is determined by checking the `VISUAL` and `EDITOR` environment variables,
/// falling back to `notepad` on Windows or `vi` on other platforms.
///
/// After the editor closes successfully, the preset file is validated.
pub fn open_in_editor(preset_path: &std::path::Path) -> Result<()> {
    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

    let status = Command::new(&editor)
        .arg(preset_path)
        .status()
        .with_context(|| format!("Failed to open editor '{}'", editor))?;

    if status.success() {
        let preset = loader::load_preset(preset_path)
            .with_context(|| format!("Preset file is invalid: {}", preset_path.display()))?;

        let validation = preset.validate(Some(preset_path));

        if !validation.is_valid() {
            for warning in &validation.warnings {
                eprintln!("Warning: {}", warning);
            }
            for error in &validation.errors {
                eprintln!("Error: {}", error);
            }
            anyhow::bail!("Preset validation failed");
        }
    } else {
        let detail = if let Some(code) = status.code() {
            format!("exit code {}", code)
        } else {
            #[cfg(unix)]
            {
                status
                    .signal()
                    .map(|sig| format!("signal {}", sig))
                    .unwrap_or_else(|| "unknown termination".to_string())
            }
            #[cfg(not(unix))]
            {
                "unknown termination".to_string()
            }
        };

        anyhow::bail!(
            "Editor '{}' failed ({})\n\nYou can change the editor by setting the VISUAL or EDITOR environment variable.",
            editor,
            detail
        );
    }

    Ok(())
}
