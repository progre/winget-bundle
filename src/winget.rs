use std::process::Stdio;

use smol::process::Command;

#[derive(Clone, Copy)]
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

fn key(source: Source) -> &'static str {
    match source {
        Source::Winget => "--id",
        Source::MsStore => "--name",
    }
}

pub async fn install(source: Source, package: &str) -> anyhow::Result<()> {
    let status = Command::new("winget")
        .args(["install", "--source", source.as_str(), key(source), package])
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to fetch {}", package))
    }
}

pub async fn exists(source: Source, package: &str) -> anyhow::Result<bool> {
    let status = Command::new("winget")
        .args(["list", "--source", source.as_str(), key(source), package])
        .stdout(Stdio::null())
        .status()
        .await?;

    Ok(status.success())
}

pub async fn uninstall(source: Source, package: &str) -> anyhow::Result<()> {
    let status = Command::new("winget")
        .args([
            "uninstall",
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
