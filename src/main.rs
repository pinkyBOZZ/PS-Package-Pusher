#![windows_subsystem = "windows"]

use pspp::{app::App, config, logging::Logger, storage};

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    // elevated firewall helper (DESIGN 9.8): no UI, exits right away
    if let Some(code) = pspp::firewall::run_cli(&args) {
        std::process::exit(code);
    }
    let debug = args.iter().any(|a| a == "--debug");
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let (start_tab, theme) = (arg("--tab"), arg("--theme"));
    let add: Vec<std::path::PathBuf> = args
        .windows(2)
        .filter(|w| w[0] == "--add")
        .map(|w| w[1].clone().into())
        .collect();
    let start = args.iter().any(|a| a == "--start");
    let data_dir = storage::data_dir();
    let log = Logger::new(&data_dir.join("logs"), debug);
    let (cfg, issues) = config::load(&data_dir);
    for i in issues {
        log.warn("app", format!("config: {i:?}"));
    }
    // panic → log with backtrace instead of a silent exit (DESIGN 11)
    let plog = log.clone();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        plog.error("panic", info.to_string())
            .field("backtrace", format!("{bt}").replace('\n', " | "));
    }));

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("PS Package Pusher")
        .with_min_inner_size([880.0, 560.0])
        .with_inner_size([1100.0, 720.0]);
    if let Some(w) = cfg.window {
        viewport = viewport
            .with_inner_size([w.w.max(880.0), w.h.max(560.0)])
            .with_position([w.x, w.y])
            .with_maximized(w.maximized);
    }
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo/png/icon-256.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        persist_window: false,
        centered: cfg.window.is_none(),
        ..Default::default()
    };
    eframe::run_native(
        "PS Package Pusher",
        options,
        Box::new(move |cc| {
            Ok(Box::new(App::new(
                cc,
                cfg,
                data_dir,
                log,
                debug,
                start_tab.as_deref(),
                theme.as_deref(),
                add,
                start,
            )))
        }),
    )
}
