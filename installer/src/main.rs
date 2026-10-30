#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};

use anyhow::{Result, anyhow};
use installer::helpers::{self, Partition};
use installer::{check_existing_install, fetch_statics, install_bootloader, install_runner};
use slint::{ModelRc, VecModel, Weak};

slint::include_modules!(); // generates AppWindow, Step, TaskItem, PartitionOption, TaskState

const DEFAULT_RUNNER_DIR: &str = "C:/Program Files (x86)/YoRBo";
const BOOTLOADER_PARTITION_MB: u32 = 100;
const MIN_SCAN_TIME: Duration = Duration::from_secs(2);

fn main() -> Result<()> {
    let app = AppWindow::new()?;
    app.set_step(Step::Scanning);
    app.set_runner_path(DEFAULT_RUNNER_DIR.into());

    let candidates = Shared::default();
    handle_window(&app);
    handle_flow(&app, &candidates);
    start_checking_installation(&app);

    app.run()?;
    Ok(())
}


#[derive(Default)]
struct Candidates {
    fat32: Vec<Partition>,
    shrink: Vec<Partition>,
}
type Shared = Arc<Mutex<Candidates>>;

#[derive(Clone, Copy)]
enum Target {
    ExistingFat32,
    NewPartition,
}


fn start_checking_installation(app: &AppWindow) {
    let weak = app.as_weak();
    thread::spawn(move || {
        thread::sleep(MIN_SCAN_TIME);
        let next = match check_existing_install() {
            // TODO: there is no "already installed" screen yet; Complete is a stand-in.
            Ok(Some(_)) => Step::Complete,
            Ok(None) => Step::InstallReady,
            Err(e) => {
                eprintln!("existing-install check failed: {e:#}");
                Step::InstallReady
            }
        };
        ui(&weak, move |a| a.set_step(next));
    });
}


fn handle_window(app: &AppWindow) {
    let w = app.as_weak();
    app.on_window_drag(move |dx, dy| {
        let app = w.unwrap();
        let win = app.window();
        let s = win.scale_factor();
        let p = win.position();
        win.set_position(slint::PhysicalPosition::new(
            p.x + (dx * s) as i32,
            p.y + (dy * s) as i32,
        ));
    });

    let w = app.as_weak();
    app.on_window_minimize(move || w.unwrap().window().set_minimized(true));

    let w = app.as_weak();
    app.on_window_maximize(move || {
        let app = w.unwrap();
        let win = app.window();
        win.set_maximized(!win.is_maximized());
    });

    app.on_window_close(|| {
        slint::quit_event_loop().ok();
    });
}


