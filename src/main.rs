mod cli;
mod command;
mod file;
mod winget;
mod winget_list_parser;

use std::env;

use anyhow::Result;

use crate::cli::Commands;
use crate::command::{cleanup, install};

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
        match cli.command {
            Commands::Install {
                no_upgrade,
                upgrade,
            } => {
                if let Err(err) = install(resolve_upgrade(no_upgrade, upgrade)).await {
                    eprintln!("\x1b[31m{err}\x1b[0m");
                    std::process::exit(1);
                }
            }
            Commands::Cleanup { force } => {
                if let Err(err) = cleanup(force).await {
                    eprintln!("\x1b[31m{err}\x1b[0m");
                    std::process::exit(1);
                }
            }
        }
        Ok(())
    })
}
