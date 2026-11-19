//! Packs the logo PNG renders into assets/icon/app.ico (PLAN 5.3).
//! Run after `render_icons`: cargo run --example make_ico

use image::codecs::ico::{IcoEncoder, IcoFrame};

fn main() {
    let sizes = [
        ("icon-16.png", 16),
        ("icon-24.png", 24),
        ("icon-32.png", 32),
        ("icon-48.png", 48),
        ("icon-256.png", 256),
    ];
    let frames: Vec<IcoFrame> = sizes
        .iter()
        .map(|(f, s)| {
            let img = image::open(format!("assets/logo/png/{f}"))
                .expect("png")
                .to_rgba8();
            assert_eq!(img.width(), *s, "{f}");
            IcoFrame::as_png(img.as_raw(), *s, *s, image::ExtendedColorType::Rgba8).expect("frame")
        })
        .collect();
    std::fs::create_dir_all("assets/icon").expect("dir");
    let out = std::fs::File::create("assets/icon/app.ico").expect("create");
    IcoEncoder::new(out).encode_images(&frames).expect("encode");
    println!("assets/icon/app.ico ({} sizes)", frames.len());
}
