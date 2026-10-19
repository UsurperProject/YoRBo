use anyhow::{Context, Result};
use hadris::fat::exfat::sync::ExFatFs;
use hadris::fs::sync::Volume;
use hadris::fs::OpenOptions;
use hadris::host::FileDevice;
use hadris::io::sync::Write;
use hadris::part::{self, Partition};
use hadris::sync::{open, AnyFs};
use std::path::{PathBuf, Path};
use crate::vlnk::create_vlnk;
use hadris::fs::MountOptions;
use std::process::Command;
use std::fs;
use grubenv::GrubEnv;
use efivar::boot::{
    BootEntry, BootEntryAttributes, BootVarWriter, EFIHardDrive, EFIHardDriveType, FilePath,
    FilePathList,
};
use efivar::efi::{Variable, VariableFlags};
use efivar::{VarEnumerator, VarWriter};





type FatVolume = Volume<hadris::fat::exfat::sync::ExFatFs<hadris::storage::Partition<FileDevice>>>;

fn get_fat_volume(drive: char) -> Result<FatVolume> {
    let img_path: PathBuf = PathBuf::from(format!("{}:/ventoy/caged_vtoy.img", drive)); 
    let image = std::fs::OpenOptions::new()
    .read(true)
    .write(true)          // ← this is what FileDevice::open omits
    .open(img_path)?;
    let mut device = FileDevice::new(image).context("failed to load image")?;
    let table = part::sync::read(&mut device).context("Failed to read img partition tabl")?;

    let mut exfat_partition: Option<Partition> = None;
    for partition in table.partitions() {
        let slice = match part::sync::open(&mut device, &partition) {
            Ok(s) => s,
            Err(_) => continue, 
        };


        match open(slice, MountOptions::new()) {
            Ok(AnyFs::ExFat(exfat_fs)) => {
                println!("Found exFAT partition #{}", partition.index());
                exfat_partition = Some(partition);
                break;
            }
            Ok(_) => {
                // Not exFAT, keep looking
                continue;
            }
            Err(_) => {
                // Not a filesystem we can mount, keep looking
                continue;
            }
        }
    }
    if exfat_partition.is_none() {
        eprintln!("Failed to find exfat partition");
        std::process::exit(404);
    }
    let slice = part::sync::open(device, exfat_partition.as_ref().unwrap()).unwrap();
    let container: ExFatFs<_> = ExFatFs::mount(slice, MountOptions::new()).unwrap();
    Ok(Volume::new(container))
}

pub fn set_iso_boot(drive: char, image: PathBuf) -> Result<()>{
    let volume = get_fat_volume(drive)
    .context("Unable to find suitable exfat volume")?;
    let filename = format!("yorbo_fast.vlnk.{}", image.extension().unwrap().to_str().unwrap().to_lowercase());
    set_ventoy_boot(drive).context("Failed to delete stuff")?;
    let mut vlnk = volume.open(filename,  OpenOptions::new().write().create().truncate())?;
    println!("file creation fails ?");
    let mut vlnk_data = create_vlnk(image).context("vlnk creation failed")?;
    vlnk.write(&vlnk_data);
    println!("Successfully wrote vlnk file");
    println!("Reboot and check!");
    Ok(())
}

pub fn set_ventoy_boot(drive: char) -> Result<()> {
    let dtypes = [".iso" , ".img", ".vhd", ".vhdx", ".vtoy", ".wim", ".efi"];
    let volume = get_fat_volume(drive)
    .context("Unable to find suitable exfat volume")?;
    let ls = volume.read_dir("/").context("Cannot list content on volume")?;
    for entry in ls {
        if entry.is_err() {
            continue;
        }
        let filename = entry.unwrap().name().to_string().to_lowercase();
        for dtype in dtypes {
            if filename.ends_with(dtype) {
                if volume.remove_file(&filename).is_err() {
                    println!("Warning! A file could not be removed, ventoy mode may fail");
                };
            }
        }
    };
    println!("Deleted all files we could");
    Ok(())
}

