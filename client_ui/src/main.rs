mod app;
mod formatting;
mod search;
mod settings;
mod status;

use app::App;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    sycamore::render(App);
}
