use anyhow::{Context, Ok, Result};

slint::include_modules!(); // generates AppWindow, Step, TaskItem, PartitionOption, TaskState


fn main() -> Result<()> {
    let app = AppWindow::new()?;
    app.set_step(Step::Scanning);
    
    app.run()?;
    Ok(())
}
