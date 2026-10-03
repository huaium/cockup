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
    Verify {
        /// Path to the YAML configuration
        config_file: PathBuf,
    },
    /// Compare a backup with the files it would restore
    Diff {
        /// Path to the YAML configuration
        config_file: PathBuf,
        /// Show changed paths without content differences
        #[arg(long)]
        summary: bool,
    },
    /// Run configured hooks
    Hook {
        #[command(flatten)]
        config: ConfigArgs,
        /// Run a named hook. Select hooks interactively when omitted
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Detect configuration paths from Homebrew casks
    Detect {
        /// Homebrew cask names. Detect all installed casks when omitted
        casks: Vec<String>,
    },
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
    /// Print an ingredient's YAML, downloading and caching it when needed
    Show {
        /// Ingredient name from the GitHub library
        name: String,
    },
    /// Search available ingredients in the GitHub library
    Search {
        /// Case-insensitive ingredient name search (quote queries containing spaces)
        query: String,
    },
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Show cache location, contents, and freshness without network access
    Status,
    /// List downloaded ingredients without network access
    List,
    /// Show one downloaded ingredient's cache metadata
    Show {
        /// Name of the cached ingredient
        name: String,
    },
    /// Refresh one ingredient, all downloaded ingredients, the catalog, or both
    #[command(
        override_usage = "cockup cache refresh [NAME|--ingredients|--catalog]",
        after_help = "With no choice, refresh all downloaded ingredients and the catalog.",
        group(clap::ArgGroup::new("scope").multiple(false).args(["name", "ingredients", "catalog"]))
    )]
    Refresh {
        /// Refresh this ingredient (ask before downloading when absent)
        name: Option<String>,
        /// Refresh all downloaded ingredients only
        #[arg(long)]
        ingredients: bool,
        /// Refresh the cached GitHub library list only
        #[arg(long)]
        catalog: bool,
    },
    /// Delete cached ingredients or the library list
    #[command(group(clap::ArgGroup::new("target")
        .required(true)
        .multiple(false)
        .args(["names", "ingredients", "catalog"])))]
    Delete {
        /// One or more cached ingredient names
        names: Vec<String>,
        /// Delete only the cached library list
        #[arg(long)]
        catalog: bool,
        /// Delete all downloaded ingredient YAML and metadata caches
        #[arg(long)]
        ingredients: bool,
    },
    /// Remove the entire Cockup cache directory
    Clean,
}
#[derive(Args)]
pub struct ConfigArgs {
    /// Path to the YAML configuration
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
#[command(
    override_usage = "cockup restore [OPTIONS] [--override|--skip-existing] <CONFIG_FILE>",
    after_help = "By default, ask before replacing conflicting local files and symlinks.",
    group(clap::ArgGroup::new("existing_files").multiple(false).args(["override", "skip_existing"]))
)]
pub struct RestoreArgs {
    #[command(flatten)]
    pub file: FileArgs,
    /// Replace existing local files without asking
    #[arg(short = 'o', long)]
    pub r#override: bool,
    /// Keep existing local files and restore only missing paths
    #[arg(short = 's', long)]
    pub skip_existing: bool,
}