#[cfg(windows)]
pub fn ensure_nvram_entry(drive: char) -> anyhow::Result<u16> {
    const DESC: &str = "YoRBo";
    const EFI_PATH: &str = "\\EFI\\BOOT\\grubx64.efi";

    if !drive.is_ascii_alphabetic() {
        anyhow::bail!("invalid drive letter: {drive}");
    }

    let grubenv_path = format!("{}:\\yorbo\\grubenv", drive);
    let mut env = if Path::new(&grubenv_path).exists() {
        GrubEnv::load(&grubenv_path).context("failed to load grubenv")?
    } else {
        GrubEnv::create(&grubenv_path).context("failed to create grubenv")?
    };
    env.set("boot_once", "false")
        .context("failed to set boot_once=false")?;

    // GPT partition GUID, number, start/size (in sectors) of the volume
    let script = format!(
        "$p = Get-Partition -DriveLetter {drive}; \
         $d = Get-Disk -Number $p.DiskNumber; \
         '{{0}}|{{1}}|{{2}}|{{3}}|{{4}}' -f $p.Guid,$p.PartitionNumber,$p.Offset,$p.Size,$d.LogicalSectorSize"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .context("failed to run powershell")?;
    if !out.status.success() {
        anyhow::bail!("powershell failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let f: Vec<&str> = text.trim().split('|').collect();
    if f.len() != 5 {
        anyhow::bail!("unexpected powershell output: {:?}", text.trim());
    }
    let sector: u64 = f[4].parse().context("bad LogicalSectorSize")?;
    let hd = EFIHardDrive {
        partition_number: f[1].parse().context("bad PartitionNumber")?,
        partition_start: f[2].parse::<u64>().context("bad Offset")? / sector,
        partition_size: f[3].parse::<u64>().context("bad Size")? / sector,
        // Guid is empty on MBR disks, so this fails there
        partition_sig: uuid::Uuid::parse_str(f[0].trim_matches(|c| c == '{' || c == '}'))
            .with_context(|| format!("{drive}: is not on a GPT disk (got {:?})", f[0]))?,
        format: 2,
        sig_type: EFIHardDriveType::Gpt,
    };

    let mut manager = efivar::system(); // needs admin

    let vars: Vec<Variable> = manager
        .get_all_vars()
        .context("failed to enumerate EFI variables")?
        .filter(|v| v.boot_var_id().is_some())
        .collect();

    let mut used_ids: Vec<u16> = Vec::new();
    let mut matched: Option<(u16, BootEntry)> = None;
    for var in &vars {
        let Some(id) = var.boot_var_id() else { continue };
        used_ids.push(id);
        if matched.is_some() {
            continue;
        }
        let Ok(entry) = BootEntry::read(&*manager, var) else { continue };
        if let Some(fpl) = &entry.file_path_list {
            if fpl.hard_drive.partition_sig == hd.partition_sig
                && fpl.file_path.path.eq_ignore_ascii_case(EFI_PATH)
            {
                matched = Some((id, entry));
            }
        }
    }

    match matched {
        Some((id, mut entry)) => {
            if entry.description != DESC {
                entry.description = DESC.to_string();
                manager
                    .add_boot_entry(id, entry)
                    .context("failed to rename firmware entry to YoRBo")?;
            }
            Ok(id)
        }
        None => {
            let id = (0u16..=u16::MAX)
                .find(|i| !used_ids.contains(i))
                .context("no free Boot#### slot")?;
            let entry = BootEntry {
                attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
                description: DESC.to_string(),
                file_path_list: Some(FilePathList {
                    file_path: FilePath { path: EFI_PATH.to_string() },
                    hard_drive: hd,
                }),
                optional_data: Vec::new(),
            };
            manager
                .add_boot_entry(id, entry)
                .context("failed to create YoRBo boot entry")?;
            Ok(id)
        }
    }
}

#[cfg(windows)]
pub fn toyboot(drive: char) -> anyhow::Result<()> {
    println!("Ensuring NVRAM entry");
    let id = ensure_nvram_entry(drive)?;

    let grubenv_path = format!("{}:\\yorbo\\grubenv", drive);
    let mut env = GrubEnv::load(&grubenv_path)
        .context("failed to load grubenv after ensure_nvram_entry")?;
    env.set("boot_once", "true")
        .context("failed to set boot_once=true")?;

    // BootNext is a single u16 (little-endian) holding the Boot#### id
    let mut manager = efivar::system();
    manager
        .write(
            &Variable::new("BootNext"),
            VariableFlags::default(),
            &id.to_le_bytes(),
        )
        .context("failed to set BootNext")?;

    Command::new("shutdown")
        .arg("/r")
        .arg("/t")
        .arg("0")
        .status()
        .context("failed to run shutdown /r /t 0")?;

    Ok(())
}