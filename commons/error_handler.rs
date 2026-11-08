use std::process::{ExitCode, Termination};
use anyhow::{Result, Error}; 
use native_dialog::{DialogBuilder, MessageLevel};

pub fn show_err(message: String) {
    let dialog = DialogBuilder::message();
    let _ = dialog.set_title("Critical error")
    .set_level(MessageLevel::Error)
    .set_text(message)
    .alert()
    .show();
}