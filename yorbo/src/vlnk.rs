use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;


pub fn create_vlnk(image: impl AsRef<Path>) -> Result<Vec<u8>> {
    const VLNK_FILE_LEN: usize = 32768;
    const VLNK_STRUCT_LEN: usize = 512; // 16 guid + 4 crc + 4 sig + 8 offset + 384 path + 96 reserved
    const NAME_MAX: usize = 384;
    const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x0056_0000;
    let err = |m: &str| Error::new(ErrorKind::Other, m.to_string());

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct DiskExtent { disk_number: u32, starting_offset: i64, extent_length: i64 }
    #[repr(C)]
    #[derive(Default)]
    struct VolumeDiskExtents { count: u32, extents: [DiskExtent; 1] }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn DeviceIoControl(h: *mut c_void, code: u32, inb: *const c_void, insz: u32,
            outb: *mut c_void, outsz: u32, ret: *mut u32, ov: *mut c_void) -> i32;
        fn GetVolumeInformationW(root: *const u16, vol: *mut u16, vsz: u32, serial: *mut u32,
            maxcomp: *mut u32, flags: *mut u32, fs: *mut u16, fssz: u32) -> i32;
    }

    fn crc32c(buf: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in buf {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0x82F6_3B78 & (!(crc & 1)).wrapping_add(1));
            }
        }
        !crc
    }

    let abs = std::path::absolute(image.as_ref())?;
    if !abs.is_file() {
        return Err(err("file does not exist"));
    }
    let full = abs.to_string_lossy().into_owned();
    let b = full.as_bytes();
    if b.len() < 5 || !b[0].is_ascii_alphabetic() || b[1] != b':' {
        return Err(err("unsupported path: need a local drive-letter path like D:\\x.iso"));
    }
    let drive = (b[0] as char).to_ascii_uppercase();

    let ext = abs.extension().map(|e| format!(".{}", e.to_string_lossy().to_ascii_lowercase()));
    match ext.as_deref() {
        Some(".iso" | ".img" | ".wim" | ".efi" | ".vhd" | ".vhdx" | ".dat" | ".vtoy") => {}
        _ => return Err(err("unsupported file suffix")),
    }

    let root: Vec<u16> = format!("{drive}:\\").encode_utf16().chain([0]).collect();
    let mut fsname = [0u16; 64];
    let ok = unsafe {
        GetVolumeInformationW(root.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut(),
            std::ptr::null_mut(), std::ptr::null_mut(), fsname.as_mut_ptr(), 63)
    };
    if ok == 0 {
        return Err(Error::last_os_error());
    }
    let n = fsname.iter().position(|&c| c == 0).unwrap_or(0);
    let fsname = String::from_utf16_lossy(&fsname[..n]).to_ascii_uppercase();
    if !matches!(fsname.as_str(), "NTFS" | "EXFAT" | "FAT" | "FAT32" | "FAT16" | "FAT12" | "UDF") {
        return Err(err("unsupported file system"));
    }

    // --- stored path: UTF-8, drive letter stripped, '\' -> '/' ---
    let rel = full[2..].replace('\\', "/");
    if rel.len() >= NAME_MAX {
        return Err(err("file path too long for vlnk (max 383 bytes)"));
    }

    let open = |p: String| OpenOptions::new().read(true).share_mode(3).open(p);
    let vol = open(format!("\\\\.\\{drive}:"))?;
    let mut ext = VolumeDiskExtents::default();
    let mut got = 0u32;
    let r = unsafe {
        DeviceIoControl(vol.as_raw_handle() as *mut c_void, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
            std::ptr::null(), 0, &mut ext as *mut _ as *mut c_void,
            std::mem::size_of::<VolumeDiskExtents>() as u32, &mut got, std::ptr::null_mut())
    };
    if r == 0 || ext.count == 0 {
        return Err(Error::last_os_error());
    }
    drop(vol);
    let part_offset = ext.extents[0].starting_offset as u64;

    let mut disk = open(format!("\\\\.\\PhysicalDrive{}", ext.extents[0].disk_number))?;
    let mut sector = [0u8; 512];
    disk.read_exact(&mut sector)?;
    let sig = &sector[0x1B8..0x1BC];

    let mut out = vec![0u8; VLNK_FILE_LEN];
    out[0..16].copy_from_slice(b"  www.ventoy.net"); 
    out[20..24].copy_from_slice(sig);
    out[24..32].copy_from_slice(&part_offset.to_le_bytes());
    out[32..32 + rel.len()].copy_from_slice(rel.as_bytes());
    let crc = crc32c(&out[..VLNK_STRUCT_LEN]); 
    out[16..20].copy_from_slice(&crc.to_le_bytes());
    Ok(out)
}