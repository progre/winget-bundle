use std::collections::HashSet;

use anyhow::Result;
use futures::try_join;

use crate::command::load_bundle_file;
use crate::file::bundlefile::{self, Bundlefile};
use crate::package_manager::{scoop, winget};

pub async fn check(upgrade: bool) -> Result<bool> {
    let (bundlefile, winget_package_list, scoop_package_list) = try_join!(
        load_bundle_file(),
        winget::list(),
        scoop::installed_packages()
    )?;
    let bundlefile: Bundlefile = bundlefile;

    let (mut installed_packages, mut upgradable_packages) = list_packages(
        &winget_package_list,
        &scoop_package_list,
        &bundlefile.entries,
    );
    if !upgrade {
        installed_packages.extend(upgradable_packages.drain());
    }

    let missing = bundlefile
        .entries
        .iter()
        .any(|entry| !installed_packages.contains(&entry.as_key()));
    if missing {
        println!("winget-bundle can't satisfy your Bundlefile's dependencies.");
        println!("Satisfy missing dependencies with `winget-bundle install`.");
        Ok(false)
    } else {
        println!("The Bundlefile's dependencies are satisfied.");
        Ok(true)
    }
}

pub fn list_packages<'a>(
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
