use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "winget-bundle",
    version,
    about = "Install packages from the \x1b[1mBundlefile\x1b[0m.
    
This command finds the \x1b[1mBundlefile\x1b[0m using environment variables only:

- If $env:XDG_CONFIG_HOME is set, use $env:XDG_CONFIG_HOME/winget-bundle/Bundlefile.
- Else, use $env:USERPROFILE/.Bundlefile."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand, Default)]
pub enum Commands {
    #[command(
        about = "\x1b[1m[Default]\x1b[0m Install all dependencies from the \x1b[1mBundlefile\x1b[0m"
    )]
    #[default]
    Install,
    #[command(about = "Uninstall all dependencies not present in the \x1b[1mBundlefile\x1b[0m")]
    Cleanup {
        /// Actually performs its cleanup operations
        #[arg(short, long)]
        force: bool,
    },
}
