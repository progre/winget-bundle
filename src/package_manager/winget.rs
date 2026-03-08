use std::{ops::Not, str::FromStr};

use anyhow::{Context, bail};
use itertools::Itertools;
use smol::process::Command;

use crate::{
    file::{bundlefile, statefile},
    package_manager::table_parser::{ColumnWidthBasis, parse_table},
};

#[derive(Clone)]
pub struct PackageEntry {
    pub source: Option<Source>,
    pub id: String,
    pub name: String,
    version: String,
    available: Option<String>,
}

impl PackageEntry {
    pub fn is_upgradable(&self) -> bool {
        self.version != "Unknown" && self.available.is_some()
    }

    pub fn as_bundlefile_key(&self) -> Option<bundlefile::CompositeKey<'_>> {
        self.source
            .map(|source| bundlefile::CompositeKey::new(source.into(), self.id.as_str()))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Source {
    Winget,
    MsStore,
}

impl Source {
    fn as_str(&self) -> &str {
        match self {
            Self::Winget => "winget",
            Self::MsStore => "msstore",
        }
    }
}

impl From<statefile::Source> for Source {
    fn from(value: statefile::Source) -> Self {
        match value {
            statefile::Source::Winget => Self::Winget,
        }
    }
}

impl FromStr for Source {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "winget" => Ok(Self::Winget),
            "msstore" => Ok(Self::MsStore),
            _ => bail!("Unknown source: {s}"),
        }
    }
}

fn key(source: Source) -> &'static str {
    match source {
        Source::Winget => "--id",
        Source::MsStore => "--name",
    }
}

pub async fn install(source: Source, package: &str) -> anyhow::Result<()> {
    let status = Command::new("winget")
        .args([
            "install",
            "--accept-source-agreements",
            "--accept-package-agreements",
            "--source",
            source.as_str(),
            key(source),
            package,
        ])
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to install {}", package))
    }
}

pub async fn upgrade(source: Source, package: &str) -> anyhow::Result<()> {
    let status = Command::new("winget")
        .args([
            "upgrade",
            "--accept-source-agreements",
            "--accept-package-agreements",
            "--source",
            source.as_str(),
            key(source),
            package,
        ])
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to upgrade {}", package))
    }
}

pub async fn list() -> anyhow::Result<Vec<PackageEntry>> {
    let output = Command::new("winget")
        .args(["list", "--accept-source-agreements"])
        .output()
        .await?;
    if !output.status.success() {
        bail!("Failed to list packages");
    }
    let output = String::from_utf8_lossy(&output.stdout);
    let mut output = output.lines();
    let first_line = output.next();
    let first_line = first_line.and_then(|x| x.split('\r').next_back());
    let lines = first_line.into_iter().chain(output);
    let (column_len, cells) =
        parse_table(lines, ColumnWidthBasis::Header).context("Failed to parse winget list")?;
    if column_len != 5 || cells[..5] != ["Name", "Id", "Version", "Available", "Source"] {
        bail!("Invalid header: {first_line:?}");
    }
    Ok(cells
        .into_iter()
        .skip(column_len)
        .chunks(column_len)
        .into_iter()
        .map(|mut columns| {
            let name = columns.next().unwrap();
            let id = columns.next().unwrap();
            let version = columns.next().unwrap();
            let available = columns.next().unwrap();
            let available = available.is_empty().not().then_some(available);
            let source = columns.next().unwrap().parse().ok();
            PackageEntry {
                source,
                id,
                name,
                version,
                available,
            }
        })
        .collect())
}

pub async fn uninstall(source: Source, package: &str) -> anyhow::Result<()> {
    let status = Command::new("winget")
        .args([
            "uninstall",
            "--accept-source-agreements",
            "--source",
            source.as_str(),
            key(source),
            package,
        ])
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to uninstall {}", package))
    }
}
