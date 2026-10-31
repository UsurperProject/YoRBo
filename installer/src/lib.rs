pub mod helpers;
use std::{fs::{self, File}, io::{self, BufReader, Read, Write}, path::{Path, PathBuf}};
use anyhow::{Ok, Result, anyhow};
use helpers::{ProgressWriter, free_space, powershell};
use zip::ZipArchive;


const STATICS_URL: &str = "https://github.com/UsurperProject/YoRBo-statics/archive/refs/heads/main.zip";

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
            return Ok(Some(drive));
        }
    }
    return Ok(None);
}

pub fn install_runner(directory: &mut PathBuf, runner: &[u8]) -> Result<()> {
    let is_dir_yorbo = directory.file_name()
    .and_then(|name| name.to_str())
    .map(|s| s.to_lowercase())
    .map_or(false, |s| s == "yorbo");

    if !is_dir_yorbo {
        directory.push("YoRBo");
    }
    fs::create_dir_all(&*directory)?;
    let runner_exe_path = directory.join("yorbo.exe");
    Ok(fs::write(runner_exe_path, runner)?)
}

///   zip/yorbo/**                -> drive:/yorbo/**
///   zip/grub.cfg                -> drive:/yorbo/grub.cfg
///   zip/caged_vtoy.img          -> drive:/ventoy/caged_vtoy.img
///   zip/grubx64_unmodified.efi  -> patch_grub() -> drive:/EFI/BOOT/grubx64.efi
pub fn install_bootloader(drive: char, bootloader: PathBuf) -> Result<()> {
    let root = PathBuf::from(format!("{drive}:\\"));
    let mut zip = ZipArchive::new(BufReader::new(File::open(&bootloader)?))?;

    // Decide what goes where before touching the drive.
    let mut plan: Vec<(usize, PathBuf)> = Vec::new();
    let mut grub_index = None;
    let mut needed = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index_raw(i)?;
        if !entry.is_file() {
            continue;
        }
        // GitHub wraps the repo in a "YoRBo-statics-main/" folder; drop it.
        let Some(rel) = entry.enclosed_name().map(|n| n.components().skip(1).collect::<PathBuf>()) else {
            continue;
        };
        if rel == Path::new("grubx64_unmodified.efi") {
            grub_index = Some(i);
        } else if let Some(dest) = destination(&rel) {
            needed += entry.size();
            plan.push((i, root.join(dest)));
        }
    }
    let grub_index = grub_index.ok_or_else(|| anyhow!("the statics archive has no grubx64_unmodified.efi"))?;
    if !plan.iter().any(|(_, d)| d.ends_with("caged_vtoy.img")) {
        return Err(anyhow!("the statics archive has no caged_vtoy.img"));
    }

    // Patch grub first: if that fails we haven't written anything yet.
    let mut grub = Vec::new();
    zip.by_index(grub_index)?.read_to_end(&mut grub)?;
    patch_grub(&mut grub)?;
    needed += grub.len() as u64;

    let free = free_space(drive)?;
    if free < needed {
        return Err(anyhow!(
            "{drive}: only has {} MB free, but YoRBo needs {} MB. Pick another partition.",
            free >> 20,
            needed.div_ceil(1 << 20)
        ));
    }

    for (i, dest) in plan {
        fs::create_dir_all(dest.parent().unwrap())?;
        io::copy(&mut zip.by_index(i)?, &mut File::create(&dest)?)?;
    }

    // Keep whatever grubx64.efi was already there (e.g. a Linux install's) once.
    let grub_dest = root.join("EFI/BOOT/grubx64.efi");
    fs::create_dir_all(grub_dest.parent().unwrap())?;
    let backup = grub_dest.with_extension("efi.yorbo-backup");
    if grub_dest.exists() && !backup.exists() {
        fs::copy(&grub_dest, &backup)?;
    }
    fs::write(&grub_dest, &grub)?;

    let _ = fs::remove_file(&bootloader);
    Ok(())
}

fn destination(rel: &Path) -> Option<PathBuf> {
    if rel == Path::new("grub.cfg") {
        Some(PathBuf::from("yorbo/grub.cfg"))
    } else if rel == Path::new("caged_vtoy.img") {
        Some(PathBuf::from("ventoy/caged_vtoy.img"))
    } else if rel.starts_with("yorbo") {
        Some(rel.to_path_buf())
    } else {
        None
    }
}
pub fn fetch_statics(mut progress: impl FnMut(u64, Option<u64>)) -> Result<PathBuf> {
    let path = std::env::temp_dir().join("yorbo-statics.zip");
    let mut response = ureq::get(STATICS_URL).call()?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    let mut reader = response.body_mut().as_reader();
    let mut writer = ProgressWriter::new(File::create(&path)?, |done| progress(done, total));
    io::copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(path)
}

pub fn patch_grub(data: &mut [u8]) -> Result<()> {
    let new_prefix: &[u8] = b"/yorbo";
    // (type 3, size 24) + b'(,2)/grub\0'
    // 24 = 8-byte header + 16-byte payload
    let mut rec = Vec::new();
    rec.extend_from_slice(&3u32.to_le_bytes());
    rec.extend_from_slice(&24u32.to_le_bytes());
    rec.extend_from_slice(b"(,2)/grub\0");

    // Count occurrences
    let count = data.windows(rec.len()).filter(|w| *w == rec.as_slice()).count();
    if count != 1 {
        return Err(anyhow!("wrong build ?"));
    }

    // Find index
    let i = data.windows(rec.len()).position(|w| w == rec.as_slice()).unwrap();

    // 16-byte null-padded buffer
    let mut padded_new = [0u8; 16];
    let len = new_prefix.len().min(16);
    padded_new[..len].copy_from_slice(&new_prefix[..len]);

    let start = i + 8;
    data[start..start + 16].copy_from_slice(&padded_new);

    Ok(())
}
