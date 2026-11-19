//! Renders assets/logo/*.svg to PNG (PLAN 0.6). Run: cargo run --example render_icons
//! Output: assets/logo/png/icon-{16,24,32,48,256}.png, logo-512.png

use resvg::{tiny_skia, usvg};

fn render(svg: &str, size: u32, out: &str) {
    let data = std::fs::read(svg).expect("read svg");
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).expect("parse svg");
    let mut pix = tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let s = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(s, s),
        &mut pix.as_mut(),
    );
    pix.save_png(out).expect("save png");
    println!("{out}");
}

fn main() {
    std::fs::create_dir_all("assets/logo/png").expect("mkdir");
    render("assets/logo/icon-16.svg", 16, "assets/logo/png/icon-16.png");
    render("assets/logo/icon-32.svg", 24, "assets/logo/png/icon-24.png");
    render("assets/logo/icon-32.svg", 32, "assets/logo/png/icon-32.png");
    render("assets/logo/icon-32.svg", 48, "assets/logo/png/icon-48.png");
    render("assets/logo/logo.svg", 256, "assets/logo/png/icon-256.png");
    render("assets/logo/logo.svg", 512, "assets/logo/png/logo-512.png");
    // preview sheet: every size upscaled 8x (nearest) is done by the viewer; also 16/32 at 8x here
    render("assets/logo/icon-16.svg", 128, "target/preview-16x8.png");
    render("assets/logo/icon-32.svg", 256, "target/preview-32x8.png");
}
