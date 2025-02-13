use yew::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[function_component(App)]
fn app() -> Html {
    html! {
        <div class="container mx-auto p-4">
            <h1 class="text-4xl font-bold mb-4">{"Very First Homa Page"}</h1>
            <p class="text-lg mb-4">{"Can i say \"Hello world\" to world?"}</p>
            <div class="mt-4">
                <a href="/blog" class="text-blue-500 hover:underline">{"View Blog Posts"}</a>
            </div>
        </div>
    }
}

#[wasm_bindgen(start)]
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}