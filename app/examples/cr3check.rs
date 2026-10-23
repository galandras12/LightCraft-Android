//! Imports the raw files of a folder through the app's import path and reports how each opened
//! (decoded sensor data or embedded preview): `cargo run --release --example cr3check -- <dir> <out.png>`.
use std::time::{Duration, Instant};

use lightcraft_engine::Session;
use lightcraft_ui_egui::headless::Headless;
use lightcraft_ui_egui::{LightcraftApp, Services};
use serde_json::json;

fn main() {
    let dir = std::env::args().nth(1).expect("dir");
    let out = std::env::args().nth(2).unwrap_or_else(|| "cr3.png".into());
    let session = Session::new().with_fs().with_system_clock();
    let mut app = LightcraftApp::new(session, Services::default());
    lightcraft_android::mobile::install(&mut app);
    let mut host = Headless::new(app, [393.0, 873.0], 2.75);
    host.view.ctx.set_zoom_factor(lightcraft_android::mobile::ZOOM);
    host.settle(Duration::from_secs(5));
    let t = Instant::now();
    let r = lightcraft_ui_egui::import::start_paths(&mut host.app, vec![dir]);
    println!("start_paths: {r:?}");
    host.settle(Duration::from_secs(120));
    println!("import took {:.1}s", t.elapsed().as_secs_f64());
    let ids: Vec<_> = host.app.session.catalog.photos().map(|p| p.id).collect();
    for p in host.app.session.catalog.photos() {
        println!("{} {}x{} format={} preview_only={:?}", p.file_name, p.width, p.height, p.format, p.preview_only);
    }
    if let Some(id) = ids.first() {
        let _ = host.app.run("library.select", json!({"ids": [id.0], "active": id.0}));
        let _ = host.app.run("view.detail", json!({}));
        if std::env::var_os("AUTO").is_some() {
            println!("auto: {:?}", host.app.run("develop.auto", json!({})).map(|_| ()));
        }
        host.settle(Duration::from_secs(60));
        let img = host.snapshot(Duration::from_secs(30));
        let [w, h] = img.size;
        let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
        image::save_buffer(&out, &bytes, w as u32, h as u32, image::ColorType::Rgba8).expect("png");
        println!("{out}");
    }
}
