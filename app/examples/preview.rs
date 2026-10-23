//! Renders the mobile layout headlessly into PNGs (CPU rasteriser), to check it without a device:
//! `cargo run --release --example preview -- <out-dir>`.
use std::time::Duration;

use lightcraft_engine::Session;
use lightcraft_ui_egui::headless::Headless;
use lightcraft_ui_egui::{LightcraftApp, Services};
use serde_json::json;

/// Writes an egui image as a PNG file.
fn save(img: &egui::ColorImage, path: &std::path::Path) {
    let [w, h] = img.size;
    let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    image::save_buffer(path, &bytes, w as u32, h as u32, image::ColorType::Rgba8).expect("write png");
    println!("{}", path.display());
}

/// Renders the welcome screen, library, file browser and edit tabs for a phone and a tablet.
fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "preview".into()));
    std::fs::create_dir_all(&out).expect("out dir");
    // (name, width pt, height pt, ppp): a 1080x2400 phone and a 1600x2560 tablet in landscape
    // a folder with a few photos to import, for the file browser
    let photos = std::env::temp_dir().join("lc-preview-photos");
    let _ = std::fs::remove_dir_all(&photos);
    std::fs::create_dir_all(photos.join("DCIM")).expect("photos dir");
    for i in 0..4u8 {
        let img = image::RgbImage::from_fn(240, 160, |x, y| image::Rgb([(x as u8).wrapping_add(i * 50), y as u8, 128 + i * 20]));
        img.save(photos.join("DCIM").join(format!("IMG_{i:04}.jpg"))).expect("jpg");
    }
    let sample = |x: u32, y: u32| [((x * 7 + y * 13) % 200 + 30) as u16, 110, 150];
    let heic = lightcraft_heif::testdata::build(&lightcraft_heif::testdata::Spec::new(96, 64, &sample));
    std::fs::write(photos.join("DCIM").join("IMG_HEIC.HEIC"), heic).expect("heic");
    println!("heic supported: {}", lightcraft_android::heic_supported());
    std::fs::create_dir_all(photos.join("Pictures")).expect("dir");
    lightcraft_android::mobile::set_platform(lightcraft_android::mobile::Platform {
        insets_px: || [0.0, 72.0, 0.0, 48.0],
        storage_access: || true,
        request_storage_access: || {},
        storage_root: photos.clone(),
    });
    for (name, w, h, ppp) in [("phone", 393.0, 873.0, 2.75), ("tablet", 1280.0, 800.0, 2.0)] {
        // an empty library first: the welcome screen
        let empty = Session::new().with_fs().with_system_clock();
        let mut welcome_app = LightcraftApp::new(empty, Services::default());
        lightcraft_android::mobile::install(&mut welcome_app);
        let mut welcome_host = Headless::new(welcome_app, [w, h], ppp);
        welcome_host.view.ctx.set_zoom_factor(lightcraft_android::mobile::ZOOM);
        welcome_host.settle(Duration::from_secs(5));
        save(&welcome_host.snapshot(Duration::from_secs(10)), &out.join(format!("{name}-welcome.png")));
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
        let _ = host.app.run("view.photoGrid", json!({}));
        lightcraft_android::mobile::open_browser(&host.view.ctx, &photos);
        host.settle(Duration::from_secs(5));
        save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{name}-browser.png")));
        lightcraft_android::mobile::open_browser(&host.view.ctx, &photos.join("DCIM"));
        host.settle(Duration::from_secs(5));
        save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{name}-browser-dcim.png")));
        let before = host.app.session.catalog.len();
        let r = lightcraft_ui_egui::import::start_paths(&mut host.app, vec![photos.join("DCIM").to_string_lossy().into_owned()]);
        println!("start_paths: {r:?}");
        host.settle(Duration::from_secs(20));
        println!("catalog {before} -> {}", host.app.session.catalog.len());
        save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{name}-after-import.png")));
        let _ = host.app.run("view.detail", json!({}));
        for tab in ["light", "color"] {
            lightcraft_android::mobile::open_edit_tab(&host.view.ctx, tab);
            host.settle(Duration::from_secs(5));
            save(&host.snapshot(Duration::from_secs(5)), &out.join(format!("{name}-edit-{tab}.png")));
        }
    }
}
