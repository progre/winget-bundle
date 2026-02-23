use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

use anyhow::Result;

use crate::command::{load_files, save_lockfile};
use crate::file::bundlefile::{self, Source};
use crate::file::lockfile::{self, PackageEntry};
use crate::winget;

pub async fn install(upgrade: bool) -> Result<()> {
    let (bundlefile, lockfile, lockfile_path) = load_files().await?;
    let mut lockfile_packages: BTreeMap<(Source, String), PackageEntry> = lockfile
        .packages
        .iter()
        .map(|x| ((x.source, x.id.clone()), x.clone()))
        .collect();

    let package_list = winget::list().await?;
    let (installed_packages, upgradable_packages) =
        list_packages(&package_list, &bundlefile.entries, upgrade);

    let mut installed = 0;
    for entry in bundlefile.entries {
        if let Err(err) = handle_entry(&entry, &installed_packages, &upgradable_packages).await {
            eprintln!("\x1b[31m{err}\x1b[0m");
            continue;
        }
        if let Entry::Vacant(e) = lockfile_packages.entry((entry.source, entry.id.clone())) {
            e.insert(to_lockfile_entry(entry));
            let lockfile =
                lockfile::Lockfile::new(lockfile_packages.clone().into_values().collect());
            save_lockfile(&lockfile, &lockfile_path).await?;
        }
        installed += 1;
    }

    println!(
        "\x1b[32m`winget-bundle` complete! {installed} Bundlefile dependencies now installed.\x1b[0m"
    );
    Ok(())
}

async fn handle_entry(
    entry: &bundlefile::PackageEntry,
    installed_packages: &HashSet<(Source, &str)>,
    upgradable_packages: &HashSet<(Source, &str)>,
) -> Result<()> {
    if installed_packages.contains(&(entry.source, &entry.id)) {
        println!("Using {entry}");
        Ok(())
    } else if upgradable_packages.contains(&(entry.source, &entry.id)) {
        println!("\x1b[32mUpgrading {entry}\x1b[0m");
        upgrade_package(entry).await
    } else {
        println!("\x1b[32mInstalling {entry}\x1b[0m");
        install_package(entry).await
    }
}

fn to_lockfile_entry(entry: bundlefile::PackageEntry) -> lockfile::PackageEntry {
    lockfile::PackageEntry {
        source: entry.source,
        id: entry.id,
        name: entry.name,
    }
}

type InstalledPackagesAndUpgradablePackages<'a> = (
    HashSet<(bundlefile::Source, &'a str)>,
    HashSet<(bundlefile::Source, &'a str)>,
);

fn list_packages<'a>(
    package_list: &'a [winget::PackageEntry],
    bundlefile: &[bundlefile::PackageEntry],
    upgrade: bool,
) -> InstalledPackagesAndUpgradablePackages<'a> {
    let require_upgrade = |x: &winget::PackageEntry| {
        upgrade
            && x.update_available
            && bundlefile
                .iter()
                .find(|y| y.id == x.id)
                .map(|y| !y.no_upgrade)
                .unwrap_or(true)
    };
    let upgradable_packages = package_list
        .iter()
        .filter(|x| x.source.is_some() && require_upgrade(x))
        .map(|x| (x.source.unwrap().into(), x.id.as_str()))
        .collect::<HashSet<_>>();
    let installed_packages = package_list
        .iter()
        .filter(|x| x.source.is_some() && !require_upgrade(x))
        .map(|x| (x.source.unwrap().into(), x.id.as_str()))
        .collect::<HashSet<_>>();
    (installed_packages, upgradable_packages)
}

async fn install_package(entry: &bundlefile::PackageEntry) -> Result<()> {
    match entry.source {
        Source::Winget => winget::install(winget::Source::Winget, &entry.id).await,
        Source::MsStore => winget::install(winget::Source::MsStore, &entry.id).await,
        Source::Scoop => unimplemented!(),
    }
}

async fn upgrade_package(entry: &bundlefile::PackageEntry) -> Result<()> {
    match entry.source {
        Source::Winget => winget::upgrade(winget::Source::Winget, &entry.id).await,
        Source::MsStore => winget::upgrade(winget::Source::MsStore, &entry.id).await,
        Source::Scoop => unimplemented!(),
    }
}
