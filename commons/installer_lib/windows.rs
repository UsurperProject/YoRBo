use super::IMAGE_TYPES;
use super::helpers::ProgressWriter;
use crate::config::Config;
use crate::partitions::{PartitionInfo, free_space};
use crate::run::powershell;
use anyhow::{Ok, Result, anyhow};
use std::{
    fs::{self, File},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};
use zip::ZipArchive;

const VENTOY_ICON: &[u8] = include_bytes!("../../icons/ventoy.ico");

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
        let file1 = drive.join("EFI/BOOT/yorbox64.efi");
        let file2 = drive.join("yorbo/grub.cfg");
        let file3 = drive.join("ventoy/caged_vtoy.img");
        if (file1.try_exists()? && file2.try_exists()? && file3.try_exists()?) {
            return Ok(Some(drive));
        }
    }
    return Ok(None);
}

pub fn create_ventoy_shortcut(runner_dir: &Path) -> Result<()> {
    let exe = runner_dir.join("yorbo.exe");
    let icon = runner_dir.join("ventoy.ico");
    fs::write(&icon, VENTOY_ICON)?;

    let q = |p: &Path| p.to_string_lossy().replace('\'', "''"); // for PowerShell '...' strings
    powershell(&format!(
        r#"
        $lnk = Join-Path ([Environment]::GetFolderPath('CommonDesktopDirectory')) 'Boot into Ventoy.lnk'
        $s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
        $s.TargetPath = '{exe}'
        $s.Arguments = '-b ventoy -r'
        $s.WorkingDirectory = '{dir}'
        $s.IconLocation = '{icon},0'
        $s.Description = 'Reboot into Ventoy with YoRBo'
        $s.Save()
        "#,
        exe = q(&exe),
        dir = q(runner_dir),
        icon = q(&icon),
    ))?;
    Ok(())
}

pub fn register_context_menu(runner_dir: &Path) -> Result<()> {
    let exe = runner_dir.join("yorbo.exe");
    let exe = exe.to_string_lossy().replace("/", "\\");
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    for ext in IMAGE_TYPES {
        let (verb, _) = hklm.create_subkey(format!(
            r"SOFTWARE\Classes\SystemFileAssociations\.{ext}\shell\YoRBo"
        ))?;
        verb.set_value("", &"Boot once with YoRBo")?;
        verb.set_value("Icon", &format!("\"{exe}\",0"))?;
        verb.set_value("MultiSelectModel", &"Single")?; // hidden when several files are selected
        let (command, _) = verb.create_subkey("command")?;
        command.set_value("", &format!("\"{exe}\" -b iso -f \"%1\" -r"))?;
    }
    Ok(())
}
