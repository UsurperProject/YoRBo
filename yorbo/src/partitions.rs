use serde::{Serialize, Deserialize};
use serde_json;
use std::process::Command;
use std::path::Path;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use crate::partitions;

#[derive(Serialize, Deserialize, Debug)]
pub struct PartitionInfo {
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
    #[serde(rename = "Guid")]
    pub guid: Uuid,
    #[serde(rename = "Offset")]
    pub offset: u64
}



impl PartitionInfo {
    #[cfg(windows)]
    pub fn ensure_mounted(&self) -> Result<()> {
        if let Some(mp) = &self.mountpoint {
            return Ok(());
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
            .ok().unwrap();
        
        if !output.status.success() {
            return Err(anyhow!("mount failed"));
        }

        Ok(())
    }

    pub fn verify_format(&self) -> Result<bool> {
        if self.mountpoint.is_none() {
            return Err(anyhow!("You forgot to mount the thing first"));
        }
        if !self.partition_type.as_ref().unwrap().to_lowercase().contains("fat32") {
            return Err(anyhow!("Illegal partition type {}", self.partition_type.as_ref().unwrap()));
        }
        #[cfg(windows)]
        let base_path = format!("{}:\\", self.mountpoint.as_ref().unwrap());
        #[cfg(linux)]
        let base_path = format!("{}/", mountpoint);
        let file1 = Path::new(&base_path).join("EFI/BOOT/grubx64.efi");
        let file2 = Path::new(&base_path).join("yorbo/grub.cfg");
        let file3 = Path::new(&base_path).join("ventoy/caged_vtoy.img");

        return Ok(file1.exists() && file2.exists() && file3.exists());
    }
}

#[cfg(windows)]
pub fn get_partitions() -> Result<Vec<PartitionInfo>> {
    let ps_command = "ConvertTo-Json -InputObject @(Get-Partition | Select-Object DiskNumber, PartitionNumber, DriveLetter, Size, @{N='FileSystem'; E={($_ | Get-Volume -EA Si).FileSystem}}, Guid, Offset) -Compress";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", ps_command])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failef to run ps command"));
    }

    let json_str = String::from_utf8(output.stdout)?;
    
    if json_str.trim().is_empty() {
        return Ok(Vec::new());
    }

    let partitions: Vec<PartitionInfo> = serde_json::from_str(&json_str)?;

    Ok(partitions)
}


pub fn find(guid: Uuid) -> Result<PartitionInfo> {
    let partitions = get_partitions()?;
    for partition in partitions {
        if partition.guid == guid {
            return Ok(partition);
        }
    }
    Err(anyhow!("failed to find partition"))
}