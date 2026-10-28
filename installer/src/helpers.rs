use std::path::{Path, PathBuf};
use anyhow::{Ok, Result, anyhow};
use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::os::windows::process::CommandExt;

#[derive(Serialize, Deserialize, Debug)]
pub struct Partition {
    #[serde(rename = "DiskNumber")]
    pub disk_number: u32,
    #[serde(rename = "PartitionNumber")]
    pub partition_number: u32,
    #[serde(rename = "DriveLetter")]
    pub mountpoint: Option<String>,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "FileSystem")]
    pub partition_type: Option<String>,
}


pub fn powershell(arg: &str) -> Result<String> {
    let create_no_window: u32 = 0x08000000;
    let command = format!("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}", arg);
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .creation_flags(create_no_window)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn list_vols() -> Result<Vec<Partition>> {
    let ps_command = "ConvertTo-Json -InputObject @(Get-Partition | Select-Object DiskNumber, PartitionNumber, DriveLetter, Size, @{N='FileSystem'; E={($_ | Get-Volume -EA Si).FileSystem}},) -Compress";
    let output = powershell(ps_command)?;
    let partitions: Vec<Partition> = serde_json::from_str(&output)?;
    Ok(partitions)
}