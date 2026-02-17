use std::collections::BTreeMap;

use anyhow::Result;
use term_grid::{Direction, Filling, Grid, GridOptions};
use terminal_size::{Width, terminal_size};

use super::load_files;
use crate::command::{exists_in_package_manager, save_lockfile};
use crate::file::bundlefile::{Bundlefile, Source};
use crate::file::lockfile::{self, Lockfile, PackageEntry};
use crate::winget;

pub async fn cleanup(force: bool) -> Result<()> {
    let (bundlefile, lockfile, lockfile_path) = load_files().await?;
    let uninstall: Vec<_> = uninstall_targets(&lockfile, &bundlefile);
    if uninstall.is_empty() {
        return Ok(());
    }
    if !force {
        println!("Would uninstall packages:");
        print_grid(uninstall.iter().map(|x| x.id.as_str()));
        println!("Run `winget-bundle cleanup --force` to make these changes.");
        return Ok(());
    }

    let mut packages: BTreeMap<(Source, String), PackageEntry> = lockfile
        .packages
        .iter()
        .map(|x| ((x.source, x.id.clone()), x.clone()))
        .collect();
    let uninstalled = uninstall_all(uninstall, &mut packages).await?;

    let packages = packages.into_values().collect();
    if packages != lockfile.packages {
        let lockfile = lockfile::Lockfile::new(packages);
        save_lockfile(&lockfile, &lockfile_path).await?;
    }
    if uninstalled > 0 {
        println!("Uninstalled {uninstalled} packages");
    }
    Ok(())
}

async fn uninstall_all(
    uninstall: Vec<&PackageEntry>,
    packages: &mut BTreeMap<(Source, String), PackageEntry>,
) -> Result<u32> {
    let mut uninstalled = 0;
    for package in uninstall {
        if exists_in_package_manager(package.source, &package.id).await? {
            println!("Uninstalling {package}...");
            if let Err(err) = uninstall_package(package).await {
                eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m");
                continue;
            }
            uninstalled += 1;
        }
        let _ = packages.remove(&(package.source, package.id.clone()));
    }
    Ok(uninstalled)
}

fn uninstall_targets<'a>(
    lockfile: &'a Lockfile,
    bundlefile: &Bundlefile,
) -> Vec<&'a lockfile::PackageEntry> {
    lockfile
        .packages
        .iter()
        .filter(|x| !exists_in_bundlefile(bundlefile, x))
        .collect()
}

fn print_grid<'a>(items: impl Iterator<Item = &'a str>) {
    let mut grid = Grid::new(GridOptions {
        direction: Direction::LeftToRight,
        filling: Filling::Spaces(2),
    });
    for item in items {
        grid.add(item.into());
    }
    let width = terminal_size().map(|(Width(w), _)| w).unwrap_or(80);
    print!("{}", grid.fit_into_width(width as usize).unwrap())
}

fn exists_in_bundlefile(bundlefile: &Bundlefile, package: &PackageEntry) -> bool {
    bundlefile
        .entries
        .iter()
        .any(|x| x.source == package.source && x.id == package.id)
}

async fn uninstall_package(entry: &lockfile::PackageEntry) -> Result<()> {
    match entry.source {
        Source::Winget => {
            winget::uninstall(winget::Source::Winget, &entry.id).await?;
            Ok(())
        }
        Source::MsStore => unimplemented!(),
        Source::Scoop => unimplemented!(),
    }
}
