use std::io::{self, Write};
use std::process::Stdio;

use anyhow::{Result, bail};
use futures::try_join;
use smol::io::AsyncReadExt;
use smol::process::Command;

const PREFIX: &str = "$Host.UI.RawUI.BufferSize = New-Object System.Management.Automation.Host.Size(32766, $Host.UI.RawUI.BufferSize.Height); ";

pub async fn has_cmd(cmd: &str) -> Result<bool> {
    let cmd = format!("Get-Command {cmd} -ErrorAction Stop > $null");
    Ok(Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &cmd])
        .status()
        .await?
        .success())
}

pub async fn exec(cmd: &str, args: &[&str]) -> Result<()> {
    let cmd = format!("{} {}", cmd, args.join(" "));
    let mut child = Command::new("powershell.exe")
        .args(["-NonInteractive", "-NoProfile", "-Command", &cmd])
        .env("PSModulePath", "")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_task = relay(stdout, io::stdout());
    let stderr_task = relay(stderr, io::stderr());
    let wait_task = child.status();

    let ((), (), status) = try_join!(stdout_task, stderr_task, wait_task)?;

    if !status.success() {
        bail!("Failed to {}", cmd);
    }
    Ok(())
}

async fn relay(mut reader: impl AsyncReadExt + Unpin, mut writer: impl Write) -> io::Result<()> {
    let mut buf = [0u8; 8192];

    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        let mut start = 0;
        for i in 0..n {
            if buf[i] == b'\r' {
                if start < i {
                    writer.write_all(&buf[start..i])?;
                }
                writer.write_all(b"\r\x1b[2K")?;
                start = i + 1;
            }
        }
        if start < n {
            writer.write_all(&buf[start..n])?;
        }

        writer.flush()?;
    }

    Ok(())
}

pub async fn exec_silent(cmd: &str, args: &[&str]) -> Result<()> {
    let cmd = format!("{PREFIX}{} {}", cmd, args.join(" "));
    let status = Command::new("powershell.exe")
        .args(["-NonInteractive", "-NoProfile", "-Command", &cmd])
        .env("PSModulePath", "")
        .stdout(Stdio::null())
        .status()
        .await?;
    if !status.success() {
        bail!("Failed to {cmd}");
    }
    Ok(())
}

pub async fn exec_output(cmd: &str, args: &[&str]) -> Result<String> {
    let cmd = format!("{PREFIX}{cmd} {}", args.join(" "));
    let output = Command::new("powershell.exe")
        .args(["-NonInteractive", "-NoProfile", "-Command", &cmd])
        .env("PSModulePath", "")
        .output()
        .await?;
    if !output.status.success() {
        bail!("Failed to {}", cmd);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
