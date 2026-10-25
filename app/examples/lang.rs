//! Renders the library, the Settings screen and the Light tab in one interface language into PNG files:
//! `cargo run --release --example lang -- <language code> <out dir>` (for example `hu` or `auto`).

use lightcraft_engine::Session;
use lightcraft_ui_egui::headless::Headless;
use lightcraft_ui_egui::{LightcraftApp, Services};
use serde_json::json;
use std::time::Duration;

/// Writes a snapshot as a PNG file.
fn save(image: &egui::ColorImage, path: &std::path::Path) {
    let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    if let Some(img) = image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, rgba) {
        let _ = img.save(path);
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let code = args.next().unwrap_or_else(|| "en".into());
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let _ = std::fs::create_dir_all(&out);
    // the settings file goes to the output folder, so the real one is untouched
    // SAFETY: set before any thread of this program starts.
    unsafe { std::env::set_var("LIGHTCRAFT_MOBILE_SETTINGS_DIR", &out) };
    lightcraft_android::settings::update(|p| p.language = code.clone());
    let session = Session::with_demo().with_fs().with_system_clock();
    let mut app = LightcraftApp::new(session, Services::default());
    lightcraft_android::mobile::install(&mut app);
    let mut host = Headless::new(app, [393.0, 873.0], 2.75);
    host.view.ctx.set_zoom_factor(lightcraft_android::mobile::ZOOM);
    let _ = host.app.run("view.photoGrid", json!({}));
    host.settle(Duration::from_secs(5));
    save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{code}-grid.png")));
    lightcraft_android::mobile::open_settings(&host.view.ctx, true);
    host.settle(Duration::from_secs(2));
    save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{code}-settings.png")));
    lightcraft_android::mobile::open_settings(&host.view.ctx, false);
    let _ = host.app.run("view.detail", json!({}));
    lightcraft_android::mobile::open_edit_tab(&host.view.ctx, "light");
    host.settle(Duration::from_secs(5));
    save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{code}-light.png")));
}
