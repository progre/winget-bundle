use anyhow::{Result, bail};
use pest::Parser;
use pest::iterators::Pair;
use std::fmt::Display;
use std::str::FromStr;

use crate::winget;

#[derive(pest_derive::Parser)]
#[grammar = "file/bundlefile.pest"]
struct BundlefileParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundlefile {
    pub entries: Vec<PackageEntry>,
}

impl FromStr for Bundlefile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        parse_file(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    pub source: Source,
    pub id: String,
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

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub enum Source {
    #[serde(rename = "winget")]
    Winget,
    #[serde(rename = "msstore")]
    MsStore,
    #[serde(rename = "scoop")]
    Scoop,
}

impl FromStr for Source {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "winget" => Ok(Source::Winget),
            "msstore" => Ok(Source::MsStore),
            "scoop" => Ok(Source::Scoop),
            _ => bail!("Invalid command: {}", s),
        }
    }
}

impl From<winget::Source> for Source {
    fn from(value: winget::Source) -> Self {
        match value {
            winget::Source::Winget => Source::Winget,
            winget::Source::MsStore => Source::MsStore,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Bool(bool),
}

fn parse_file(content: &str) -> Result<Bundlefile> {
    let mut pairs = BundlefileParser::parse(Rule::file, content)?;
    let file = pairs.next().unwrap();
    debug_assert!(file.as_rule() == Rule::file);
    let elements = file
        .into_inner()
        .filter(|x| x.as_rule() == Rule::element)
        .map(parse_element)
        .collect::<Result<_>>()?;
    Ok(Bundlefile { entries: elements })
}

fn parse_element(element: Pair<'_, Rule>) -> Result<PackageEntry> {
    debug_assert!(element.as_rule() == Rule::element);

    let mut inner = element.into_inner();
    let command = inner.next().unwrap();
    debug_assert!(command.as_rule() == Rule::source);
    let command = command.as_str().parse().unwrap();

    let id = inner.next().unwrap();
    debug_assert!(id.as_rule() == Rule::string);
    let id = id.into_inner().next().unwrap().as_str().to_string();

    let mut name = None;
    for (key, value) in inner.map(parse_option) {
        match key.as_str() {
            "name" => {
                let Value::String(n) = value else {
                    bail!("Expected string value for 'name' option, got {:?}", value);
                };
                name = Some(n);
            }
            _ => bail!("Unknown option: {}", key),
        }
    }
    Ok(PackageEntry {
        source: command,
        id,
        name,
    })
}

fn parse_option(option: Pair<'_, Rule>) -> (String, Value) {
    debug_assert!(option.as_rule() == Rule::option);

    let mut inner = option.into_inner();

    let key = inner.next().unwrap();
    debug_assert!(key.as_rule() == Rule::key);
    let key = key.as_str().to_string();

    let value = inner.next().unwrap();
    debug_assert!(value.as_rule() == Rule::value);

    let value = value.into_inner().next().unwrap();
    match value.as_rule() {
        Rule::boolean => (key, Value::Bool(value.as_str().parse::<bool>().unwrap())),
        Rule::string => (
            key,
            Value::String(value.into_inner().next().unwrap().as_str().to_string()),
        ),
        _ => unreachable!(),
    }
}
