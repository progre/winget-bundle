use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use anyhow::Result;

use super::bundlefile::Source;

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Lockfile {
    pub version: u8,

    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "package")]
    pub packages: Vec<PackageEntry>,
}

impl Lockfile {
    pub fn new(packages: Vec<PackageEntry>) -> Self {
        Self {
            version: 0,
            packages,
        }
    }
}

impl FromStr for Lockfile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| anyhow::anyhow!("failed to parse Bundlefile.lock: {}", e))
    }
}

impl Display for Lockfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = toml::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", s)
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

impl Display for PackageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name}")
        } else {
            write!(f, "{}", self.id)
        }
    }
}
