use serde::{Serialize, Deserialize};
use serde_json;
use std::process::Command;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct PartitionInfo {
    #[serde(rename = "DiskNumber")]
    pub disk_number: u32,
    #[serde(rename = "PartitionNumber")]
    pub partition_number: u32,
    #[serde(rename = "DriveLetter")]
    pub drive_letter: Option<char>,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "Type")]
    pub partition_type: String,
}



impl PartitionInfo {
    pub fn ensure_mounted(&self) -> Option<char> {
        if let Some(letter) = self.drive_letter {
            return Some(letter);
        }

        let ps_script = format!(
            r#"
            $disk = {}
            $part = {}
            $freeLetter = 68..90 | ForEach-Object {{ [char]$_ }} | Where-Object {{ -not (Test-Path "$_:\") }} | Select-Object -First 1
            if ($freeLetter) {{
                Set-Partition -DiskNumber $disk -PartitionNumber $part -NewDriveLetter $freeLetter
                Write-Output $freeLetter
            }}
            "#,
            self.disk_number, self.partition_number
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        String::from_utf8(output.stdout)
            .ok()?
            .trim()
            .chars()
            .next()
    }

    pub fn verify_format(&self) -> bool {
        let ps_fs_check = format!(
            r#"
            Get-Partition -DiskNumber {} -PartitionNumber {} | Get-Volume | Select-Object -ExpandProperty FileSystem
            "#,
            self.disk_number, self.partition_number
        );

        let output = match Command::new("powershell").args(["-NoProfile", "-Command", &ps_fs_check]).output() {
            Ok(out) => out,
            Err(_) => return false,
        };

        let filesystem = String::from_utf8_lossy(&output.stdout);
        if !filesystem.to_uppercase().contains("FAT32") {
            return false;
        }

        let was_already_mounted = self.drive_letter.is_some();
        let drive_letter = match self.ensure_mounted() {
            Some(letter) => letter,
            None => return false,
        };

        let base_path = format!("{}:\\", drive_letter);
        let file1 = Path::new(&base_path).join("EFI/BOOT/grubx64.efi");
        let file2 = Path::new(&base_path).join("yorbo/grub.cfg");
        let file3 = Path::new(&base_path).join("ventoy/caged_vtoy.img");

        let all_files_exist = file1.exists() && file2.exists() && file3.exists();

        if !was_already_mounted {
            let ps_unmount = format!(
                r#"
                Remove-PartitionAccessPath -DiskNumber {} -PartitionNumber {} -AccessPath "{}:\"
                "#,
                self.disk_number, self.partition_number, drive_letter
            );
            let _ = Command::new("powershell").args(["-NoProfile", "-Command", &ps_unmount]).output();
        }

        all_files_exist
    }
}


pub fn get_windows_partitions() -> Result<Vec<PartitionInfo>, Box<dyn std::error::Error>> {
    let ps_command = "@(Get-Partition) | Select-Object DiskNumber, PartitionNumber, DriveLetter, Size, Type | ConvertTo-Json -Compress";

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_command])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PowerShell command failed: {}", err).into());
    }

    let json_str = String::from_utf8(output.stdout)?;
    
    if json_str.trim().is_empty() {
        return Ok(Vec::new());
    }

    let partitions: Vec<PartitionInfo> = serde_json::from_str(&json_str)?;

    Ok(partitions)
}

pub fn find(disk_num: u32, part_num: u32) -> Option<PartitionInfo> {
    let partitions = get_windows_partitions().ok()?;
    
    partitions
        .into_iter()
        .find(|p| p.disk_number == disk_num && p.partition_number == part_num)
}