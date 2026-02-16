use smol::process::Command;

pub async fn install(package: &str) -> anyhow::Result<()> {
    let status = Command::new("winget")
        .args(["install", package])
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to fetch {}", package))
    }
}

pub async fn uninstall(package: &str) -> anyhow::Result<()> {
    let status = Command::new("winget")
        .args(["uninstall", package])
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to uninstall {}", package))
    }
}
