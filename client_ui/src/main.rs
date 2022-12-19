mod app;
mod search;
mod settings;
mod status;

use app::App;

// #[cfg(all(not(debug_assertions), not(feature = "ssg")))]
// fn main() {
//     sycamore::hydrate(App);
// }

// #[cfg(all(debug_assertions, not(feature = "ssg")))]
fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    sycamore::render(App);
}

#[cfg(feature = "ssg")]
fn main() {
    let out_dir = std::env::args().nth(1).unwrap();

    println!("out_dir {}", out_dir);

    let template = std::fs::read_to_string(format!("{}/index.html", out_dir)).unwrap();

    let html = sycamore::render_to_string(App);

    let html = template.replace("<!--app-html-->\n", &html);

    let path = format!("{}/index.html", out_dir);

    println!("Writing html to file \"{}\"", path);
    std::fs::write(path, html).unwrap();
}
