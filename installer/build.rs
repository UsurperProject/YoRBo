use embed_manifest::{embed_manifest, manifest::ExecutionLevel, new_manifest};

fn main() {
    slint_build::compile("ui/app-window.slint").unwrap();
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
    let manifest = new_manifest("Yorbo.yorbo")
        .requested_execution_level(ExecutionLevel::RequireAdministrator);

    embed_manifest(manifest).expect("failed to embed Windows manifest");
    }

    println!("cargo:rerun-if-changed=build.rs");
}