use std::path::{Path, PathBuf};
use anyhow::{Ok, Result, anyhow};
use serde_json::Value;
use std::process::Command;
use std::os::windows::process::CommandExt;

pub fn powershell(arg: &str) -> Result<String> {
    let CREATE_NO_WINDOW: u32 = 0x08000000;
    let command = format!("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}", arg);
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}