use super::PartitionInfo;
use crate::run::powershell;
use anyhow::Result;
use serde_json;

const PARTITION_FIELDS: &str = "DiskNumber, PartitionNumber, \
    @{N='Type'; E={[string]$_.Type}}, \
    @{N='DriveLetter'; E={if ([int]$_.DriveLetter -ne 0) {[string]$_.DriveLetter} else {$null}}}, \
    Size, \
    @{N='FileSystem'; E={($_ | Get-Volume -EA SilentlyContinue).FileSystem}}, \
    @{N='Guid'; E={if ($_.Guid) {[string]$_.Guid} else {'00000000-0000-0000-0000-000000000000'}}}, \
    @{N='Offset'; E={[string]$_.Offset}},
    @{N='SizeRemaining'; E={($_ | Get-Volume -EA SilentlyContinue).SizeRemaining}}";

impl PartitionInfo {
    pub fn ensure_mounted(&mut self) -> Result<String> {
        if let Some(mp) = &self.mountpoint {
            return Ok(mp.clone());
        }

        let ps_script = format!(
            "Add-PartitionAccessPath -DiskNumber {d} -PartitionNumber {p} -AssignDriveLetter; \
             (Get-Partition -DiskNumber {d} -PartitionNumber {p}).DriveLetter",
            d = self.disk_number,
            p = self.partition_number
        );

        let letter = powershell(&ps_script)?;
        if letter.len() != 1 || !letter.chars().all(|c| c.is_ascii_alphabetic()) {
            anyhow::bail!("Windows didn't assign a drive letter (got {:?})", letter);
        }
        self.mountpoint = Some(letter.clone());
        Ok(letter)
    }

    pub fn shrink(&self, size: u32) -> Result<Self> {
        let Some(letter) = &self.mountpoint else {
            anyhow::bail!("only a mounted volume can be shrunk");
        };
        let ps_script = format!(
            r#"
            $disk = {disk}; $part = {part}; $mb = {mb}
            $current = (Get-Partition -DiskNumber $disk -PartitionNumber $part).Size
            $min = (Get-PartitionSupportedSize -DiskNumber $disk -PartitionNumber $part).SizeMin
            $target = $current - ($mb * 1MB)
            if ($target -lt $min) {{ throw "{letter}: doesn't have $mb MB to spare. Free up some space and try again." }}
            Resize-Partition -DiskNumber $disk -PartitionNumber $part -Size $target
            $new = New-Partition -DiskNumber $disk -Size ($mb * 1MB) -AssignDriveLetter
            $new | Format-Volume -FileSystem FAT32 -NewFileSystemLabel 'YORBO' -Confirm:$false | Out-Null
            $new = Get-Partition -DiskNumber $disk -PartitionNumber $new.PartitionNumber
            ConvertTo-Json -InputObject @($new | Select-Object {fields}) -Compress
            "#,
            disk = self.disk_number,
            part = self.partition_number,
            mb = size,
            letter = letter,
            fields = PARTITION_FIELDS,
        );

        let output = powershell(&ps_script)?;
        let new_part = serde_json::from_str::<Vec<PartitionInfo>>(&output)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Windows didn't report the new partition"))?;
        Ok(new_part)
    }
}

pub fn get_partitions() -> Result<Vec<PartitionInfo>> {
    let ps_command = format!(
        "ConvertTo-Json -InputObject @(Get-Partition | Select-Object {}) -Compress",
        PARTITION_FIELDS
    );
    let output = powershell(&ps_command)?;
    let partitions: Vec<PartitionInfo> = serde_json::from_str(&output)?;
    Ok(partitions)
}

pub fn free_space(drive: char) -> Result<u64> {
    let out = powershell(&format!("(Get-Volume -DriveLetter {drive}).SizeRemaining"))?;
    Ok(out.trim().parse()?)
}
