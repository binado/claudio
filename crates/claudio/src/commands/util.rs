use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;

pub(crate) fn resolve_claude_executable(
    claude_executable_cli: Option<&str>,
    scope: Scope,
) -> String {
    // Resolve claude executable with precedence: CLI > Env > Settings > Default
    claude_executable_cli
        .map(|s| s.to_string())
        .or_else(|| std::env::var("CLAUDIO_CLAUDE_EXECUTABLE").ok())
        .or_else(|| {
            settings_loader::resolve_claude_executable(scope)
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| "claude".to_string())
}
