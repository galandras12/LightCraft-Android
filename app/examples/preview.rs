//! Renders the mobile layout headlessly into PNGs (CPU rasteriser), to check it without a device:
//! `cargo run --release --example preview -- <out-dir>`.
use std::time::Duration;

use lightcraft_engine::Session;
use lightcraft_ui_egui::headless::Headless;
use lightcraft_ui_egui::{LightcraftApp, Services};
use serde_json::json;

fn save(img: &egui::ColorImage, path: &std::path::Path) {
    let [w, h] = img.size;
    let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    image::save_buffer(path, &bytes, w as u32, h as u32, image::ColorType::Rgba8).expect("write png");
    println!("{}", path.display());
}

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "preview".into()));
    std::fs::create_dir_all(&out).expect("out dir");
    // (name, width pt, height pt, ppp): a 1080x2400 phone and a 1600x2560 tablet in landscape
    for (name, w, h, ppp) in [("phone", 393.0, 873.0, 2.75), ("tablet", 1280.0, 800.0, 2.0)] {
        let session = Session::with_demo().with_fs().with_system_clock();
        let mut app = LightcraftApp::new(session, Services::default());
        lightcraft_android::mobile::install(&mut app);
        let zoom = lightcraft_android::mobile::ZOOM;
        let mut host = Headless::new(app, [w, h], ppp);
        host.view.ctx.set_zoom_factor(zoom);
        let _ = host.app.run("view.photoGrid", json!({}));
        host.settle(Duration::from_secs(5));
        save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{name}-grid.png")));
        let _ = host.app.run("view.detail", json!({}));
        host.settle(Duration::from_secs(5));
        save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{name}-detail.png")));
        for tab in ["light", "color"] {
            lightcraft_android::mobile::open_edit_tab(&host.view.ctx, tab);
            host.settle(Duration::from_secs(5));
            save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{name}-edit-{tab}.png")));
        }
    }
}
