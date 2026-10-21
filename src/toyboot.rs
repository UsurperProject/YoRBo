use anyhow::{Context, Result, anyhow};
use hadris::fat::exfat::sync::ExFatFs;
use hadris::fs::sync::Volume;
use hadris::fs::OpenOptions;
use hadris::host::FileDevice;
use hadris::io::sync::Write;
use hadris::part::{self, Partition};
use hadris::sync::{open, AnyFs};
use crate::partitions::PartitionInfo;
use std::path::{PathBuf, Path};
use crate::vlnk::create_vlnk;
use hadris::fs::MountOptions;
use std::process::Command;
use std::{any, fs};
use grubenv::GrubEnv;
use efivar::boot::{
    BootEntry, BootEntryAttributes, BootVarWriter, EFIHardDrive, EFIHardDriveType, FilePath,
    FilePathList,
};
use efivar::efi::{Variable, VariableFlags};





type FatVolume = Volume<hadris::fat::exfat::sync::ExFatFs<hadris::storage::Partition<FileDevice>>>;

fn get_fat_volume(partition: &PartitionInfo) -> Result<FatVolume> {
    if partition.mountpoint.is_none() {
        return Err(anyhow!("Partition not mounted"));
    }
    let img_path: PathBuf = PathBuf::from(format!("{}:/ventoy/caged_vtoy.img", partition.mountpoint.as_ref().unwrap())); 
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

pub fn set_iso_boot(partition: &PartitionInfo, image: PathBuf) -> Result<()>{
    let volume = get_fat_volume(&partition)
    .context("Unable to find suitable exfat volume")?;
    let filename = format!("yorbo_fast.vlnk.{}", image.extension().unwrap().to_str().unwrap().to_lowercase());
    set_ventoy_boot(&partition).context("Failed to delete stuff")?;
    let mut vlnk = volume.open(filename,  OpenOptions::new().write().create().truncate())?;
    println!("file creation fails ?");
    let mut vlnk_data = create_vlnk(image).context("vlnk creation failed")?;
    vlnk.write(&vlnk_data);
    println!("Successfully wrote vlnk file");
    println!("Reboot and check!");
    Ok(())
}

pub fn set_ventoy_boot(partition: &PartitionInfo) -> Result<()> {
    let dtypes = [".iso" , ".img", ".vhd", ".vhdx", ".vtoy", ".wim", ".efi"];
    let volume = get_fat_volume(&partition)
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

pub fn ensure_nvram_entry(partition: &PartitionInfo) -> Result<u16> {
    const DESC: &str = "YoRBo";
    const EFI_PATH: &str = "/EFI/BOOT/grubx64.efi";
    let Some(mountpoint) = partition.mountpoint.as_ref() else {
        return Err(anyhow!("partition not mounted"));
    };
    #[cfg(windows)]
    let grubenv_path = format!("{}:/yorbo/grubenv", mountpoint);
    #[cfg(linux)]
    let grubenv_path = format!("{}/yorbo/grubenv", mount_point );
    
    let mut env = if Path::new(&grubenv_path).exists() {
        GrubEnv::load(&grubenv_path).context("failed to load grubenv")?
    } else {
        GrubEnv::create(&grubenv_path).context("failed to create grubenv")?
    };
    env.set("boot_once", "false")
        .context("failed to set boot_once=false")?;

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
            if fpl.hard_drive.partition_sig == partition.guid
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
            println!("found nvram entry");
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
                    hard_drive: EFIHardDrive{
                        partition_number: partition.partition_number,
                        partition_start: partition.offset,
                        partition_size: partition.size,
                        partition_sig: partition.guid,
                        format: 2,
                        sig_type: EFIHardDriveType::Gpt
                    }
                }),
                optional_data: Vec::new(),
            };
            println!("doesnt exist, creating");
            manager
                .add_boot_entry(id, entry)
                .context("failed to create YoRBo boot entry")?;
            Ok(id)
        }
    }
}


pub fn toyboot(partition: &PartitionInfo) -> Result<()> {
    let Some(mountpoint) = partition.mountpoint.as_ref() else {
        return Err(anyhow!("partition not mounted"));
    };
    println!("Ensuring NVRAM entry");
    let id = ensure_nvram_entry(partition)?;

    #[cfg(windows)]
    let grubenv_path = format!("{}:\\yorbo\\grubenv", mountpoint);
    
    #[cfg(linux)]
    let grubenv_path = format!("{}/yorbo/grubenv", mountpoint);
    
    
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
    panic!("");
    #[cfg(windows)]
    Command::new("shutdown")
        .arg("/r")
        .arg("/t")
        .arg("0")
        .status()
        .context("failed to run shutdown /r /t 0")?;

    #[cfg(linux)]
    Command::new("reboot")
        .status()
        .context("failed to run reboot")?;
    Ok(())
}