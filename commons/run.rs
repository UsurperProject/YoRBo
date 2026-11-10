use anyhow::{Ok, Result};
use std::os::windows::process::CommandExt;
use std::process::Command;

#[cfg(windows)]
pub fn powershell(arg: &str) -> Result<String> {
    let create_no_window: u32 = 0x08000000;
    let command = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $ErrorActionPreference = 'Stop'; {}",
        arg
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .creation_flags(create_no_window)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "{}",
            stderr
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("PowerShell failed")
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
