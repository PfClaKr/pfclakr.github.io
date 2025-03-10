use wasm_bindgen::prelude::*;
use web_sys::{Document, Element};

#[wasm_bindgen(start)]
pub fn main() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    let body = document.body().expect("Document should have a body");

    let title = create_element(&document, "h1", "My Rust WASM Blog");
    body.append_child(&title).unwrap();

    let posts = vec![
        ("첫 번째 포스트", "이것은 첫 번째 글입니다."),
        ("두 번째 포스트", "두 번째 글의 내용입니다."),
    ];

    for (title, content) in posts {
        let post_title = create_element(&document, "h2", title);
        let post_content = create_element(&document, "p", content);

        body.append_child(&post_title).unwrap();
        body.append_child(&post_content).unwrap();
    }
}

/// DOM 요소를 생성하는 유틸리티 함수
fn create_element(document: &Document, tag: &str, text: &str) -> Element {
    let element = document.create_element(tag).unwrap();
    element.set_inner_html(text);
    element
}
