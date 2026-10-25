//! Draws the watermark variants onto a sample picture and renders the Settings screen with the watermark section,
//! into PNG files: `cargo run --release --example wm -- <out dir>`.

use lightcraft_android::settings::{self, WmPrefs};
use lightcraft_engine::Session;
use lightcraft_ui_egui::headless::Headless;
use lightcraft_ui_egui::{LightcraftApp, Services};
use std::time::Duration;

/// Writes a snapshot as a PNG file.
fn save_color(image: &egui::ColorImage, path: &std::path::Path) {
    let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    if let Some(img) = image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, rgba) {
        let _ = img.save(path);
    }
}

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let _ = std::fs::create_dir_all(&out);
    // SAFETY: set before any thread of this program starts.
    unsafe { std::env::set_var("LIGHTCRAFT_MOBILE_SETTINGS_DIR", &out) };
    settings::register_fonts();
    // a logo with a transparent background
    let logo = out.join("logo.png");
    let img = image::RgbaImage::from_fn(200, 120, |x, y| {
        let (dx, dy) = (x as f32 - 100.0, y as f32 - 60.0);
        if dx * dx / 9000.0 + dy * dy / 3000.0 < 1.0 { image::Rgba([240, 90, 40, 230]) } else { image::Rgba([0, 0, 0, 0]) }
    });
    let _ = img.save(&logo);
    let variants: Vec<(&str, WmPrefs)> = vec![
        ("corner", WmPrefs { enabled: true, text: "© Jane Doe 2026".into(), ..Default::default() }),
        ("mosaic-diagonal", WmPrefs { enabled: true, text: "SAMPLE".into(), tile: true, direction: "diagonalUp".into(), opacity: 0.35, size: 0.06, font: "heavy".into(), ..Default::default() }),
        ("vertical-left", WmPrefs { enabled: true, text: "Jane Doe".into(), position: "bottomLeft".into(), direction: "vertical".into(), font: "serif-italic".into(), color: [250, 200, 40], ..Default::default() }),
        ("mono-top", WmPrefs { enabled: true, text: "IMG_0001 · f/2.8".into(), position: "top".into(), font: "mono".into(), size: 0.04, ..Default::default() }),
        ("logo-mosaic", WmPrefs { enabled: true, use_image: true, image: logo.to_string_lossy().into_owned(), tile: true, direction: "diagonalDown".into(), image_width: 0.14, opacity: 0.5, ..Default::default() }),
        ("logo-corner", WmPrefs { enabled: true, use_image: true, image: logo.to_string_lossy().into_owned(), position: "topRight".into(), image_width: 0.2, ..Default::default() }),
    ];
    for (name, prefs) in &variants {
        let (w, h) = (900usize, 600usize);
        let mut img = lightcraft_raster::Rgba8::from_fn(w, h, |x, y| {
            let t = y as f32 / h as f32;
            let hill = 0.62 + 0.08 * ((x as f32 / 90.0).sin() + 0.5 * (x as f32 / 31.0).sin());
            if t > hill { [28, 44 + (t * 60.0) as u8, 36, 255] } else { [(110.0 + 120.0 * t) as u8, (150.0 + 60.0 * t) as u8, (210.0 - 40.0 * t) as u8, 255] }
        });
        if let Some(json) = settings::watermark_json(prefs) {
            match serde_json::from_value::<lightcraft_engine::export::Watermark>(json) {
                Ok(mark) => lightcraft_engine::export::draw_watermark(&mut img, &mark),
                Err(e) => eprintln!("{name}: {e}"),
            }
        }
        let pixels: Vec<u8> = img.data.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
        if let Some(png) = image::RgbaImage::from_raw(w as u32, h as u32, pixels) {
            let _ = png.save(out.join(format!("wm-{name}.png")));
        }
    }
    // the Settings screen, tall enough to show the watermark section
    settings::update(|p| p.watermark = WmPrefs { enabled: true, tile: true, direction: "diagonalUp".into(), text: "SAMPLE".into(), ..Default::default() });
    let session = Session::with_demo().with_fs().with_system_clock();
    let mut app = LightcraftApp::new(session, Services::default());
    lightcraft_android::mobile::install(&mut app);
    let mut host = Headless::new(app, [393.0, 1500.0], 2.0);
    host.view.ctx.set_zoom_factor(lightcraft_android::mobile::ZOOM);
    lightcraft_android::mobile::open_settings(&host.view.ctx, true);
    host.settle(Duration::from_secs(3));
    save_color(&host.snapshot(Duration::from_secs(5)), &out.join("wm-settings.png"));
}