fn handle_flow(app: &AppWindow, candidates: &Shared) {
    // Ready to install -> Install location
    let w = app.as_weak();
    app.on_install(move || w.unwrap().set_step(Step::RunnerDirectory));

    // Back (Install location and Bootloader screens share this callback)
    let w = app.as_weak();
    app.on_back(move || {
        let app = w.unwrap();
        match app.get_step() {
            Step::RunnerDirectory => app.set_step(Step::InstallReady),
            Step::BootloaderChoice => app.set_step(Step::RunnerDirectory),
            _ => {}
        }
    });

    // Browse… (native folder picker; blocking, so off the UI thread)
    let w = app.as_weak();
    app.on_browse_runner(move || {
        let w = w.clone();
        thread::spawn(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Select YoRBo runner installation directory")
                .pick_folder();
            if let Some(dir) = picked {
                let dir = dir.to_string_lossy().to_string();
                ui(&w, move |a| a.set_runner_path(dir.into()));
            }
        });
    });

    // Install location -> install the runner, then look up partitions
    let w = app.as_weak();
    let cands = candidates.clone();
    app.on_install_runner(move || {
        let app = w.unwrap();
        let dir = PathBuf::from(app.get_runner_path().as_str());
        app.set_runner_progress(0.0);
        app.set_runner_caption("Copying files…".into());
        app.set_step(Step::InstallingRunner);

        let (w, cands) = (w.clone(), cands.clone());
        thread::spawn(move || {
            let installed =
                with_estimated_progress(&w, AppWindow::set_runner_progress, || install_runner(dir));
            if let Err(e) = installed {
                return fail(&w, "Couldn't install the runner", &e, Step::RunnerDirectory);
            }

            ui(&w, |a| a.set_runner_caption("Scanning partitions…".into()));
            match load_candidates() {
                Ok(found) => {
                    let fat32 = options(&found.fat32);
                    let shrink = options(&found.shrink);
                    let has_fat32 = !fat32.is_empty();
                    *cands.lock().unwrap() = found;
                    ui(&w, move |a| {
                        a.set_fat32_partitions(ModelRc::new(VecModel::from(fat32)));
                        a.set_shrink_partitions(ModelRc::new(VecModel::from(shrink)));
                        a.set_fat32_index(0);
                        a.set_shrink_index(0);
                        a.set_use_existing_partition(has_fat32);
                        a.set_step(Step::BootloaderChoice);
                    });
                }
                Err(e) => fail(&w, "Couldn't read your partitions", &e, Step::RunnerDirectory),
            }
        });
    });

    // Bootloader screen: Continue (existing FAT32) / Create partition
    let w = app.as_weak();
    let cands = candidates.clone();
    app.on_continue_existing(move |i| {
        install_bootloader_async(w.clone(), cands.clone(), Target::ExistingFat32, i)
    });
    let w = app.as_weak();
    let cands = candidates.clone();
    app.on_create_partition(move |i| {
        install_bootloader_async(w.clone(), cands.clone(), Target::NewPartition, i)
    });


    app.on_finish(|| {
        slint::quit_event_loop().ok();
    });
    app.on_cancel_confirmed(|| {
        slint::quit_event_loop().ok();
    });
}

/// Bootloader destination -> download statics -> write boot files -> done.
fn install_bootloader_async(weak: Weak<AppWindow>, cands: Shared, target: Target, index: i32) {
    thread::spawn(move || {
        if let Err(e) = run_bootloader_install(&weak, &cands, target, index) {
            fail(&weak, "Couldn't set up the bootloader", &e, Step::BootloaderChoice);
        }
    });
}

fn run_bootloader_install(
    weak: &Weak<AppWindow>,
    cands: &Shared,
    target: Target,
    index: i32,
) -> Result<()> {
    let index = usize::try_from(index).map_err(|_| anyhow!("No partition selected"))?;

    // 1. Decide which drive the boot files go on.
    let drive: char = {
        let found = cands.lock().unwrap();
        match target {
            Target::ExistingFat32 => {
                let part = found.fat32.get(index).ok_or_else(|| anyhow!("No partition selected"))?;
                drive_letter(part)?
            }
            Target::NewPartition => {
                let part = found.shrink.get(index).ok_or_else(|| anyhow!("No volume selected"))?;
                let text = format!("Shrinking {} by 100 MB…", label(part));
                ui(weak, move |a| {
                    a.set_bootloader_path(text.into());
                    a.set_step(Step::PreparingBootloader);
                });
                // Returns the new FAT32 partition; it has no drive letter yet.
                let new_part = part.shrink(BOOTLOADER_PARTITION_MB)?;
                drive_letter(&new_part)?
            }
        }
    };

    ui(weak, |a| {
        a.set_statics_progress(0.0);
        a.set_statics_caption("Downloading…".into());
        a.set_step(Step::DownloadingStatics);
    });
    let archive = with_estimated_progress(weak, AppWindow::set_statics_progress, fetch_statics)?;

    ui(weak, move |a| {
        a.set_bootloader_path(format!("{drive}:\\").into());
        a.set_step(Step::PreparingBootloader);
    });
    install_bootloader(drive, archive)?;

    // There are no post-install hooks in the lib yet, so Step::PostInstall is skipped.
    ui(weak, |a| a.set_step(Step::Complete));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Partition helpers
// ─────────────────────────────────────────────────────────────────────────────

fn load_candidates() -> Result<Candidates> {
    let mut found = Candidates::default();
    for p in helpers::list_vols()? {
        let fs = p.partition_type.as_deref().map(str::to_ascii_uppercase);
        match fs.as_deref() {
            Some("FAT32") => found.fat32.push(p),
            Some("NTFS") if p.mountpoint.is_some() => found.shrink.push(p),
            _ => {}
        }
    }
    // EFI system partition first, so it's the dropdown's default.
    found.fat32.sort_by_key(|p| !is_esp(p));
    Ok(found)
}

fn is_esp(p: &Partition) -> bool {
    p.system_type.as_deref() == Some("System")
}

/// Drive letter of `p`, mounting it first if it has none (the EFI partition and
/// freshly created partitions usually don't).
fn drive_letter(p: &Partition) -> Result<char> {
    if p.mountpoint.is_none() {
        p.ensure_mounted()?; // expected to fill in `p.mountpoint`
    }
    p.mountpoint
        .as_deref()
        .and_then(|m| m.chars().next())
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase())
        .ok_or_else(|| {
            anyhow!("{} has no drive letter, so files can't be written to it.", label(p))
        })
}

