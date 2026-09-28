mod brew;
mod cli;
mod config;
mod files;
mod hooks;
mod manifest;
mod report;
use clap::{CommandFactory, Parser};
use cli::{Commands, ConfigArgs};
fn configured(args: &ConfigArgs, dry_run: bool) -> Result<Option<config::Config>, String> {
    let cfg = config::load(&args.config_file)?;
    if !dry_run && !args.quiet && cfg.all_hooks().next().is_some() {
        report::warning(
            "Hooks detected in configuration. Ensure commands are safe before execution.",
        );
        if !report::confirm()? {
            return Ok(None);
        }
    }
    Ok(Some(cfg))
}
fn run() -> Result<(), String> {
    if std::env::args_os().len() == 1 {
        cli::Cli::command()
            .print_help()
            .map_err(|e| e.to_string())?;
        println!();
        return Ok(());
    }
    match cli::Cli::parse().command {
        Commands::Backup(args) => {
            if let Some(cfg) = configured(&args.config, args.dry_run)? {
                files::execute(&cfg, false, args.dry_run)?;
            }
        }
        Commands::Restore(args) => {
            if let Some(cfg) = configured(&args.config, args.dry_run)? {
                files::execute(&cfg, true, args.dry_run)?;
            }
        }
        Commands::Hook { config, name } => {
            if let Some(cfg) = configured(&config, false)? {
                hooks::select(&cfg, name.as_deref())?;
            }
        }
        Commands::List { casks } => brew::list(casks)?,
        Commands::Template => print!("{}", include_str!("template.yaml")),
        Commands::Completions { shell } => {
            let mut command = cli::Cli::command();
            clap_complete::generate(shell, &mut command, "cockup", &mut std::io::stdout());
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        report::error(&error);
        std::process::exit(1);
    }
}
