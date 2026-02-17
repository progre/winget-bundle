mod cleanup;
mod install;

use std::io::ErrorKind;
use std::path::PathBuf;
use std::{env, io};

use anyhow::{Result, bail};
use smol::fs;

use crate::file::bundlefile::{self, Bundlefile};
use crate::file::lockfile::Lockfile;
use crate::winget;

pub use cleanup::cleanup;
pub use install::install;

async fn load_files() -> Result<(Bundlefile, Lockfile, PathBuf)> {
    let bundlefile_path = locate_bundlefile()?;
    let bundlefile = fs::read_to_string(&bundlefile_path)
        .await
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", bundlefile_path.display(), e))?;
    let bundlefile: Bundlefile = bundlefile.parse()?;

    let lockfile_path = bundlefile_path.with_extension("lock");
    let lockfile = match fs::read_to_string(&lockfile_path).await {
        Ok(lockfile) => lockfile.parse()?,
        Err(err) if err.kind() == ErrorKind::NotFound => Lockfile::default(),
        Err(err) => bail!("failed to read {}: {}", lockfile_path.display(), err),
    };
    Ok((bundlefile, lockfile, lockfile_path))
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

async fn save_lockfile(lockfile: &Lockfile, lockfile_path: &PathBuf) -> io::Result<()> {
    fs::write(&lockfile_path, lockfile.to_string()).await
}

async fn exists_in_package_manager(source: bundlefile::Source, id: &str) -> Result<bool> {
    match source {
        bundlefile::Source::Winget => winget::exists(winget::Source::Winget, id).await,
        bundlefile::Source::MsStore => winget::exists(winget::Source::MsStore, id).await,
        bundlefile::Source::Scoop => unimplemented!(),
    }
}
