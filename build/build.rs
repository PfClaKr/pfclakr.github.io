use std::env;
use std::fs;
use std::path::Path;

fn main() {
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

    fs::copy(&src_path, &dest_path).expect("Failed to copy index.html to pkg folder");
    println!("cargo:rerun-if-changed=src/index.html");
}
