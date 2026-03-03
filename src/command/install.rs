use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

use anyhow::Result;
use futures::try_join;

use crate::command::{load_files, save_lockfile};
use crate::file::bundlefile;
use crate::file::lockfile;
use crate::package_manager::{scoop, winget};

pub async fn install(upgrade: bool) -> Result<()> {
    let ((bundlefile, lockfile, lockfile_path), winget_package_list, scoop_package_list) =
        try_join!(load_files(), winget::list(), scoop::installed_packages())?;

    let mut lockfile_packages: BTreeMap<(lockfile::Source, String), lockfile::PackageEntry> =
        lockfile
            .packages
            .iter()
            .map(|x| ((x.source, x.id.clone()), x.clone()))
            .collect();

    let (mut installed_packages, mut upgradable_packages) = list_packages(
        &winget_package_list,
        &scoop_package_list,
        &bundlefile.entries,
    );
    if !upgrade {
        installed_packages.extend(upgradable_packages.drain());
    }

    let mut installed = 0;
    for entry in bundlefile.entries {
        if let Err(err) = handle_entry(&entry, &installed_packages, &upgradable_packages).await {
            eprintln!("\x1b[31m{err}\x1b[0m");
            continue;
        }
        if let Ok(source) = entry.source.try_into()
            && let Entry::Vacant(e) = lockfile_packages.entry((source, entry.id.clone()))
        {
            e.insert(lockfile::PackageEntry::new(source, entry.id, entry.name));
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
    installed_packages: &HashSet<bundlefile::CompositeKey<'_>>,
    upgradable_packages: &HashSet<bundlefile::CompositeKey<'_>>,
) -> Result<()> {
    if installed_packages.contains(&entry.as_key()) {
        println!("Using {entry}");
        Ok(())
    } else if upgradable_packages.contains(&entry.as_key()) {
        println!("\x1b[32mUpgrading {entry}\x1b[0m");
        upgrade_package(entry).await
    } else {
        println!("\x1b[32mInstalling {entry}\x1b[0m");
        install_package(entry).await
    }
}

fn list_packages<'a>(
    winget_package_list: &'a [winget::PackageEntry],
    scoop_package_list: &'a [scoop::PackageEntry],
    bundlefile: &[bundlefile::PackageEntry],
) -> (
    HashSet<bundlefile::CompositeKey<'a>>,
    HashSet<bundlefile::CompositeKey<'a>>,
) {
    let (upgradable_winget_packages, installed_winget_packages) =
        list_winget_packages(winget_package_list, bundlefile);

    let (upgradable_scoop_packages, installed_scoop_packages) =
        list_scoop_packages(scoop_package_list, bundlefile);

    let upgradable_packages = upgradable_winget_packages
        .chain(upgradable_scoop_packages)
        .collect::<HashSet<_>>();
    let installed_packages = installed_winget_packages
        .chain(installed_scoop_packages)
        .collect::<HashSet<_>>();

    (installed_packages, upgradable_packages)
}

fn list_winget_packages<'a>(
    winget_package_list: &'a [winget::PackageEntry],
    bundlefile: &[bundlefile::PackageEntry],
) -> (
    impl Iterator<Item = bundlefile::CompositeKey<'a>>,
    impl Iterator<Item = bundlefile::CompositeKey<'a>>,
) {
    let (upgradable_pkgs, installed_pkgs): (Vec<_>, _) = winget_package_list
        .iter()
        .filter(|x| x.source.is_some())
        .partition(|x| {
            x.is_upgradable() && is_upgrade_granted(bundlefile, x.as_bundlefile_key().unwrap())
        });
    let upgradable_pkgs = upgradable_pkgs
        .into_iter()
        .map(|x| x.as_bundlefile_key().unwrap());
    let installed_pkgs = installed_pkgs
        .into_iter()
        .map(|x| x.as_bundlefile_key().unwrap());
    (upgradable_pkgs, installed_pkgs)
}

fn list_scoop_packages<'a>(
    scoop_package_list: &'a [scoop::PackageEntry],
    bundlefile: &[bundlefile::PackageEntry],
) -> (
    impl Iterator<Item = bundlefile::CompositeKey<'a>>,
    impl Iterator<Item = bundlefile::CompositeKey<'a>>,
) {
    let (upgradable_pkgs, installed_pkgs): (Vec<_>, _) = scoop_package_list
        .iter()
        .partition(|x| x.is_upgradable() && is_upgrade_granted(bundlefile, x.as_bundlefile_key()));
    let upgradable_pkgs = upgradable_pkgs.into_iter().map(|x| x.as_bundlefile_key());
    let installed_pkgs = installed_pkgs.into_iter().map(|x| x.as_bundlefile_key());
    (upgradable_pkgs, installed_pkgs)
}

fn is_upgrade_granted(
    bundlefile: &[bundlefile::PackageEntry],
    key: bundlefile::CompositeKey,
) -> bool {
    bundlefile
        .iter()
        .find(|y| y.as_key() == key)
        .map(|y| !y.no_upgrade)
        .unwrap_or(true)
}

async fn install_package(entry: &bundlefile::PackageEntry) -> Result<()> {
    match entry.source {
        bundlefile::Source::Winget => winget::install(winget::Source::Winget, &entry.id).await,
        bundlefile::Source::MsStore => winget::install(winget::Source::MsStore, &entry.id).await,
        bundlefile::Source::Scoop => scoop::install(&entry.id).await,
    }
}

async fn upgrade_package(entry: &bundlefile::PackageEntry) -> Result<()> {
    match entry.source {
        bundlefile::Source::Winget => winget::upgrade(winget::Source::Winget, &entry.id).await,
        bundlefile::Source::MsStore => winget::upgrade(winget::Source::MsStore, &entry.id).await,
        bundlefile::Source::Scoop => scoop::upgrade(&entry.id).await,
    }
}
