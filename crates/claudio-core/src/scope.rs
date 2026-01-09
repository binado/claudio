/// Configuration scope for preset/settings discovery.
///
/// This type is intentionally CLI-agnostic (no `clap` derives) so it can live in `claudio-core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Use the default resolution behavior (project shadows user for reads).
    Auto,
    /// Only use project-local presets from `./.claudio/presets`.
    Project,
    /// Only use user presets from `~/.claudio/presets`.
    User,
}