fn label(p: &Partition) -> String {
    if is_esp(p) {
        "Windows EFI partition".into()
    } else if let Some(letter) = p.mountpoint.as_deref().and_then(|m| m.chars().next()) {
        format!("{}:\\", letter.to_ascii_uppercase())
    } else {
        format!("Disk {} · Partition {}", p.disk_number, p.partition_number)
    }
}

fn options(parts: &[Partition]) -> Vec<PartitionOption> {
    parts
        .iter()
        .map(|p| {
            let fs = p.partition_type.as_deref().unwrap_or("?").to_ascii_uppercase();
            let detail = if is_esp(p) { "System".to_string() } else { format_size(p.size) };
            PartitionOption { name: label(p).into(), meta: format!("{fs} · {detail}").into() }
        })
        .collect()
}

fn format_size(bytes: u64) -> String {
    const GB: u64 = 1 << 30;
    const MB: u64 = 1 << 20;
    if bytes >= GB { format!("{} GB", bytes / GB) } else { format!("{} MB", bytes / MB) }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI-thread plumbing
// ─────────────────────────────────────────────────────────────────────────────

/// Run `f` on the UI thread. Slint windows may only be touched from there, so
/// worker threads go through this.
fn ui(weak: &Weak<AppWindow>, f: impl FnOnce(&AppWindow) + Send + 'static) {
    let _ = weak.upgrade_in_event_loop(move |app| f(&app));
}

/// Show the error in a native message box, then return to an earlier screen.
fn fail(weak: &Weak<AppWindow>, what: &str, err: &anyhow::Error, back_to: Step) {
    eprintln!("{what}: {err:#}");
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("YoRBo Setup")
        .set_description(format!("{what}.\n\n{err:#}"))
        .show();
    ui(weak, move |a| a.set_step(back_to));
}


fn with_estimated_progress<T>(
    weak: &Weak<AppWindow>,
    set: fn(&AppWindow, f32),
    work: impl FnOnce() -> T,
) -> T {
    let done = Arc::new(AtomicBool::new(false));
    let ticker = {
        let (weak, done) = (weak.clone(), done.clone());
        thread::spawn(move || {
            let mut p = 0.0_f32;
            while !done.load(Ordering::Relaxed) {
                p += (0.9 - p) * 0.04;
                ui(&weak, move |a| set(a, p));
                thread::sleep(Duration::from_millis(100));
            }
        })
    };
    let out = work();
    done.store(true, Ordering::Relaxed);
    let _ = ticker.join();
    ui(weak, move |a| set(a, 1.0));
    out
}