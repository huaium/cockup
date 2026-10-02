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
    Restore(RestoreArgs),
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
    /// Find ingredients in the GitHub library
    Ingredient {
        #[command(subcommand)]
        command: IngredientCommand,
    },
    /// Manage the ingredient and library-list caches
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
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
    /// Search available ingredients in the GitHub library
    Search { query: String },
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Show cache location, contents, and freshness without network access
    Status,
    /// List downloaded ingredients without network access
    List,
    /// Show one downloaded ingredient's cache metadata
    Show { name: String },
    /// Refresh one ingredient, all downloaded ingredients, the catalog, or both
    Refresh {
        /// Refresh this ingredient (ask before downloading when absent)
        name: Option<String>,
        /// Refresh all downloaded ingredients only
        #[arg(long, conflicts_with_all = ["name", "catalog"])]
        ingredients: bool,
        /// Refresh the cached GitHub library list only
        #[arg(long, conflicts_with = "name")]
        catalog: bool,
    },
    /// Delete cached ingredients or the library list
    Delete {
        names: Vec<String>,
        /// Delete only the cached library list
        #[arg(long, conflicts_with_all = ["names", "ingredients"], required_unless_present_any = ["names", "ingredients"])]
        catalog: bool,
        /// Delete all downloaded ingredient YAML and metadata caches
        #[arg(long, conflicts_with = "names")]
        ingredients: bool,
    },
    /// Remove the entire Cockup cache directory
    Clean,
}
#[derive(Args)]
pub struct ConfigArgs {
    pub config_file: PathBuf,
    /// Run configured hooks without asking for confirmation
    #[arg(short = 'a', long)]
    pub approve_hooks: bool,
}
#[derive(Args)]
pub struct FileArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
    /// Show planned file changes without writing files or running hooks
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args)]
pub struct RestoreArgs {
    #[command(flatten)]
    pub file: FileArgs,
    /// Replace existing local files without asking
    #[arg(short = 'o', long, conflicts_with = "skip_existing")]
    pub r#override: bool,
    /// Keep existing local files and restore only missing paths
    #[arg(short = 's', long)]
    pub skip_existing: bool,
}
