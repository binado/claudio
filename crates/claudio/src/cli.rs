use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use claudio_core::scope::Scope as CoreScope;

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum Scope {
    /// Use the default resolution behavior (project shadows user for reads).
    Auto,
    /// Only use project-local presets from `./.claudio/presets`.
    Project,
    /// Only use user presets from `~/.claudio/presets`.
    User,
}

impl From<Scope> for CoreScope {
    fn from(value: Scope) -> Self {
        match value {
            Scope::Auto => CoreScope::Auto,
            Scope::Project => CoreScope::Project,
            Scope::User => CoreScope::User,
        }
    }
}

#[derive(Debug, Clone, Parser)]
pub struct ScopeArgs {
    /// Where to read/write presets from.
    #[arg(long, value_enum, default_value_t = Scope::Auto)]
    pub scope: Scope,
}

#[derive(Debug, Clone, Parser)]
pub struct RunArgs {
    /// Preset name to use (defaults to "default" preset if available)
    pub preset: Option<String>,

    #[command(flatten)]
    pub scope: ScopeArgs,

    /// Require a preset (error if none available instead of running bare claude)
    #[arg(long)]
    pub require_preset: bool,

    /// Print the command that would be executed without running it
    #[arg(long)]
    pub dry_run: bool,

    /// Additional arguments to pass to claude
    #[arg(last = true, allow_hyphen_values = true)]
    pub claude_args: Vec<String>,
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
    pub command: Option<Commands>,

    /// Flattened run args for shorthand syntax (claudio my-preset)
    #[command(flatten)]
    pub run_args: RunArgs,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run Claude Code with a specific preset
    Run(RunArgs),
    /// Initialize a project-local preset directory at `<project-root>/.claudio/presets`
    Init {
        #[command(flatten)]
        scope: ScopeArgs,
    },
    /// Manage presets (list, show, edit, add, env)
    Preset {
        #[command(subcommand)]
        command: PresetCommands,
    },
    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
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
    /// Create a new preset
    Add {
        /// Name for the new preset
        preset: String,

        /// Base preset to extend from
        #[arg(long = "extends", alias = "extend")]
        extends: Option<String>,

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
