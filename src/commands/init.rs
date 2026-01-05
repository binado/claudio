use crate::cli::Scope;
use crate::preset::loader;
use anyhow::{Context, Result};

pub fn init(scope: Scope) -> Result<()> {
    // For init, we only meaningfully support initializing the project directory.
    // If the user passes `auto`, we treat it as `project` to match expectations:
    // "init" should set up a project-local directory.
    let scope = match scope {
        Scope::Auto => Scope::Project,
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
