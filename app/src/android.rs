//! NativeActivity entry point.
//!
//! The library lives in the app's private storage; a new library starts empty (no sample photos). File import (Storage Access Framework) is not wired yet.

use android_activity::AndroidApp;
use jni::JavaVM;
use jni::objects::{JObject, JValue};
use lightcraft_engine::Session;
use lightcraft_ui_egui::{LightcraftApp, Services};

/// The application id; it must match the `package` of `android/AndroidManifest.xml`.
const PACKAGE: &str = "ai.storyteller.lightcraft.android";

/// The running activity, kept for the JNI calls below.
static APP: std::sync::OnceLock<AndroidApp> = std::sync::OnceLock::new();

/// Runs `f` with a JNI environment and the activity; a Java exception is cleared and reported.
fn with_jni<T>(f: impl FnOnce(&mut jni::JNIEnv, &JObject) -> jni::errors::Result<T>) -> Option<T> {
    let app = APP.get()?;
    // SAFETY: the pointers come from the running NativeActivity and stay valid while it lives.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr() as jni::sys::jobject) };
    match f(&mut env, &activity) {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("jni: {e}");
            let _ = env.exception_clear();
            None
        }
    }
}

/// The Android API level of the device.
fn sdk_int(env: &mut jni::JNIEnv) -> jni::errors::Result<i32> {
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()
}

/// `[left, top, right, bottom]` of the system bars and display cut-out, in pixels.
fn insets_px() -> [f32; 4] {
    with_jni(|env, activity| {
        let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        let decor = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
        let wi = env.call_method(&decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?.l()?;
        if wi.as_raw().is_null() {
            return Ok([0.0; 4]);
        }
        if sdk_int(env)? >= 30 {
            let bars = env.call_static_method("android/view/WindowInsets$Type", "systemBars", "()I", &[])?.i()?;
            let cutout = env.call_static_method("android/view/WindowInsets$Type", "displayCutout", "()I", &[])?.i()?;
            let ins = env.call_method(&wi, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(bars | cutout)])?.l()?;
            let mut out = [0.0; 4];
            for (i, f) in ["left", "top", "right", "bottom"].into_iter().enumerate() {
                out[i] = env.get_field(&ins, f, "I")?.i()? as f32;
            }
            Ok(out)
        } else {
            let mut out = [0.0; 4];
            for (i, m) in ["getSystemWindowInsetLeft", "getSystemWindowInsetTop", "getSystemWindowInsetRight", "getSystemWindowInsetBottom"].into_iter().enumerate() {
                out[i] = env.call_method(&wi, m, "()I", &[])?.i()? as f32;
            }
            Ok(out)
        }
    })
    .unwrap_or([0.0; 4])
}

/// All-files access on Android 11+, the read permission before it.
fn storage_access() -> bool {
    with_jni(|env, activity| {
        if sdk_int(env)? >= 30 {
            env.call_static_method("android/os/Environment", "isExternalStorageManager", "()Z", &[])?.z()
        } else {
            let perm = env.new_string("android.permission.READ_EXTERNAL_STORAGE")?;
            Ok(env.call_method(activity, "checkSelfPermission", "(Ljava/lang/String;)I", &[JValue::Object(&perm)])?.i()? == 0)
        }
    })
    .unwrap_or(false)
}

/// Asks the user for file access: opens the "All files access" settings page of the app (Android 11+),
/// or shows the permission prompt on older versions.
fn request_storage_access() {
    with_jni(|env, activity| {
        if sdk_int(env)? >= 30 {
            let action = env.new_string("android.settings.MANAGE_APP_ALL_FILES_ACCESS_PERMISSION")?;
            let intent = env.new_object("android/content/Intent", "(Ljava/lang/String;)V", &[JValue::Object(&action)])?;
            let uri_text = env.new_string(format!("package:{PACKAGE}"))?;
            let uri = env.call_static_method("android/net/Uri", "parse", "(Ljava/lang/String;)Landroid/net/Uri;", &[JValue::Object(&uri_text)])?.l()?;
            env.call_method(&intent, "setData", "(Landroid/net/Uri;)Landroid/content/Intent;", &[JValue::Object(&uri)])?;
            env.call_method(activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&intent)])?;
        } else {
            let perm = env.new_string("android.permission.READ_EXTERNAL_STORAGE")?;
            let string_class = env.find_class("java/lang/String")?;
            let list = env.new_object_array(1, string_class, &perm)?;
            env.call_method(activity, "requestPermissions", "([Ljava/lang/String;I)V", &[JValue::Object(&list), JValue::Int(1)])?;
        }
        Ok(())
    });
}

/// The system language as a BCP-47 tag (`hu-HU`), from `java.util.Locale.getDefault()`; empty when unknown.
fn system_language() -> String {
    with_jni(|env, _| {
        let locale = env.call_static_method("java/util/Locale", "getDefault", "()Ljava/util/Locale;", &[])?.l()?;
        let tag = env.call_method(&locale, "toLanguageTag", "()Ljava/lang/String;", &[])?.l()?;
        let tag: String = env.get_string(&jni::objects::JString::from(tag))?.into();
        Ok(tag)
    })
    .unwrap_or_default()
}

/// Wraps the upstream app so that `eframe` can drive it.
struct App(LightcraftApp);

/// Forwards the frame callbacks of `eframe` to the upstream app.
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.0.raw_input_hook(raw);
        // the selection mode of the library makes a tap tick a photo
        crate::mobile::input_hook(ctx, raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("LightCraft"));
    log::info!("HEIC/HEIF support: {}", crate::heic_supported());
    let _ = APP.set(app.clone());
    crate::mobile::set_platform(crate::mobile::Platform {
        insets_px,
        storage_access,
        request_storage_access,
        storage_root: "/storage/emulated/0".into(),
        system_language,
    });
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
            let opened = library.as_deref().map(|dir| session.open_library(dir, false).map(|_| ()));
            if let Some(Err(e)) = opened {
                log::error!("library: {e}");
                session = Session::new().with_fs().with_system_clock();
            }
            // the "can't lock the library" notice is for desktops with two programs on one library
            for w in session.take_library_warnings() {
                log::warn!("library: {w}");
            }
            cc.egui_ctx.set_zoom_factor(crate::mobile::ZOOM);
            // exported files are written straight to the folder the Settings screen names
            let mut services = Services::default();
            services.write = Some(Box::new(|path: &str, bytes: &[u8]| lightcraft_engine::export::write_file(path, bytes)));
            services.write_shared = Some(std::sync::Arc::new(|path: &str, bytes: &[u8]| lightcraft_engine::export::write_file(path, bytes)));
            let mut app = LightcraftApp::new(session, services);
            crate::mobile::install(&mut app);
            Ok(Box::new(App(app)))
        }),
    );
    if let Err(e) = result {
        log::error!("LightCraft stopped: {e}");
    }
}
