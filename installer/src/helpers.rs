use std::path::{Path, PathBuf};
use anyhow::{Ok, Result, anyhow, bail};
use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::process::Command;
use std::os::windows::process::CommandExt;

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[serde(rename = "Type")]
    pub system_type: Option<String> // If this is "System", then we have our ESP
}

impl Partition {
    /// Gives the partition a drive letter if it has none (e.g. the ESP) and returns it.
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
            bail!("Windows didn't assign a drive letter (got {:?})", letter);
        }
        self.mountpoint = Some(letter.clone());
        Ok(letter)
    }

    pub fn shrink(&self, size: u32) -> Result<Self> {
        let Some(letter) = &self.mountpoint else {
            bail!("only a mounted volume can be shrunk");
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
        let new_part = serde_json::from_str::<Vec<Partition>>(&output)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Windows didn't report the new partition"))?;
        Ok(new_part)
    }
}

pub struct ProgressWriter<W: Write, F: FnMut(u64)> {
    inner: W,
    written: u64,
    on_write: F,
}

impl<W: Write, F: FnMut(u64)> ProgressWriter<W, F> {
    pub fn new(inner: W, on_write: F) -> Self {
        Self { inner, written: 0, on_write }
    }
}

impl<W: Write, F: FnMut(u64)> Write for ProgressWriter<W, F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.written += n as u64;
        (self.on_write)(self.written);
        io::Result::Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

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
        bail!("{}", stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("PowerShell failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

const PARTITION_FIELDS: &str = "DiskNumber, PartitionNumber, \
    @{N='Type'; E={[string]$_.Type}}, \
    @{N='DriveLetter'; E={if ([int]$_.DriveLetter -ne 0) {[string]$_.DriveLetter} else {$null}}}, \
    Size, \
    @{N='FileSystem'; E={($_ | Get-Volume -EA SilentlyContinue).FileSystem}}";

pub fn list_vols() -> Result<Vec<Partition>> {
    let ps_command = format!(
        "ConvertTo-Json -InputObject @(Get-Partition | Select-Object {}) -Compress",
        PARTITION_FIELDS
    );
    let output = powershell(&ps_command)?;
    let partitions: Vec<Partition> = serde_json::from_str(&output)?;
    Ok(partitions)
}

pub fn free_space(drive: char) -> Result<u64> {
    let out = powershell(&format!("(Get-Volume -DriveLetter {drive}).SizeRemaining"))?;
    Ok(out.trim().parse()?)
}
