//! Windows resources: icon and VERSIONINFO from Cargo.toml (DESIGN 13.1).

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        // the .ico is produced by `cargo run --example make_ico`; without it build plain VERSIONINFO
        if std::path::Path::new("assets/icon/app.ico").exists() {
            res.set_icon("assets/icon/app.ico");
        }
        res.set("ProductName", "PS Package Pusher")
            .set("FileDescription", "PS Package Pusher")
            .set("LegalCopyright", "Copyright (c) 2026 pinkyBOZZ (MIT)");
        res.compile().expect("compile Windows resources");
    }
}
