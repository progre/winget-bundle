use std::collections::BTreeMap;

use anyhow::Result;

use crate::command::{exists_in_package_manager, load_files, save_lockfile};
use crate::file::bundlefile::{self, Source};
use crate::file::lockfile::{self, PackageEntry};
use crate::winget;

pub async fn install() -> Result<()> {
    let (bundlefile, lockfile, lockfile_path) = load_files().await?;

    let mut packages: BTreeMap<(Source, String), PackageEntry> = lockfile
        .packages
        .iter()
        .map(|x| ((x.source, x.id.clone()), x.clone()))
        .collect();
    let installed = install_all(bundlefile.entries, &mut packages).await?;

    let packages = packages.into_values().collect();
    if packages != lockfile.packages {
        let lockfile = lockfile::Lockfile::new(packages);
        save_lockfile(&lockfile, &lockfile_path).await?;
    }
    println!(
        "\x1b[32m`winget-bundle` complete! {installed} Bundlefile dependencies now installed.\x1b[0m"
    );
    Ok(())
}

async fn install_all(
    bundlefile_entries: Vec<bundlefile::PackageEntry>,
    packages: &mut BTreeMap<(Source, String), PackageEntry>,
) -> Result<u32> {
    let mut installed = 0;
    for entry in bundlefile_entries {
        if exists_in_package_manager(entry.source, &entry.id).await? {
            println!("Using {entry}");
        } else {
            println!("\x1b[32mInstalling {entry}\x1b[0m");
            if let Err(err) = install_package(&entry).await {
                eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m");
                continue;
            }
        }
        let _ = packages.insert((entry.source, entry.id.clone()), to_lockfile_entry(entry));
        installed += 1;
    }
    Ok(installed)
}

fn to_lockfile_entry(entry: bundlefile::PackageEntry) -> lockfile::PackageEntry {
    lockfile::PackageEntry {
        source: entry.source,
        id: entry.id,
        name: entry.name,
    }
}

async fn install_package(entry: &bundlefile::PackageEntry) -> Result<()> {
    match entry.source {
        Source::Winget => winget::install(winget::Source::Winget, &entry.id).await,
        Source::MsStore => winget::install(winget::Source::MsStore, &entry.id).await,
        Source::Scoop => unimplemented!(),
    }
}
