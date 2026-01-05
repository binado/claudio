use crate::cli::Scope;
use crate::preset::loader;
use anyhow::{Context, Result};

pub fn init(scope: Scope) -> Result<()> {
    // For init, we support initializing either the project directory or the
    // user directory. If the user passes `auto`, we treat it as `user` so that
    // `claudio init --scope auto` works even when no project root is detected.
    let scope = match scope {
        Scope::Auto => Scope::User,
        other => other,
    };

    let preset_dir = match scope {
        Scope::Project => loader::get_preset_write_dir_scoped(Scope::Project)?,
        Scope::User => loader::get_preset_write_dir_scoped(Scope::User)?,
        Scope::Auto => unreachable!("handled above"),
    };

    std::fs::create_dir_all(&preset_dir).with_context(|| {
        format!(
            "Could not create preset directory: {}",
            preset_dir.display()
        )
    })?;

    Ok(())
}
