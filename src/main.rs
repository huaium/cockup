mod brew;
mod cli;
mod config;
mod files;
mod hooks;
mod ingredients;
mod manifest;
mod report;
use clap::{CommandFactory, Parser};
use cli::{CacheCommand, Commands, ConfigArgs, IngredientCommand};
use ingredients::Mode;
fn backup_initialized(path: &std::path::Path) -> Result<bool, String> {
    let destination = config::destination(path)?;
    match std::fs::symlink_metadata(&destination) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            report::warning(&format!(
                "Backup is not initialized: {}",
                destination.display()
            ));
            Ok(false)
        }
        Err(error) => Err(format!("{}: {error}", destination.display())),
        Ok(_) if destination.is_dir() => Ok(true),
        Ok(_) => Err(format!(
            "Backup destination is not a directory: {}",
            destination.display()
        )),
    }
}
fn snapshot(path: &std::path::Path) -> Result<Mode, String> {
    let destination = config::destination(path)?;
    let ingredients = manifest::Manifest::load(&destination)?
        .map(|manifest| manifest.ingredients)
        .unwrap_or_default();
    Ok(Mode::Snapshot(ingredients))
}
fn configured(
    args: &ConfigArgs,
    dry_run: bool,
    mode: Mode,
) -> Result<Option<config::Config>, String> {
    let cfg = config::load(&args.config_file, mode)?;
    if !dry_run && !args.approve_hooks && cfg.all_hooks().next().is_some() {
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
            if let Some(cfg) = configured(
                &args.config,
                args.dry_run,
                if args.dry_run {
                    Mode::ReadOnly
                } else {
                    Mode::Update
                },
            )? {
                files::execute(&cfg, false, args.dry_run, files::RestorePolicy::Ask)?;
            }
        }
        Commands::Restore(args) => {
            let mode = snapshot(&args.file.config.config_file)?;
            if let Some(cfg) = configured(&args.file.config, args.file.dry_run, mode)? {
                let policy = if args.r#override {
                    files::RestorePolicy::Override
                } else if args.skip_existing {
                    files::RestorePolicy::SkipExisting
                } else {
                    files::RestorePolicy::Ask
                };
                files::execute(&cfg, true, args.file.dry_run, policy)?;
            }
        }
        Commands::Verify { config_file } => {
            if backup_initialized(&config_file)? {
                let mode = snapshot(&config_file)?;
                let cfg = config::load(&config_file, mode)?;
                files::verify(&cfg)?;
            }
        }
        Commands::Diff {
            config_file,
            summary,
        } => {
            if backup_initialized(&config_file)? {
                let mode = snapshot(&config_file)?;
                let cfg = config::load(&config_file, mode)?;
                files::diff(&cfg, summary)?;
            }
        }
        Commands::Hook { config, name } => {
            if let Some(cfg) = configured(&config, false, Mode::ReadOnly)? {
                hooks::select(&cfg, name.as_deref())?;
            }
        }
        Commands::Detect { casks } => brew::detect(casks)?,
        Commands::Ingredient { command } => match command {
            IngredientCommand::Show { name } => ingredients::show(&name)?,
            IngredientCommand::Search { query } => ingredients::search(&query)?,
        },
        Commands::Cache { command } => match command {
            CacheCommand::Status => ingredients::cache_status()?,
            CacheCommand::List => ingredients::cache_list()?,
            CacheCommand::Show { name } => ingredients::status(&name)?,
            CacheCommand::Refresh {
                name,
                ingredients: all_ingredients,
                catalog,
            } => {
                ingredients::refresh(name.as_deref(), all_ingredients, catalog)?;
            }
            CacheCommand::Delete {
                names,
                catalog,
                ingredients: all_ingredients,
            } => {
                if catalog {
                    ingredients::delete_catalog()?;
                } else if all_ingredients {
                    ingredients::delete_all()?;
                } else {
                    ingredients::delete(&names)?;
                }
            }
            CacheCommand::Clean => ingredients::clean()?,
        },
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
