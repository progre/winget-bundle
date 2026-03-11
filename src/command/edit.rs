use std::env;
use std::process::Command;

use anyhow::{Context, Result};

use super::locate_bundlefile;

pub fn edit() -> Result<()> {
    let bundlefile_path = locate_bundlefile()?;

    if let Some(editor) = env::var_os("EDITOR") {
        Command::new(&editor)
            .arg(&bundlefile_path)
            .status()
            .with_context(|| {
                format!(
                    "failed to launch editor {:?} for {}",
                    editor,
                    bundlefile_path.display()
                )
            })?;
    } else {
        Command::new("powershell.exe")
            .args([
                "-NonInteractive",
                "-NoProfile",
                "-Command",
                &format!("Start-Process '{}'", bundlefile_path.to_string_lossy()),
            ])
            .status()
            .with_context(|| {
                format!(
                    "failed to open {} with default application",
                    bundlefile_path.display()
                )
            })?;
    }

    Ok(())
}
