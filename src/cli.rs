use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Configuration backup and restore",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
#[derive(Subcommand)]
pub enum Commands {
    /// Back up files from a YAML configuration
    Backup(FileArgs),
    /// Restore files from a backup
    Restore(FileArgs),
    /// Check backup manifest and configured backup paths
    Verify { config_file: PathBuf },
    /// Compare a backup with the files it would restore
    Diff {
        config_file: PathBuf,
        /// Show changed paths without content differences
        #[arg(long)]
        summary: bool,
    },
    /// Run configured hooks
    Hook {
        #[command(flatten)]
        config: ConfigArgs,
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Detect configuration paths from Homebrew casks
    Detect { casks: Vec<String> },
    /// Manage downloaded ingredients
    Ingredient {
        #[command(subcommand)]
        command: IngredientCommand,
    },
    /// Print a commented YAML configuration template
    Template,
    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
#[derive(Subcommand)]
pub enum IngredientCommand {
    /// Check GitHub now; ask before downloading an uncached ingredient
    Update { name: String },
    /// Show cached ingredient dates and sources without network access
    Status { name: Option<String> },
    /// Delete cached YAML and metadata for one or more ingredients
    Delete {
        #[arg(required = true, num_args = 1..)]
        names: Vec<String>,
    },
    /// Remove the entire ingredient cache directory
    Clean,
}
#[derive(Args)]
pub struct ConfigArgs {
    pub config_file: PathBuf,
    #[arg(short = 'q', long)]
    pub quiet: bool,
}
#[derive(Args)]
pub struct FileArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
    /// Show planned file changes without writing files or running hooks
    #[arg(long)]
    pub dry_run: bool,
}
