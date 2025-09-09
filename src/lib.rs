use wasm_bindgen::prelude::*;
use pulldown_cmark::{Parser, Options, html};

// WebAssembly entrypoint
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! 🚀 Welcome to my Rust+WASM blog!", name)
}

// Rust에서 마크다운 -> HTML 변환
#[wasm_bindgen]
pub fn render_markdown(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(md, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
