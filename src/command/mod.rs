mod cleanup;
mod install;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::{env, io};

use anyhow::{Result, bail};
use smol::fs;

use crate::file::bundlefile::Bundlefile;
use crate::file::statefile::Statefile;

pub use cleanup::cleanup;
pub use install::install;

async fn load_files() -> Result<(Bundlefile, Statefile, PathBuf)> {
    let bundlefile_path = locate_bundlefile()?;
    let bundlefile = fs::read_to_string(&bundlefile_path)
        .await
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", bundlefile_path.display(), e))?;
    let bundlefile: Bundlefile = bundlefile.parse()?;

    let statefile_path = bundlefile_path.with_extension("state");
    let statefile = match fs::read_to_string(&statefile_path).await {
        Ok(statefile) => statefile.parse()?,
        Err(err) if err.kind() == ErrorKind::NotFound => Statefile::default(),
        Err(err) => bail!("failed to read {}: {}", statefile_path.display(), err),
    };
    Ok((bundlefile, statefile, statefile_path))
}

fn locate_bundlefile() -> Result<PathBuf> {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("winget-bundle").join("Bundlefile"));
    }

    if let Some(userprofile) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(userprofile).join(".Bundlefile"));
    }

    Err(anyhow::anyhow!(
        "$env:USERPROFILE is not set, cannot locate Bundlefile"
    ))
}

async fn save_statefile(statefile: &Statefile, statefile_path: &Path) -> io::Result<()> {
    fs::write(&statefile_path, statefile.to_string()).await
}
