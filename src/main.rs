mod cli;
mod command;
mod file;
mod winget;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::command::{cleanup, install};

fn main() -> Result<()> {
    smol::block_on(async {
        let cli = Cli::parse();
        match cli.command.unwrap_or_default() {
            Commands::Install => {
                if let Err(err) = install().await {
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
