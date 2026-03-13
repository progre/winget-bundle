use std::env;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "winget-bundle",
    version,
    about = "Install packages from the \x1b[1mBundlefile\x1b[0m.

This command finds the \x1b[1mBundlefile\x1b[0m using environment variables only:

- If \x1b[1m$env:XDG_CONFIG_HOME\x1b[0m is set, use \x1b[1m$env:XDG_CONFIG_HOME/winget-bundle/Bundlefile\x1b[0m
- Else, use \x1b[1m$env:USERPROFILE/.Bundlefile\x1b[0m"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(
        about = "\x1b[1m[Default]\x1b[0m Install all dependencies from the \x1b[1mBundlefile\x1b[0m"
    )]
    Install {
        #[arg(
            long,
            conflicts_with = "upgrade",
            help = "Don't run upgrade on outdated dependencies. Enabled by default if
\x1b[1m$env:WINGET_BUNDLE_NO_UPGRADE\x1b[0m is set."
        )]
        no_upgrade: bool,
        #[arg(
            long,
            conflicts_with = "no_upgrade",
            help = "Run upgrade on outdated dependencies, even if \x1b[1m$env:WINGET_BUNDLE_NO_UPGRADE\x1b[0m is
set."
        )]
        upgrade: bool,
    },
    #[command(about = "Uninstall all dependencies not present in the \x1b[1mBundlefile\x1b[0m")]
    Cleanup {
        /// Actually performs its cleanup operations
        #[arg(short, long)]
        force: bool,
    },
    #[command(
        about = "Edit the \x1b[1mBundlefile\x1b[0m in your editor.",
        long_about = "Edit the \x1b[1mBundlefile\x1b[0m in your editor.

Uses \x1b[1m$env:EDITOR\x1b[0m if set, otherwise opens with the system default application."
    )]
    Edit,
    #[command(
        about = "Check if all dependencies present in the \x1b[1mBundlefile\x1b[0m are installed."
    )]
    Check {
        #[arg(
            long,
            conflicts_with = "upgrade",
            help = "Don't run upgrade on outdated dependencies. Enabled by default if
\x1b[1m$env:WINGET_BUNDLE_NO_UPGRADE\x1b[0m is set."
        )]
        no_upgrade: bool,
        #[arg(
            long,
            conflicts_with = "no_upgrade",
            help = "Run upgrade on outdated dependencies, even if \x1b[1m$env:WINGET_BUNDLE_NO_UPGRADE\x1b[0m is
set."
        )]
        upgrade: bool,
    },
}

pub fn parse_cli() -> Cli {
    let mut args: Vec<String> = env::args().collect();

    if needs_default(&args) {
        args.insert(1, "install".to_string());
    }

    Cli::parse_from(args)
}

fn needs_default(args: &[String]) -> bool {
    const ROOT_ARGS: [&str; 4] = ["-h", "--help", "-V", "--version"];
    args.get(1)
        .map(|x| !ROOT_ARGS.contains(&x.as_str()) && x.starts_with("-"))
        .unwrap_or(true)
}
