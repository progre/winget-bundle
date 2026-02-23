use std::str::FromStr;

use anyhow::bail;
use smol::process::Command;

use crate::winget_list_parser::parse_package_entries;

#[derive(Clone)]
pub struct PackageEntry {
    pub source: Option<Source>,
    pub id: String,
    pub _name: String,
    pub version: String,
    pub available: Option<String>,
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
    parse_package_entries(&String::from_utf8_lossy(&output.stdout))
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
