use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

use anyhow::Result;
use futures::try_join;

use crate::command::check::list_packages;
use crate::command::{load_files, save_statefile};
use crate::file::bundlefile;
use crate::file::statefile::{self, Statefile};
use crate::package_manager::{scoop, winget};

pub async fn install(upgrade: bool) -> Result<()> {
    let ((bundlefile, statefile, statefile_path), winget_package_list, scoop_package_list) =
        try_join!(load_files(), winget::list(), scoop::installed_packages())?;

    let mut statefile_packages: BTreeMap<(statefile::Source, String), statefile::PackageEntry> =
        statefile
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
            && let Entry::Vacant(e) = statefile_packages.entry((source, entry.id.clone()))
        {
            e.insert(statefile::PackageEntry::new(source, entry.id, entry.name));
            let statefile = Statefile::new(statefile_packages.clone().into_values().collect());
            save_statefile(&statefile, &statefile_path).await?;
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
