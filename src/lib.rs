use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <div class="container mx-auto p-4">
            <h1 class="text-4xl font-bold mb-4">{"Welcome to My Rust + WASM Blog"}</h1>
            <p class="text-lg mb-4">{"This is a personal website built with Rust, WebAssembly, and Yew."}</p>
            <div class="mt-4">
                <a href="/blog" class="text-blue-500 hover:underline">{"View Blog Posts"}</a>
            </div>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}