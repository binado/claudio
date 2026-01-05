use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Scope {
    /// Use the default resolution behavior (project shadows user for reads).
    Auto,
    /// Only use project-local presets from `./.claudio/presets`.
    Project,
    /// Only use user presets from `~/.claudio/presets`.
    User,
}

#[derive(Debug, Clone, Parser)]
pub struct ScopeArgs {
    /// Where to read/write presets from.
    #[arg(long, value_enum, default_value_t = Scope::Auto)]
    pub scope: Scope,
}

#[derive(Parser)]
#[command(name = "claudio")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run Claude Code with a specific preset
    Run {
        /// Preset name to use
        preset: String,

        #[command(flatten)]
        scope: ScopeArgs,

        /// Additional arguments to pass to claude
        #[arg(last = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
    /// List all available presets (optionally filter by name)
    List {
        /// Optional preset name to filter by (prints matching preset file paths)
        name: Option<String>,

        #[command(flatten)]
        scope: ScopeArgs,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show details of a specific preset
    Show {
        /// Preset name to show
        preset: String,

        #[command(flatten)]
        scope: ScopeArgs,

        /// Show resolved preset
        #[arg(long)]
        resolved: bool,

        /// Output raw JSON only
        #[arg(long)]
        json: bool,
    },
    /// Edit a preset in your default editor
    Edit {
        /// Preset name to edit
        preset: String,

        #[command(flatten)]
        scope: ScopeArgs,
    },
    /// Initialize a project-local preset directory at `<project-root>/.claudio/presets`
    Init {
        #[command(flatten)]
        scope: ScopeArgs,
    },
    /// Print environment variables for a preset
    Env {
        /// Preset name
        preset: String,

        #[command(flatten)]
        scope: ScopeArgs,

        /// Format output for shell export
        #[arg(long)]
        export: bool,

        /// Show resolved values
        #[arg(long)]
        resolved: bool,
    },
}
