use anyhow::Result;
use term_grid::{Direction, Filling, Grid, GridOptions};
use terminal_size::{Width, terminal_size};

use super::load_files;
use crate::command::save_lockfile;
use crate::file::bundlefile::{Bundlefile, Source};
use crate::file::lockfile::{self, PackageEntry};
use crate::winget;

pub async fn cleanup(force: bool) -> Result<()> {
    let (bundlefile, mut lockfile, lockfile_path) = load_files().await?;
    let count = lockfile.packages.len();
    let uninstall: Vec<_> = lockfile
        .packages
        .clone()
        .into_iter()
        .filter(|x| !exists(&bundlefile, x))
        .collect();
    if uninstall.is_empty() {
        return Ok(());
    }
    if !force {
        println!("Would uninstall packages:");
        print_grid(uninstall.iter().map(|x| x.id.as_str()));
        println!("Run `winget-bundle cleanup --force` to make these changes.");
        return Ok(());
    }

    for package in uninstall {
        println!("Uninstalling {}...", package.id);
        match uninstall_package(&package).await {
            Ok(()) => {
                let pos = lockfile
                    .packages
                    .iter()
                    .position(|x| x.source == package.source && x.id == package.id)
                    .unwrap();
                lockfile.packages.swap_remove(pos);
            }
            Err(err) => eprintln!("\x1b[31m`winget-bundle` failed! {err}\x1b[0m"),
        }
    }

    lockfile
        .packages
        .sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.id.cmp(&b.id)));

    save_lockfile(&lockfile, &lockfile_path).await?;
    println!("Uninstalled {} packages", count - lockfile.packages.len());
    Ok(())
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

fn exists(bundlefile: &Bundlefile, package: &PackageEntry) -> bool {
    bundlefile
        .entries
        .iter()
        .any(|x| x.source == package.source && x.id == package.id)
}

async fn uninstall_package(entry: &lockfile::PackageEntry) -> Result<()> {
    match entry.source {
        Source::Winget => {
            winget::uninstall(&entry.id).await?;
            Ok(())
        }
        Source::MsStore => unimplemented!(),
        Source::Scoop => unimplemented!(),
    }
}
