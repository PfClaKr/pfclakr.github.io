use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Run wasm-pack build
    Command::new("wasm-pack")
        .args(&["build", "--target", "web"])
        .status()
        .expect("Failed to run wasm-pack build");

    // Move index.html to pkg folder
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dest_path = project_root.join("pkg/index.html");
    let src_path = project_root.join("src/index.html");

    if !src_path.exists() {
        eprintln!(
            "Error: Source file src/index.html does not exist at path: {}",
            src_path.display()
        );
        std::process::exit(1);
    }

    // Ensure the destination directory exists
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create pkg directory");
    }

    fs::copy(&src_path, &dest_path).expect("Failed to copy index.html to pkg folder");
    println!("cargo:rerun-if-changed=src/index.html");
}
