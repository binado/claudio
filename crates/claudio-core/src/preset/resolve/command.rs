use anyhow::{Context, Result};
use std::process::Command;

/// CLI-provided interface for confirming command execution.
///
/// `claudio-core` does not perform terminal I/O. If a preset contains a
/// command-backed env value and confirmation is required, the caller must
/// provide a `ConfirmCommand` implementation via [`ResolverConfig`].
pub trait ConfirmCommand {
    fn confirm_command(&self, request: ConfirmCommandRequest<'_>) -> Result<bool>;
}

#[derive(Debug, Clone, Copy)]
pub struct ConfirmCommandRequest<'a> {
    pub command: &'a str,
}

#[derive(Clone, Copy, Default)]
pub struct ResolverConfig<'a> {
    /// If true, command-backed env values do not require confirmation unless
    /// the preset explicitly sets `confirm: true`.
    pub skip_command_confirmation: bool,

    /// Confirmation handler.
    ///
    /// If confirmation is required and this is `None`, resolution fails with an error.
    pub confirm_command: Option<&'a dyn ConfirmCommand>,
}

/// Execute a command and return its stdout.
///
/// If confirmation is required and [`ResolverConfig::confirm_command`] is `None`,
/// this function returns an error rather than prompting.
pub(crate) fn execute_command(
    env_key: &str,
    command: &str,
    confirm: Option<bool>,
    cfg: &ResolverConfig<'_>,
) -> Result<String> {
    let should_confirm = should_confirm_command(confirm, cfg);

    if should_confirm {
        let Some(confirmer) = cfg.confirm_command else {
            anyhow::bail!(
                "Command confirmation required for env key '{}' but no confirmer was provided.\n\n\
This is a safety feature: claudio-core does not prompt for input.\n\
Provide a ConfirmCommand implementation (CLI) or set 'skip_command_confirmation' to true in settings.",
                env_key
            );
        };

        let allowed = confirmer.confirm_command(ConfirmCommandRequest { command })?;
        if !allowed {
            anyhow::bail!("Command execution denied");
        }
    }

    let (shell, shell_arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let output = Command::new(shell)
        .arg(shell_arg)
        .arg(command)
        .output()
        .with_context(|| format!("Failed to execute command: {}", command))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Command failed with exit code {:?}: {}\n\nstderr:\n{}",
            output.status.code(),
            command,
            stderr.trim_end()
        );
    }

    Ok(String::from_utf8(output.stdout)?.trim_end().to_string())
}

fn should_confirm_command(confirm: Option<bool>, cfg: &ResolverConfig<'_>) -> bool {
    if let Some(confirm_value) = confirm {
        return confirm_value;
    }

    !cfg.skip_command_confirmation
}
