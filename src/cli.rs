use std::env;

use clap::{Parser, Subcommand};
use const_format::formatcp;

const APP_ABOUT: &str =
"Install packages from the \x1b[1mBundlefile\x1b[0m.

This command finds the \x1b[1mBundlefile\x1b[0m using environment variables only:

- If \x1b[1m$env:XDG_CONFIG_HOME\x1b[0m is set, use \x1b[1m$env:XDG_CONFIG_HOME/winget-bundle/Bundlefile\x1b[0m
- Else, use \x1b[1m$env:USERPROFILE/.Bundlefile\x1b[0m";

const INSTALL_ABOUT: &str =
    "\x1b[1m[Default]\x1b[0m Install all dependencies from the \x1b[1mBundlefile\x1b[0m";
const INSTALL_NO_UPGRADE_HELP: &str =
    "Don't run upgrade on outdated dependencies. Enabled by default if
\x1b[1m$env:WINGET_BUNDLE_NO_UPGRADE\x1b[0m is set.";
const INSTALL_UPGRADE_HELP: &str = "Run upgrade on outdated dependencies, even if
\x1b[1m$env:WINGET_BUNDLE_NO_UPGRADE\x1b[0m is set.";

const CLEANUP_ABOUT: &str =
    "Uninstall all dependencies not present in the \x1b[1mBundlefile\x1b[0m";
const CLEANUP_FORCE_HELP: &str = "Actually performs its cleanup operations";

const EDIT_ABOUT: &str = "Edit the \x1b[1mBundlefile\x1b[0m in your editor.";
const EDIT_LONG_ABOUT: &str = formatcp!(
    "{EDIT_ABOUT}

    Uses \x1b[1m$env:EDITOR\x1b[0m if set, otherwise opens with the system default application.",
);
const CHECK_ABOUT: &str =
    "Check if all dependencies present in the \x1b[1mBundlefile\x1b[0m are installed.";

#[derive(Debug, Parser)]
#[command(name = "winget-bundle", version, about = APP_ABOUT)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = INSTALL_ABOUT)]
    Install {
        #[arg(long, conflicts_with = "upgrade", help = INSTALL_NO_UPGRADE_HELP)]
        no_upgrade: bool,
        #[arg(long, conflicts_with = "no_upgrade", help = INSTALL_UPGRADE_HELP)]
        upgrade: bool,
    },
    #[command(about = CLEANUP_ABOUT)]
    Cleanup {
        #[arg(short, long, help = CLEANUP_FORCE_HELP)]
        force: bool,
    },
    #[command(about = EDIT_ABOUT, long_about = EDIT_LONG_ABOUT)]
    Edit,
    #[command(about = CHECK_ABOUT)]
    Check {
        #[arg(long, conflicts_with = "upgrade", help = INSTALL_NO_UPGRADE_HELP)]
        no_upgrade: bool,
        #[arg(long, conflicts_with = "no_upgrade", help = INSTALL_UPGRADE_HELP)]
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
