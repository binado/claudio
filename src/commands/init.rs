use crate::cli::Scope;
use crate::preset::loader;
use anyhow::{Context, Result};

pub fn init(scope: Scope) -> Result<()> {
    // For init, we support initializing either the project directory or the
    // user directory. If the user passes `auto`, we prefer project-local when
    // a project root is detected, otherwise fall back to user directory.
    let preset_dir = loader::get_preset_write_dir_scoped(scope)?;

    std::fs::create_dir_all(&preset_dir).with_context(|| {
        format!(
            "Failed to create preset directory: {}",
            preset_dir.display()
        )
    })?;

    Ok(())
}
