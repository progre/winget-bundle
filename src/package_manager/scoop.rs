use std::process::Stdio;

use anyhow::{Context, Result, bail};
use const_format::formatcp;
use futures::{future::try_join_all, try_join};
use itertools::Itertools;
use smol::process::Command;

use crate::{
    file::bundlefile,
    package_manager::table_parser::{ColumnWidthBasis, parse_table},
};

/// https://github.com/ScoopInstaller/Scoop/blob/5c896e901fafbe371b39673129120e3c88496a39/lib/depends.ps1#L103
pub const INSTALLATION_HELPERS: [&str; 4] = ["7zip", "lessmsi", "innounp", "dark"];

const SCOOP_PREFIX: &str =
    "$Host.UI.RawUI.BufferSize = @{Width=65536; Height=$Host.UI.RawUI.BufferSize.Height}; scoop ";

#[derive(Clone, Debug)]
pub struct PackageEntry {
    pub name: String,
    pub _installed_version: String,
    pub _source: String,
    latest_version: Option<String>,
    pub dependencies: Vec<String>,
}

impl PackageEntry {
    pub fn is_upgradable(&self) -> bool {
        self.latest_version.is_some()
    }

    pub fn as_bundlefile_key(&self) -> bundlefile::CompositeKey<'_> {
        bundlefile::CompositeKey::new(bundlefile::Source::Scoop, self.name.as_str())
    }
}

pub async fn install(name: &str) -> Result<()> {
    exec_update_self().await?;
    exec(&["install", name]).await
}

pub async fn upgrade(name: &str) -> Result<()> {
    exec_update_self().await?;
    exec(&["update", name]).await
}

pub async fn installed_packages() -> Result<Vec<PackageEntry>> {
    exec_update_self().await?;
    let (list, status) = try_join!(list(), status())?;
    let iter = list
        .into_iter()
        .map(|[name, installed_version, source, ..]| async {
            let dependencies = depends(&name)
                .await?
                .into_iter()
                .map(|[_, name]| name)
                .collect();
            let latest_version = status
                .iter()
                .find(|[n, ..]| n == &name)
                .map(|[_, _, latest, ..]| latest.to_owned());
            Ok::<_, anyhow::Error>(PackageEntry {
                name,
                _installed_version: installed_version,
                _source: source,
                latest_version,
                dependencies,
            })
        });
    try_join_all(iter).await
}

async fn list() -> Result<Vec<[String; 5]>> {
    let output = exec_output(&["list"]).await?;
    let (column_count, list_cells) = parse_table(output.lines(), ColumnWidthBasis::SeparatorLine)
        .context("Failed to parse scoop list")?;
    const LEN: usize = 5;
    const COLS: [&str; LEN] = ["Name", "Version", "Source", "Updated", "Info"];
    if column_count != LEN || list_cells.len() % column_count != 0 || list_cells[0..LEN] != COLS {
        bail!("Invalid header: {output}");
    }
    let list = list_cells
        .into_iter()
        .skip(LEN)
        .chunks(LEN)
        .into_iter()
        .map(|x| x.collect_array::<LEN>().unwrap())
        .collect::<Vec<_>>();
    Ok(list)
}

async fn status() -> Result<Vec<[String; 5]>> {
    let output = exec_output(&["status", "--local"]).await?;
    let (column_count, status_cells) = parse_table(output.lines(), ColumnWidthBasis::SeparatorLine)
        .context("Failed to parse scoop status")?;
    const LEN: usize = 5;
    const COLS: [&str; LEN] = [
        "Name",
        "Installed Version",
        "Latest Version",
        "Missing Dependencies",
        "Info",
    ];
    if column_count != LEN || status_cells.len() % column_count != 0 || status_cells[..LEN] != COLS
    {
        bail!("Invalid header: {output}");
    }
    let status = status_cells
        .into_iter()
        .skip(LEN)
        .chunks(LEN)
        .into_iter()
        .map(|x| x.collect_array::<LEN>().unwrap())
        .collect::<Vec<_>>();
    Ok(status)
}

async fn depends(name: &str) -> Result<Vec<[String; 2]>> {
    let output = exec_output(&["depends", name]).await?;
    let (column_count, status_cells) = parse_table(output.lines(), ColumnWidthBasis::SeparatorLine)
        .context("Failed to parse scoop depends")?;
    const LEN: usize = 2;
    const COLS: [&str; LEN] = ["Source", "Name"];
    if column_count != LEN || status_cells.len() % column_count != 0 || status_cells[..LEN] != COLS
    {
        bail!("Invalid header: {output}");
    }
    let status = status_cells
        .into_iter()
        .skip(LEN)
        .chunks(LEN)
        .into_iter()
        .map(|x| x.collect_array::<LEN>().unwrap())
        .filter(|[_, n]| n != name)
        .collect::<Vec<_>>();
    Ok(status)
}

pub async fn uninstall(name: &str) -> Result<()> {
    exec(&["uninstall", name]).await
}

async fn exec(args: &[&str]) -> Result<()> {
    let cmd = format!("{SCOOP_PREFIX}{}", args.join(" "));
    let status = Command::new("powershell.exe")
        .args(["-NonInteractive", "-NoProfile", "-Command", &cmd])
        .env("PSModulePath", "")
        .status()
        .await?;
    if !status.success() {
        bail!("Failed to {}", args.join(" "));
    }
    Ok(())
}

async fn exec_update_self() -> Result<()> {
    exec_silent(formatcp!("{SCOOP_PREFIX}update")).await
}

async fn exec_silent(cmd: &str) -> Result<()> {
    let status = Command::new("powershell.exe")
        .args(["-NonInteractive", "-NoProfile", "-Command", cmd])
        .env("PSModulePath", "")
        .stdout(Stdio::null())
        .status()
        .await?;
    if !status.success() {
        bail!("Failed to {cmd}");
    }
    Ok(())
}

async fn exec_output(args: &[&str]) -> Result<String> {
    let cmd = format!("{SCOOP_PREFIX}{}", args.join(" "));
    let output = Command::new("powershell.exe")
        .args(["-NonInteractive", "-NoProfile", "-Command", &cmd])
        .env("PSModulePath", "")
        .output()
        .await?;
    if !output.status.success() {
        bail!("Failed to {}", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
