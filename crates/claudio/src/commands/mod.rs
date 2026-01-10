pub mod add;
pub mod completion;
pub mod edit;
pub mod editor;
pub mod env;
pub mod init;
pub mod list;
pub mod run;
pub mod show;
pub mod util;

use anyhow::Result;
use std::io::{self, Write};

use claudio_core::preset::resolver::ResolverConfig;
use claudio_core::preset::resolver::{ConfirmCommand, ConfirmCommandRequest};
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;

pub(crate) struct InteractiveConfirmCommand;

static INTERACTIVE_CONFIRMER: InteractiveConfirmCommand = InteractiveConfirmCommand;

impl ConfirmCommand for InteractiveConfirmCommand {
    fn confirm_command(&self, request: ConfirmCommandRequest<'_>) -> Result<bool> {
        println!("\nPreset wants to execute command:");
        println!("  {}", request.command);
        print!("Allow? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        Ok(input.trim().eq_ignore_ascii_case("y"))
    }
}

pub(crate) fn effective_read_scope(scope: Scope) -> Scope {
    let ignore_user = settings_loader::should_ignore_user_presets().unwrap_or(false);

    if scope == Scope::Auto && ignore_user {
        Scope::Project
    } else {
        scope
    }
}

pub(crate) fn build_resolver_config(scope: Scope) -> ResolverConfig<'static> {
    // Option A: if settings are missing/unreadable, default to requiring confirmation.
    let skip_command_confirmation = settings_loader::load_settings(scope)
        .ok()
        .flatten()
        .and_then(|s| s.skip_command_confirmation)
        .unwrap_or(false);

    ResolverConfig {
        skip_command_confirmation,
        confirm_command: Some(&INTERACTIVE_CONFIRMER),
    }
}
