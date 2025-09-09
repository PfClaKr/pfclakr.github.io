use wasm_bindgen::prelude::*;

// WebAssembly entrypoint
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! 🚀 Welcome to my Rust+WASM blog!", name)
}