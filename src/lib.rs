use wasm_bindgen::prelude::*;
use web_sys::{Document, Element};

#[wasm_bindgen(start)]
pub fn main() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // HTML 직접 생성
    document.body().unwrap().set_inner_html(
        r#"
        <style>
            body { font-family: Arial, sans-serif; text-align: center; }
            h1 { color: #0077cc; }
            p { color: #333; }
        </style>
        <h1>My Rust WASM Blog</h1>
        <p>이 페이지는 Rust + WASM으로 생성되었습니다!</p>
        "#,
    );
}

/// DOM 요소를 생성하는 유틸리티 함수
fn create_element(document: &Document, tag: &str, text: &str) -> Element {
    let element = document.create_element(tag).unwrap();
    element.set_inner_html(text);
    element
}
