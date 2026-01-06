use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
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
    /// Highlight color (cyan, green, yellow, blue, magenta)
    #[arg(long, global = true, default_value = "cyan")]
    pub color: String,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run Claude Code with a specific preset
    Run {
        /// Preset name to use (defaults to "default" preset if available)
        preset: Option<String>,

        #[command(flatten)]
        scope: ScopeArgs,

        /// Require a preset (error if none available instead of running bare claude)
        #[arg(long)]
        require_preset: bool,

        /// Additional arguments to pass to claude
        #[arg(last = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
    /// Initialize a project-local preset directory at `<project-root>/.claudio/presets`
    Init {
        #[command(flatten)]
        scope: ScopeArgs,
    },
    /// Manage presets (list, show, edit, env)
    Preset {
        #[command(subcommand)]
        command: PresetCommands,
    },
}

#[derive(Subcommand)]
pub enum PresetCommands {
    /// List all available presets (optionally filter by name)
    List {
        /// Optional preset name to filter by (prints matching preset file paths)
        name: Option<String>,

        #[command(flatten)]
        scope: ScopeArgs,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Custom fields to display (comma-separated or multiple flags)
        /// Available: name, description, filepath, env, args, extends, prompt
        /// Use "all" to display all available fields
        #[arg(long, value_delimiter = ',')]
        fields: Option<Vec<String>>,

        /// Maximum length for prompt field before truncation (0 = no truncation)
        #[arg(long, default_value_t = 30)]
        prompt_max_length: usize,
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
