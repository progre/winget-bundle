mod cli;
mod command;
mod file;
mod package_manager;

use std::env;

use anyhow::Result;

use crate::cli::Commands;
use crate::command::{check, cleanup, edit, install};

fn resolve_upgrade(no_upgrade: bool, upgrade: bool) -> bool {
    debug_assert!(!no_upgrade || !upgrade);
    if no_upgrade {
        return false;
    }
    if upgrade {
        return true;
    }
    env::var_os("WINGET_BUNDLE_NO_UPGRADE").is_none()
}

fn main() -> Result<()> {
    smol::block_on(async {
        let cli = cli::parse_cli();
        if let Err(err) = match cli.command {
            Commands::Install {
                no_upgrade,
                upgrade,
            } => install(resolve_upgrade(no_upgrade, upgrade)).await,
            Commands::Cleanup { force } => cleanup(force).await,
            Commands::Check {
                no_upgrade,
                upgrade,
            } => match check(resolve_upgrade(no_upgrade, upgrade)).await {
                Ok(true) => Ok(()),
                Ok(false) => std::process::exit(1),
                Err(err) => Err(err),
            },
            Commands::Edit => edit(),
        } {
            eprintln!("\x1b[31m{err}\x1b[0m");
            std::process::exit(1);
        }
        Ok(())
    })
}
