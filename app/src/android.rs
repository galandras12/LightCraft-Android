//! NativeActivity entry point.
//!
//! The library lives in the app's private storage; the first start seeds the procedural demo
//! photos. File import (Storage Access Framework) is not wired yet.

use android_activity::AndroidApp;
use lightcraft_engine::Session;
use lightcraft_ui_egui::{LightcraftApp, Services};

struct App(LightcraftApp);

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.0.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("LightCraft"));
    let data = app.internal_data_path();
    if let Some(dir) = &data {
        // the engine's config / cache lookups start from $HOME
        // SAFETY: set once at start-up, before the app starts any thread of its own.
        unsafe { std::env::set_var("HOME", dir) };
    }
    let library = data.map(|d| d.join("library"));

    let options = eframe::NativeOptions { renderer: eframe::Renderer::Wgpu, android_app: Some(app), ..Default::default() };
    let result = eframe::run_native(
        "LightCraft",
        options,
        Box::new(move |cc| {
            let mut session = Session::new().with_fs().with_default_denoise_models().with_system_clock();
            let opened = library.as_deref().map(|dir| session.open_library(dir, true).map(|_| ()));
            if let Some(Err(e)) = opened {
                log::error!("library: {e}");
                session = Session::with_demo().with_fs().with_system_clock();
            }
            // the "can't lock the library" notice is for desktops with two programs on one library
            for w in session.take_library_warnings() {
                log::warn!("library: {w}");
            }
            cc.egui_ctx.set_zoom_factor(crate::mobile::ZOOM);
            let mut app = LightcraftApp::new(session, Services::default());
            crate::mobile::install(&mut app);
            Ok(Box::new(App(app)))
        }),
    );
    if let Err(e) = result {
        log::error!("LightCraft stopped: {e}");
    }
}
