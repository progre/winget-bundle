use anyhow::Result;

use crate::command::{load_files, save_lockfile};
use crate::file::bundlefile::{self, Source};
use crate::file::lockfile::{self, Lockfile};
use crate::winget;

pub async fn install() -> Result<()> {
    let (bundlefile, mut lockfile, lockfile_path) = load_files().await?;

    let mut succeeded = 0;
    for entry in bundlefile.entries {
        if exists(&lockfile, &entry) {
            println!("Using {}", entry.id);
            succeeded += 1;
            continue;
        }
        println!("\x1b[32mInstalling {}\x1b[0m", entry.id);
        match install_package(entry).await {
            Ok(entry) => lockfile.packages.push(entry),
            Err(err) => eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m"),
        }
        succeeded += 1;
    }
    lockfile
        .packages
        .sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.id.cmp(&b.id)));

    save_lockfile(&lockfile, &lockfile_path).await?;
    println!(
        "\x1b[32m`winget-bundle` complete! {} Bundlefile dependencies now installed.\x1b[0m",
        succeeded
    );
    Ok(())
}

fn exists(lockfile: &Lockfile, entry: &bundlefile::PackageEntry) -> bool {
    lockfile
        .packages
        .iter()
        .any(|x| x.source == entry.source && x.id == entry.id)
}

async fn install_package(entry: bundlefile::PackageEntry) -> Result<lockfile::PackageEntry> {
    match entry.source {
        Source::Winget => {
            winget::install(&entry.id).await?;
            Ok(lockfile::PackageEntry {
                source: Source::Winget,
                id: entry.id,
                name: entry.name,
            })
        }
        Source::MsStore => unimplemented!(),
        Source::Scoop => unimplemented!(),
    }
}
