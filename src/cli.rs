use clap::{Parser, Subcommand};

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

        /// Additional arguments to pass to claude
        #[arg(last = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
    /// List all available presets
    List {
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show details of a specific preset
    Show {
        /// Preset name to show
        preset: String,

        /// Show resolved preset
        #[arg(long)]
        resolved: bool,
    },
    /// Edit a preset in your default editor
    Edit {
        /// Preset name to edit
        preset: String,
    },
    /// Show the file path of a preset
    Which {
        /// Preset name to locate
        preset: String,
    },
    /// Print environment variables for a preset
    Env {
        /// Preset name
        preset: String,

        /// Format output for shell export
        #[arg(long)]
        export: bool,

        /// Show resolved values
        #[arg(long)]
        resolved: bool,
    },
}
