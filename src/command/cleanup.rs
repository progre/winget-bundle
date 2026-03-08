use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use anyhow::Result;
use futures::try_join;
use term_grid::{Direction, Filling, Grid, GridOptions};
use terminal_size::{Width, terminal_size};

use super::load_files;
use crate::command::save_statefile;
use crate::file::bundlefile::{self, Bundlefile};
use crate::file::statefile::{self, Statefile};
use crate::package_manager::{scoop, winget};

pub async fn cleanup(force: bool) -> Result<()> {
    let ((bundlefile, statefile, statefile_path), winget_package_list, scoop_installed_packages) =
        try_join!(load_files(), winget::list(), scoop::installed_packages())?;

    let statefile_targets = statefile_uninstall_target(&statefile, &bundlefile);
    let msstore_targets = msstore_uninstall_target(&winget_package_list, &bundlefile);
    let scoop_targets = scoop_uninstall_target(&scoop_installed_packages, &bundlefile);
    if statefile_targets.is_empty() && msstore_targets.is_empty() && scoop_targets.is_empty() {
        return Ok(());
    }
    if !force {
        println!("Would uninstall packages:");
        let statefile_targets = statefile_targets.into_iter().map(|x| x.id.as_str());
        let msstore_items = msstore_targets.into_iter().map(|x| x.name.as_str());
        let scoop_targets = scoop_targets.into_iter().flat_map(|x| x.into_iter());
        print_grid(statefile_targets.chain(msstore_items).chain(scoop_targets));
        println!("Run `winget-bundle cleanup --force` to make these changes.");
        return Ok(());
    }

    let mut uninstalled = cleanup_statefile(
        &statefile_targets,
        &winget_package_list,
        &statefile,
        &statefile_path,
    )
    .await?;
    uninstalled += cleanup_msstore(&msstore_targets).await?;
    uninstalled += cleanup_scoop(&scoop_targets).await?;

    if uninstalled > 0 {
        println!("Uninstalled {uninstalled} packages");
    }
    Ok(())
}

async fn cleanup_statefile(
    uninstall: &[&statefile::PackageEntry],
    winget_package_list: &[winget::PackageEntry],
    statefile: &Statefile,
    statefile_path: &Path,
) -> Result<u32> {
    let installed_packages: HashSet<(statefile::Source, &str)> = winget_package_list
        .iter()
        .filter_map(|x| {
            x.source
                .and_then(|y: winget::Source| y.try_into().ok())
                .map(|y| (y, x.id.as_str()))
        })
        .collect();

    let mut packages: BTreeMap<_, statefile::PackageEntry> = statefile
        .packages
        .iter()
        .map(|x| ((x.source, x.id.as_str()), x.clone()))
        .collect();

    let mut uninstalled = 0;
    for &entry in uninstall {
        let key = (entry.source, entry.id.as_str());
        if installed_packages.contains(&key) {
            println!("Uninstalling {}...", entry.id);
            if let Err(err) = winget::uninstall(key.0.into(), key.1).await {
                eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m");
                continue;
            }
            uninstalled += 1;
        }
        let _ = packages.remove(&key);
        let statefile = Statefile::new(packages.values().cloned().collect());
        save_statefile(&statefile, statefile_path).await?;
    }
    Ok(uninstalled)
}

async fn cleanup_msstore(uninstall: &[&winget::PackageEntry]) -> Result<u32> {
    let mut uninstalled = 0;
    for entry in uninstall {
        println!("Uninstalling {}...", entry.name);
        if let Err(err) = winget::uninstall(winget::Source::MsStore, &entry.id).await {
            eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m");
            continue;
        }
        uninstalled += 1;
    }
    Ok(uninstalled)
}

async fn cleanup_scoop(uninstall: &[Vec<&str>]) -> Result<u32> {
    let mut uninstalled = 0;
    for group in uninstall {
        for name in group {
            println!("Uninstalling {name}...");
            if let Err(err) = scoop::uninstall(name).await {
                eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m");
                continue;
            }
            uninstalled += 1;
        }
    }
    Ok(uninstalled)
}

fn statefile_uninstall_target<'a>(
    statefile: &'a Statefile,
    bundlefile: &Bundlefile,
) -> Vec<&'a statefile::PackageEntry> {
    statefile
        .packages
        .iter()
        .filter(|x| !exists_in_bundlefile(bundlefile, x.source.into(), &x.id))
        .collect()
}

fn msstore_uninstall_target<'a>(
    winget_package_list: &'a [winget::PackageEntry],
    bundlefile: &Bundlefile,
) -> Vec<&'a winget::PackageEntry> {
    winget_package_list
        .iter()
        .filter(|x| x.source == Some(winget::Source::MsStore))
        .filter(|x| !exists_in_bundlefile(bundlefile, bundlefile::Source::MsStore, &x.id))
        .collect()
}

fn scoop_uninstall_target<'a>(
    scoop_installed_packages: &'a [scoop::PackageEntry],
    bundlefile: &Bundlefile,
) -> Vec<Vec<&'a str>> {
    let mut scoop_targets = vec![];
    let mut packages: Vec<_> = scoop_installed_packages.iter().collect();
    loop {
        let depends: HashSet<_> = packages.iter().flat_map(|x| &x.dependencies).collect();
        let (required, removing): (Vec<_>, _) =
            std::mem::take(&mut packages).into_iter().partition(|x| {
                scoop::INSTALLATION_HELPERS.contains(&x.name.as_str())
                    || depends.contains(&x.name)
                    || exists_in_bundlefile(bundlefile, bundlefile::Source::Scoop, &x.name)
            });
        if removing.is_empty() {
            break;
        }
        scoop_targets.push(removing.into_iter().map(|x| x.name.as_str()).collect());
        packages = required;
    }
    scoop_targets
}

fn print_grid<'a>(items: impl Iterator<Item = &'a str>) {
    let cells = items.collect();
    let grid = Grid::new(
        cells,
        GridOptions {
            filling: Filling::Spaces(2),
            direction: Direction::LeftToRight,
            width: terminal_size().map(|(Width(w), _)| w).unwrap_or(80) as usize,
        },
    );
    print!("{grid}")
}

fn exists_in_bundlefile(bundlefile: &Bundlefile, source: bundlefile::Source, key: &str) -> bool {
    bundlefile
        .entries
        .iter()
        .any(|x| x.source == source && x.id == key)
}
