use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

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

    let mut installed = 0;
    for entry in bundlefile.entries {
        if exists_in_package_manager(entry.source, &entry.id).await? {
            println!("Using {entry}");
        } else {
            println!("\x1b[32mInstalling {entry}\x1b[0m");
            if let Err(err) = install_package(&entry).await {
                eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m");
                continue;
            }
        }
        if let Entry::Vacant(e) = packages.entry((entry.source, entry.id.clone())) {
            e.insert(to_lockfile_entry(entry));
            let lockfile = lockfile::Lockfile::new(packages.clone().into_values().collect());
            save_lockfile(&lockfile, &lockfile_path).await?;
        }
        installed += 1;
    }

    println!(
        "\x1b[32m`winget-bundle` complete! {installed} Bundlefile dependencies now installed.\x1b[0m"
    );
    Ok(())
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
