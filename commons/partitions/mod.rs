use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[serde(rename = "Type")]
    pub system_type: Option<String>, // If this is "System", then we have our ESP
    #[serde(rename = "Guid")]
    pub guid: Option<Uuid>,
    #[serde(rename = "Offset")]
    pub offset: u64,
    #[serde(rename = "SizeRemaining")]
    pub free: Option<u64>, // free bytes on the volume; None if Windows can't say
}

impl PartitionInfo {
    pub fn verify_format(&self) -> Result<bool> {
        if self.mountpoint.is_none() {
            return Err(anyhow!("You forgot to mount the thing first"));
        }
        if !self
            .partition_type
            .as_ref()
            .unwrap()
            .to_lowercase()
            .contains("fat32")
        {
            return Err(anyhow!(
                "Illegal partition type {}",
                self.partition_type.as_ref().unwrap()
            ));
        }
        #[cfg(windows)]
        let base_path = format!("{}:\\", self.mountpoint.as_ref().unwrap());
        #[cfg(linux)]
        let base_path = format!("{}/", self.mountpoint.as_ref().unwrap());
        let file1 = Path::new(&base_path).join("EFI/BOOT/yorbox64.efi");
        let file2 = Path::new(&base_path).join("yorbo/grub.cfg");
        let file3 = Path::new(&base_path).join("ventoy/caged_vtoy.img");

        return Ok(file1.exists() && file2.exists() && file3.exists());
    }
}

pub fn find(guid: Uuid) -> Result<PartitionInfo> {
    let partitions = get_partitions()?;
    for partition in partitions {
        if partition.guid.is_some() && partition.guid.unwrap() == guid {
            return Ok(partition);
        }
    }
    Err(anyhow!("failed to find partition"))
}
