use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use anyhow::Result;
use term_grid::{Direction, Filling, Grid, GridOptions};
use terminal_size::{Width, terminal_size};

use super::load_files;
use crate::command::save_lockfile;
use crate::file::bundlefile::{self, Bundlefile};
use crate::file::lockfile::{self, Lockfile};
use crate::package_manager::{scoop, winget};

pub async fn cleanup(force: bool) -> Result<()> {
    let (bundlefile, lockfile, lockfile_path) = load_files().await?;
    let scoop_installed_packages = scoop::installed_packages().await?;

    let lockfile_targets = lockfile_uninstall_target(&lockfile, &bundlefile);
    let scoop_targets = scoop_uninstall_target(&scoop_installed_packages, &bundlefile);
    if lockfile_targets.is_empty() && scoop_targets.is_empty() {
        return Ok(());
    }
    if !force {
        println!("Would uninstall packages:");
        let lockfile_targets = lockfile_targets.into_iter().map(|(_, key)| key);
        let scoop_targets = scoop_targets.into_iter().flat_map(|x| x.into_iter());
        print_grid(lockfile_targets.chain(scoop_targets));
        println!("Run `winget-bundle cleanup --force` to make these changes.");
        return Ok(());
    }

    let mut uninstalled = cleanup_lockfile(&lockfile_targets, &lockfile, &lockfile_path).await?;
    uninstalled += cleanup_scoop(&scoop_targets).await?;

    if uninstalled > 0 {
        println!("Uninstalled {uninstalled} packages");
    }
    Ok(())
}

async fn cleanup_lockfile(
    uninstall: &[(lockfile::Source, &str)],
    lockfile: &Lockfile,
    lockfile_path: &Path,
) -> Result<u32> {
    let mut packages: BTreeMap<_, lockfile::PackageEntry> = lockfile
        .packages
        .iter()
        .map(|x| ((x.source, x.id.as_str()), x.clone()))
        .collect();

    let installed_packages = winget::list().await?;
    let installed_packages = installed_packages
        .iter()
        .filter_map(|x| x.source.map(|source| (source, x.id.as_str())))
        .collect::<HashSet<_>>();

    let mut uninstalled = 0;
    for &(source, key) in uninstall {
        if installed_packages.contains(&(source.into(), key)) {
            println!("Uninstalling {key}...");
            if let Err(err) = winget::uninstall(source.into(), key).await {
                eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m");
                continue;
            }
            uninstalled += 1;
        }
        let _ = packages.remove(&(source, key));
        let lockfile = lockfile::Lockfile::new(packages.clone().into_values().collect());
        save_lockfile(&lockfile, lockfile_path).await?;
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

fn lockfile_uninstall_target<'a>(
    lockfile: &'a Lockfile,
    bundlefile: &Bundlefile,
) -> Vec<(lockfile::Source, &'a str)> {
    lockfile
        .packages
        .iter()
        .filter(|x| !exists_in_bundlefile(bundlefile, x.source.into(), &x.id))
        .map(|x| (x.source, x.id.as_str()))
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
