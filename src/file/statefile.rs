use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use anyhow::{Result, bail};

use crate::{file::bundlefile, package_manager::winget};

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub enum Source {
    #[serde(rename = "winget")]
    Winget,
}

impl TryFrom<bundlefile::Source> for Source {
    type Error = anyhow::Error;

    fn try_from(value: bundlefile::Source) -> Result<Self> {
        Ok(match value {
            bundlefile::Source::Winget => Self::Winget,
            bundlefile::Source::MsStore => {
                bail!("MsStore packages are managed directly via winget, not via statefile")
            }
            bundlefile::Source::Scoop => {
                bail!("Scoop packages are managed directly via scoop, not via statefile")
            }
        })
    }
}

impl TryFrom<winget::Source> for Source {
    type Error = anyhow::Error;

    fn try_from(value: winget::Source) -> Result<Self> {
        Ok(match value {
            winget::Source::Winget => Self::Winget,
            winget::Source::MsStore => {
                bail!("MsStore packages are managed directly via winget, not via statefile")
            }
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Statefile {
    pub version: u8,

    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "package")]
    pub packages: Vec<PackageEntry>,
}

impl Statefile {
    pub fn new(packages: Vec<PackageEntry>) -> Self {
        Self {
            version: 0,
            packages,
        }
    }
}

impl Display for Statefile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = toml::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", s)
    }
}

impl FromStr for Statefile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| anyhow::anyhow!("failed to parse Bundlefile.state: {}", e))
    }
}

/// ロックに記録する単一パッケージのエントリ
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PackageEntry {
    pub source: Source,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PackageEntry {
    pub fn new(source: Source, id: String, name: Option<String>) -> Self {
        Self { source, id, name }
    }
}

impl Display for PackageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name}")
        } else {
            write!(f, "{}", self.id)
        }
    }
}
