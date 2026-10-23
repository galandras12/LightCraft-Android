//! Imports the raw files of a folder through the app's import path and reports how each opened
//! (decoded sensor data or embedded preview): `cargo run --release --example cr3check -- <dir> <out.png>`.
use std::time::{Duration, Instant};

use lightcraft_engine::Session;
use lightcraft_ui_egui::headless::Headless;
use lightcraft_ui_egui::{LightcraftApp, Services};
use serde_json::json;

/// Imports every file of the folder and prints how each one opened; saves a snapshot of the first photo.
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
            if let Some(d) = host.app.session.develop_of(*id) {
                println!(
                    "  exposure {:+.2} contrast {:+.0} highlights {:+.0} shadows {:+.0} whites {:+.0} blacks {:+.0} vibrance {:+.0}",
                    d.light.exposure, d.light.contrast, d.light.highlights, d.light.shadows, d.light.whites, d.light.blacks, d.color.vibrance
                );
            }
        }
        // LENS_XML=<lensfun file>: load it, list the lenses and give the photo the first one
        if let Ok(xml) = std::env::var("LENS_XML") {
            println!("load: {:?}", host.app.run("optics.loadLensProfiles", json!({"path": xml})));
            let list = host.app.run("optics.lensProfiles", json!({"filter": "test"}));
            println!("list: {list:?}");
            let first = list.ok().and_then(|l| l["lenses"][0]["id"].as_u64());
            if let Some(lens) = first {
                println!("set: {:?}", host.app.run("optics.lensProfile", json!({"id": lens})));
                if let Some(photo) = host.app.session.catalog.photo(*id) {
                    println!("embedded lens: warp={} vignette={} stages={} id={}", photo.embedded_lens.is_some_and(|l| l.warp.is_some()), photo.embedded_lens.is_some_and(|l| l.vignette.is_some()), photo.embedded_lens.is_some_and(|l| l.stages.iter().any(Option::is_some)), photo.embedded_lens.map_or(0, |l| l.profile_id));
                    println!("optics: {:?}", photo.develop.optics);
                }
                println!("now: {:?}", host.app.run("optics.lensProfiles", json!({"limit": 1})).map(|l| l["current"].clone()));
            }
        }
        // NR=<0..100>: luminance and colour noise reduction at that strength
        if let Some(nr) = std::env::var("NR").ok().and_then(|v| v.parse::<f64>().ok()) {
            for control in ["detail.nrLuminance", "detail.nrColor"] {
                let _ = host.app.run("develop.set", json!({"control": control, "value": nr}));
            }
        }
        // EXPORT=<file.png>: the full-size render, to look at the noise at 100 %
        if let Ok(path) = std::env::var("EXPORT") {
            let t = Instant::now();
            let options = lightcraft_engine::export::ExportOptions::from_json(&json!({"format": "png"}));
            match lightcraft_engine::export::export_photo(&mut host.app.session, *id, &options, 1) {
                Ok(out) => {
                    let _ = std::fs::write(&path, &out.bytes);
                    println!("exported {path}: {}x{} in {:.1}s", out.width, out.height, t.elapsed().as_secs_f64());
                }
                Err(e) => println!("export failed: {e}"),
            }
        }
        host.settle(Duration::from_secs(60));
        let img = host.snapshot(Duration::from_secs(30));
        let [w, h] = img.size;
        let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
        image::save_buffer(&out, &bytes, w as u32, h as u32, image::ColorType::Rgba8).expect("png");
        println!("{out}");
    }
}
