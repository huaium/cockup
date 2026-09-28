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
    Backup(ConfigArgs),
    /// Restore files from a backup
    Restore(ConfigArgs),
    /// Run configured hooks
    Hook {
        #[command(flatten)]
        config: ConfigArgs,
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List configuration paths from Homebrew casks
    List { casks: Vec<String> },
    /// Print a commented YAML configuration template
    Template,
    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
#[derive(Args)]
pub struct ConfigArgs {
    pub config_file: PathBuf,
    #[arg(short = 'q', long)]
    pub quiet: bool,
}
