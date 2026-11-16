use super::PartitionInfo;
use crate::run::sh_exec;
use anyhow::Result;
use serde::Deserialize;
use serde::de::{self, Deserializer};
use uuid::Uuid;

struct PartitionList(Vec<PartitionInfo>);

impl<'de> Deserialize<'de> for PartitionList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = serde_json::Value::deserialize(deserializer)?;
        let mut partitions = Vec::new();

        let blockdevices = val
            .get("blockdevices")
            .and_then(|v| v.as_array())
            .ok_or_else(|| de::Error::custom("missing blockdevices array in lsblk output"))?;

        for (disk_idx, dev) in blockdevices.iter().enumerate() {
            if let Some(children) = dev.get("children").and_then(|c| c.as_array()) {
                for child in children {
                    let Some(partition_number) = child
                        .get("partn")
                        .and_then(|p| p.as_u64())
                        .map(|n| n as u32)
                    else {
                        continue;
                    };

                    let size = child.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                    let start = child.get("start").and_then(|s| s.as_u64()).unwrap_or(0);
                    let offset = start * 512;

                    let partition_type = child
                        .get("fstype")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let system_type = child
                        .get("parttype")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let mountpoint = child
                        .get("mountpoint")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    let uuid_str = child
                        .get("partuuid")
                        .and_then(|v| v.as_str())
                        .or_else(|| child.get("uuid").and_then(|v| v.as_str()));
                    let guid = uuid_str.and_then(|s| Uuid::parse_str(s).ok());

                    // Calculate free space if mounted
                    let free = if let Some(ref mp) = mountpoint {
                        if let Ok(df_out) =
                            sh_exec(&format!("df --output=avail -B1 {} | tail -n 1", mp))
                        {
                            df_out.trim().parse::<u64>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    partitions.push(PartitionInfo {
                        disk_number: disk_idx as u32,
                        partition_number,
                        mountpoint,
                        size,
                        partition_type,
                        system_type,
                        guid,
                        offset,
                        free,
                    });
                }
            }
        }

        Ok(PartitionList(partitions))
    }
}

fn find_device_path(disk_number: u32, partition_number: u32) -> Result<String> {
    let output = sh_exec("lsblk -J -b -o NAME,PATH,PARTN")?;
    let parsed: PartitionList = serde_json::from_str(&output)?;

    let dev_path = format!("/dev/disk/by-path/...");
    let output_raw = sh_exec("lsblk -J -b -o NAME,PATH,PARTN")?;
    let val: serde_json::Value = serde_json::from_str(&output_raw)?;
    if let Some(devices) = val.get("blockdevices").and_then(|v| v.as_array()) {
        let mut current_disk = 0u32;
        for dev in devices {
            if let Some(children) = dev.get("children").and_then(|c| c.as_array()) {
                for child in children {
                    if child.get("partn").and_then(|p| p.as_u64()) == Some(partition_number as u64)
                        && current_disk == disk_number
                    {
                        if let Some(path) = child.get("path").and_then(|p| p.as_str()) {
                            return Ok(path.to_string());
                        }
                    }
                }
                current_disk += 1;
            }
        }
    }
    anyhow::bail!(
        "Partition not found for disk {} part {}",
        disk_number,
        partition_number
    )
}

impl PartitionInfo {
    pub fn ensure_mounted(&mut self) -> Result<String> {
        if let Some(mp) = &self.mountpoint {
            return Ok(mp.clone());
        }

        let dev_path = find_device_path(self.disk_number, self.partition_number)?;
        let mount_dir = format!("/mnt/disk{}part{}", self.disk_number, self.partition_number);

        sh_exec(&format!("mkdir -p {}", mount_dir))?;
        sh_exec(&format!("mount {} {}", dev_path, mount_dir))?;

        self.mountpoint = Some(mount_dir.clone());
        Ok(mount_dir)
    }

    pub fn shrink(&self, size: u32) -> Result<Self> {
        let _ = self
            .mountpoint
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("only a mounted volume can be shrunk"))?;

        let dev_path = find_device_path(self.disk_number, self.partition_number)?;
        let shrink_bytes = (size as u64) * 1024 * 1024;

        let script = format!(
            r#"
            dev="{dev_path}"
            disk=$(lsblk -no pkname "$dev")
            if [ -z "$disk" ]; then
                disk=$(echo "$dev" | sed 's/[0-9]*$//')
            else
                disk="/dev/$disk"
            fi
            partn="{partn}"

            current_size=$(blockdev --getsize64 "$dev")
            shrink_bytes=$(( {mb} * 1024 * 1024 ))
            if [ "$current_size" -le "$shrink_bytes" ]; then
                echo "Partition doesn't have {mb} MB to spare." >&2
                exit 1
            fi
            new_size_bytes=$(( current_size - shrink_bytes ))

            parted -s "$disk" unit B resizepart "$partn" "$new_size_bytes"
            parted -s "$disk" mkpart primary fat32 "${{new_size_bytes}}B" "100%"

            new_part_dev=$(lsblk -n -r -o PATH "$disk" | tail -n 1)
            mkfs.vfat -F 32 -n "YORBO" "$new_part_dev" >/dev/null

            echo "$new_part_dev"
            "#,
            dev_path = dev_path,
            partn = self.partition_number,
            mb = size,
        );

        sh_exec(&script)?;

        let partitions = get_partitions()?;
        let new_part = partitions
            .into_iter()
            .find(|p| {
                p.disk_number == self.disk_number
                    && p.partition_number != self.partition_number
                    && p.size == shrink_bytes
            })
            .ok_or_else(|| anyhow::anyhow!("Linux didn't report the new partition"))?;

        Ok(new_part)
    }
}

pub fn get_partitions() -> Result<Vec<PartitionInfo>> {
    let output = sh_exec(
        "lsblk -J -b -o NAME,PATH,SIZE,FSTYPE,PARTTYPE,PARTN,UUID,PARTUUID,START,MOUNTPOINT",
    )?;
    let parsed: PartitionList = serde_json::from_str(&output)?;
    Ok(parsed.0)
}

pub fn free_space(drive: char) -> Result<u64> {
    let path = if drive == '/' {
        "/".to_string()
    } else {
        format!("/mnt/{}", drive.to_ascii_lowercase())
    };
    let out = sh_exec(&format!("df --output=avail -B1 {} | tail -n 1", path))?;
    Ok(out.trim().parse()?)
}
