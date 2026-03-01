use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

use anyhow::Result;

use crate::command::{load_files, save_lockfile};
use crate::file::bundlefile;
use crate::file::lockfile;
use crate::package_manager::{scoop, winget};

pub async fn install(upgrade: bool) -> Result<()> {
    let (bundlefile, lockfile, lockfile_path) = load_files().await?;
    let mut lockfile_packages: BTreeMap<(lockfile::Source, String), lockfile::PackageEntry> =
        lockfile
            .packages
            .iter()
            .map(|x| ((x.source, x.id.clone()), x.clone()))
            .collect();

    let winget_package_list = winget::list().await?;
    let scoop_package_list = scoop::installed_packages().await?;
    let (installed_packages, upgradable_packages) = list_packages(
        &winget_package_list,
        &scoop_package_list,
        &bundlefile.entries,
        upgrade,
    );

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
    if installed_packages.contains(&entry.to_key()) {
        println!("Using {entry}");
        Ok(())
    } else if upgradable_packages.contains(&entry.to_key()) {
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
    upgrade: bool,
) -> (
    HashSet<bundlefile::CompositeKey<'a>>,
    HashSet<bundlefile::CompositeKey<'a>>,
) {
    let winget_package_list = winget_package_list.iter().filter(|x| x.source.is_some());
    let has_no_upgrade = |bundlefile: &[bundlefile::PackageEntry],
                          key: bundlefile::CompositeKey| {
        bundlefile
            .iter()
            .find(|y| y.to_key() == key)
            .map(|y| !y.no_upgrade)
            .unwrap_or(true)
    };

    let require_upgrade = |x: &winget::PackageEntry| {
        upgrade
            && x.is_upgradable()
            && has_no_upgrade(
                bundlefile,
                bundlefile::CompositeKey::new(x.source.unwrap().into(), &x.id),
            )
    };
    let upgradable_packages = winget_package_list
        .clone()
        .filter(|x| require_upgrade(x))
        .map(|x| x.to_bundlefile_key().unwrap());
    let installed_packages = winget_package_list
        .filter(|x| !require_upgrade(x))
        .map(|x| x.to_bundlefile_key().unwrap());

    let (upgradable_scoop_packages, installed_scoop_packages): (Vec<_>, _) =
        scoop_package_list.iter().partition(|x| {
            upgrade && x.is_upgradable() && has_no_upgrade(bundlefile, x.as_bundlefile_key())
        });

    let upgradable_packages = upgradable_scoop_packages
        .into_iter()
        .map(|x| x.as_bundlefile_key())
        .chain(upgradable_packages)
        .collect::<HashSet<_>>();
    let installed_packages = installed_scoop_packages
        .into_iter()
        .map(|x| x.as_bundlefile_key())
        .chain(installed_packages)
        .collect::<HashSet<_>>();

    (installed_packages, upgradable_packages)
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
