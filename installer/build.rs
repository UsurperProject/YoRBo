use embed_manifest::{embed_manifest, manifest::ExecutionLevel, new_manifest};

fn main() {
    slint_build::compile("ui/app-window.slint").unwrap();
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
    let manifest = new_manifest("Yorbo.yorbo")
        .requested_execution_level(ExecutionLevel::RequireAdministrator);

    embed_manifest(manifest).expect("failed to embed Windows manifest");

        // Icon: Explorer, taskbar, shortcuts, and the right-click menu entry all use it.
        winresource::WindowsResource::new()
            .set_icon("../icons/yorbo_logo.ico")
            .compile()
            .expect("failed to embed the icon (is icons/yorbo_logo.ico there?)");

    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../icons/yorbo_logo.ico");
}