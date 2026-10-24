use std::path::{Path, PathBuf};
use anyhow::{Ok, Result, anyhow};
use serde_json::Value;
use std::process::Command;
use std::os::windows::process::CommandExt;

pub fn check_existing_install() -> Result<Option<PathBuf>> {
    let expected_path = PathBuf::from("C:/Program Files (x86)/YoRBo/yorbo.exe");
    println!("looking for {}", expected_path.to_string_lossy());
    if !expected_path.try_exists()? {
        println!("Not found");
        return Ok(None);
    }
    let ps_command = "(Get-CimInstance Win32_LogicalDisk).DeviceID | ConvertTo-Json -Compress";
    let json_str = powershell(ps_command)?;
    println!("searching drives: {}", json_str);
    let drives: Vec<PathBuf> = serde_json::from_str(&json_str)?;
    for drive in drives {
        let file1 = drive.join("EFI/BOOT/grubx64.efi");
        let file2 = drive.join("yorbo/grub.cfg");
        let file3 = drive.join("ventoy/caged_vtoy.img");
        if (file1.try_exists()? && file2.try_exists()? && file3.try_exists()?) {
            println!("Found ya");
            return Ok(Some(drive));
        }
    }
    return Ok(None);
}

pub fn powershell(arg: &str) -> Result<String> {
    let CREATE_NO_WINDOW: u32 = 0x08000000;
    let command = format!("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}", arg);
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}