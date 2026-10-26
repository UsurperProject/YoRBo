#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{thread, time::Duration};
use anyhow::{Context, Ok, Result};
use installer::helpers;

slint::include_modules!(); // generates AppWindow, Step, TaskItem, PartitionOption, TaskState

fn main() -> Result<()> {
    let app = AppWindow::new()?;
    app.set_step(Step::Scanning);
    handle_window(&app);
    start_check(&app);
    handle_flow(&app);
    app.run()?;
    Ok(())
}

fn start_check(app: &AppWindow) {
    let weak = app.as_weak();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        let file = helpers::check_existing_install();
        if file.is_ok() && file.unwrap().is_some() {
             let _ = weak.upgrade_in_event_loop(move |app| app.set_step(Step::Complete));
            //let _ = weak.upgrade_in_event_loop(move |app| app.set_step(Step::InstallReady));
        } else {
            let _ = weak.upgrade_in_event_loop(move |app| app.set_step(Step::InstallReady));
        }
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

    // Minimize
    let w = app.as_weak();
    app.on_window_minimize(move || w.unwrap().window().set_minimized(true));

    // Maximize / restore
    let w = app.as_weak();
    app.on_window_maximize(move || {
        let app = w.unwrap();
        let win = app.window();
        win.set_maximized(!win.is_maximized());
    });

    // Close
    app.on_window_close(|| { slint::quit_event_loop().ok(); });
}

fn handle_flow(app: &AppWindow) {
    let w = app.as_weak();
    app.on_finish(move || {slint::quit_event_loop().ok(); });
    app.on_cancel_confirmed(move || {slint::quit_event_loop().ok(); });
}