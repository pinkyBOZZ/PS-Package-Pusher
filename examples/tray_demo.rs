//! Manual smoke check for the tray notifications (src/tray.rs).
//! Run: cargo run --example tray_demo   (clicking a notification prints "clicked")
//! Not part of the release.

use pspp::tray::Tray;
use std::{thread, time::Duration};

fn main() {
    let Some(tray) = Tray::new(|| println!("clicked")) else {
        eprintln!("tray: could not create the tray icon");
        return;
    };
    tray.notify("PS Package Pusher", "Test notification", false);
    thread::sleep(Duration::from_secs(3));
    tray.notify("PS Package Pusher", "Error notification", true);
    thread::sleep(Duration::from_secs(10));
}
