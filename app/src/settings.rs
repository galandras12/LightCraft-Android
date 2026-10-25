//! The Settings screen of the phone / tablet interface (opened from the menu of the library's top bar)
//! and the preferences behind it: the interface language, the export settings and the copyright
//! information that can be written into exported files.
//!
//! The preferences are kept in one small JSON file in the app's data folder (`$HOME`, which the host sets
//! to the private storage on Android) and are loaded once.

use crate::i18n::{self, LANGUAGES, tf, tr};
use crate::mobile::{BAR_H, Insets, bar_frame, icon_btn, platform};
use egui::{Frame, Margin, RichText};
use lightcraft_ui_egui::LightcraftApp;
use lightcraft_ui_egui::i18n::Locale;
use lightcraft_ui_egui::icons::Icon;
use lightcraft_ui_egui::theme::Tokens;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Mutex;

/// The preferences of the app (everything in the Settings screen).
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Prefs {
    /// `auto` (follow the system language) or a language code (`hu`, `de`...); `en` is English.
    pub language: String,
    /// Export file format: `jpeg`, `png`, `tiff` or `webp`.
    pub format: String,
    /// JPEG / WebP quality, 1 to 100.
    pub quality: u32,
    /// Longest edge of the exported picture in pixels; 0 keeps the full size.
    pub long_edge: u32,
    /// Colour space of the exported file: `srgb`, `displayP3` or `adobeRgb`.
    pub color_space: String,
    /// What metadata the file carries: `all`, `allExceptCamera`, `copyright` or `none`.
    pub metadata: String,
    /// Leave the GPS position out of exported files.
    pub remove_location: bool,
    /// The folder exported files are written to.
    pub folder: String,
    /// The creator (author) name.
    pub creator: String,
    /// The copyright notice, for example `© 2026 Jane Doe`.
    pub copyright: String,
    /// Write the creator and the copyright notice into exported files.
    pub write_copyright: bool,
    /// Add the creator and the copyright notice to photos when they are imported.
    pub copyright_on_import: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            format: "jpeg".into(),
            quality: 90,
            long_edge: 0,
            color_space: "srgb".into(),
            metadata: "all".into(),
            remove_location: false,
            folder: platform().storage_root.join("Pictures").join("LightCraft").to_string_lossy().into_owned(),
            creator: String::new(),
            copyright: String::new(),
            write_copyright: true,
            copyright_on_import: false,
        }
    }
}

/// The preferences in memory (loaded on first use).
static PREFS: Mutex<Option<Prefs>> = Mutex::new(None);

/// Where the preferences are stored.
fn file() -> std::path::PathBuf {
    let base = std::env::var_os("LIGHTCRAFT_MOBILE_SETTINGS_DIR").or_else(|| std::env::var_os("HOME")).map(std::path::PathBuf::from).unwrap_or_default();
    base.join("lightcraft-mobile.json")
}

/// The current preferences.
pub fn get() -> Prefs {
    let mut guard = PREFS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.get_or_insert_with(|| std::fs::read(file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()).clone()
}

/// Changes the preferences with `f` and writes them to disk.
pub fn update(f: impl FnOnce(&mut Prefs)) {
    let mut next = get();
    f(&mut next);
    let mut guard = PREFS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.as_ref() != Some(&next) {
        if let Ok(bytes) = serde_json::to_vec_pretty(&next)
            && let Err(e) = std::fs::write(file(), bytes)
        {
            log::warn!("settings: {e}");
        }
        *guard = Some(next);
    }
}

/// The interface language the preferences ask for: the chosen one, or the system's (English when the
/// system language is not one of the shipped ones).
pub fn wanted_language(p: &Prefs) -> Locale {
    if p.language == "auto" {
        i18n::locale_for_system(&(platform().system_language)())
    } else {
        match Locale::parse(&p.language) {
            Some(l) if l == Locale::En || LANGUAGES.contains(&l) => l,
            _ => Locale::En,
        }
    }
}

/// The export formats of the Settings screen: (code, label).
const FORMATS: [(&str, &str); 4] = [("jpeg", "JPEG"), ("png", "PNG"), ("tiff", "TIFF"), ("webp", "WebP")];
/// The sizes of the Settings screen: longest edge in pixels (0 = full size).
const SIZES: [u32; 6] = [0, 4096, 3072, 2048, 1600, 1080];
/// The colour spaces: (code, label).
const SPACES: [(&str, &str); 3] = [("srgb", "sRGB"), ("displayP3", "Display P3"), ("adobeRgb", "Adobe RGB")];
/// The metadata choices: (code, label).
const METADATA: [(&str, &str); 4] =
    [("all", "Everything"), ("allExceptCamera", "Everything except camera data"), ("copyright", "Copyright only"), ("none", "Nothing")];

/// The label of a size choice.
fn size_label(px: u32) -> String {
    if px == 0 { tr("Full size") } else { tf("{n} px on the long edge", &[("n", px.to_string())]) }
}

/// A small grey heading above a group of settings.
fn heading(ui: &mut egui::Ui, text: &str) {
    let t_ = Tokens::get(ui.ctx());
    ui.add_space(18.0);
    ui.label(RichText::new(tr(text)).size(13.0).strong().color(t_.text_dim));
    ui.add_space(6.0);
}

/// One row: the label on the left, the control on the right.
fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.label(tr(label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add);
    });
    ui.add_space(4.0);
}

/// A drop-down list of `(code, label)` choices; returns the code that was picked.
fn choice(ui: &mut egui::Ui, id: &str, current: &str, items: &[(&str, String)]) -> Option<String> {
    let shown = items.iter().find(|(c, _)| *c == current).map(|(_, l)| l.clone()).unwrap_or_else(|| current.to_string());
    let mut picked = None;
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(190.0).show_ui(ui, |ui| {
        for (code, label) in items {
            if ui.selectable_label(*code == current, label).clicked() {
                picked = Some((*code).to_string());
            }
        }
    });
    picked
}

/// The Settings screen. Returns true when it should be closed (the back arrow was tapped).
pub fn screen(app: &mut LightcraftApp, ui: &mut egui::Ui, insets: Insets) -> bool {
    let t_ = Tokens::get(ui.ctx());
    let mut close = false;
    egui::Panel::top("m_set_top").exact_size(BAR_H + insets.top).frame(bar_frame(&t_)).show(ui, |ui| {
        ui.add_space(insets.top);
        ui.horizontal_centered(|ui| {
            if icon_btn(ui, "m_set_back", Icon::Back, false, true, "Back").clicked() {
                close = true;
            }
            ui.label(RichText::new(tr("Settings")).size(18.0).strong().color(t_.text));
        });
    });
    egui::CentralPanel::default().frame(Frame::NONE.fill(t_.grid_bg).inner_margin(Margin { left: 16, right: 16, top: 0, bottom: 0 })).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let mut p = get();
            ui.set_max_width(ui.available_width());
            language_section(ui, &mut p);
            export_section(ui, &mut p);
            copyright_section(app, ui, &mut p);
            ui.add_space(insets.bottom + 24.0);
            update(|stored| *stored = p);
        });
    });
    close
}

/// The language choice: Automatic (the system language) or one of the shipped languages.
fn language_section(ui: &mut egui::Ui, p: &mut Prefs) {
    heading(ui, "Language");
    let mut items: Vec<(&str, String)> = vec![("auto", tr("Automatic (system language)")), ("en", "English".to_string())];
    items.extend(LANGUAGES.iter().map(|l| (l.code(), l.name().to_string())));
    row(ui, "Interface language", |ui| {
        if let Some(code) = choice(ui, "m_set_language", &p.language, &items) {
            p.language = code;
        }
    });
    ui.label(RichText::new(tr("Languages the system uses that are not in the list show English.")).size(12.0).color(Tokens::get(ui.ctx()).text_dim));
}

/// The export settings: file format, quality, size, colour space, metadata and the destination folder.
fn export_section(ui: &mut egui::Ui, p: &mut Prefs) {
    heading(ui, "Export");
    let formats: Vec<(&str, String)> = FORMATS.iter().map(|(c, l)| (*c, (*l).to_string())).collect();
    row(ui, "File format", |ui| {
        if let Some(code) = choice(ui, "m_set_format", &p.format, &formats) {
            p.format = code;
        }
    });
    if matches!(p.format.as_str(), "jpeg" | "webp") {
        row(ui, "Quality", |ui| {
            let mut q = p.quality as f64;
            if ui.add(egui::Slider::new(&mut q, 1.0..=100.0).integer()).changed() {
                p.quality = q as u32;
            }
        });
    }
    let sizes: Vec<(&str, String)> = vec![("0", size_label(0)), ("4096", size_label(4096)), ("3072", size_label(3072)), ("2048", size_label(2048)), ("1600", size_label(1600)), ("1080", size_label(1080))];
    debug_assert_eq!(sizes.len(), SIZES.len());
    row(ui, "Size", |ui| {
        if let Some(code) = choice(ui, "m_set_size", &p.long_edge.to_string(), &sizes) {
            p.long_edge = code.parse().unwrap_or(0);
        }
    });
    let spaces: Vec<(&str, String)> = SPACES.iter().map(|(c, l)| (*c, (*l).to_string())).collect();
    row(ui, "Colour space", |ui| {
        if let Some(code) = choice(ui, "m_set_space", &p.color_space, &spaces) {
            p.color_space = code;
        }
    });
    let metadata: Vec<(&str, String)> = METADATA.iter().map(|(c, l)| (*c, tr(l))).collect();
    row(ui, "Metadata in the file", |ui| {
        if let Some(code) = choice(ui, "m_set_metadata", &p.metadata, &metadata) {
            p.metadata = code;
        }
    });
    ui.checkbox(&mut p.remove_location, tr("Remove the GPS position"));
    ui.add_space(6.0);
    ui.label(tr("Folder for exported files"));
    ui.add(egui::TextEdit::singleline(&mut p.folder).desired_width(f32::INFINITY));
    ui.label(RichText::new(tr("Use Export in the menu of the edit screen to write the photo here.")).size(12.0).color(Tokens::get(ui.ctx()).text_dim));
}

/// The copyright information: creator and notice, written into exported files and optionally added at import.
fn copyright_section(app: &mut LightcraftApp, ui: &mut egui::Ui, p: &mut Prefs) {
    heading(ui, "Copyright");
    ui.label(tr("Creator"));
    let creator = ui.add(egui::TextEdit::singleline(&mut p.creator).hint_text(tr("Your name")).desired_width(f32::INFINITY));
    ui.add_space(6.0);
    ui.label(tr("Copyright notice"));
    let notice = ui.add(egui::TextEdit::singleline(&mut p.copyright).hint_text("© 2026 Your Name").desired_width(f32::INFINITY));
    ui.add_space(6.0);
    ui.checkbox(&mut p.write_copyright, tr("Write the copyright information into exported files"));
    let toggled = ui.checkbox(&mut p.copyright_on_import, tr("Add it to photos when they are imported")).changed();
    // the import preference is the engine's own: it adds these fields to photos that have none
    if toggled || creator.lost_focus() || notice.lost_focus() {
        let (c, n) = if p.copyright_on_import { (p.copyright.trim(), p.creator.trim()) } else { ("", "") };
        let _ = app.run("library.preferences", json!({"import": {"copyright": c, "creator": n}}));
    }
}

/// Exports the active photo with the saved export settings, in the background. The copyright and creator
/// are first added to the photo when it has none. Returns the folder written to, or the error text.
pub fn export_active(app: &mut LightcraftApp) -> Result<String, String> {
    let p = get();
    let id = app.session.active().ok_or_else(|| tr("Select a photo to edit"))?;
    std::fs::create_dir_all(&p.folder).map_err(|e| e.to_string())?;
    let mut metadata = p.metadata.clone();
    if p.write_copyright && (!p.copyright.trim().is_empty() || !p.creator.trim().is_empty()) {
        let (has_c, has_n) = app.session.catalog.photo(id).map(|ph| (!ph.meta.copyright.is_empty(), !ph.meta.creator.is_empty())).unwrap_or((true, true));
        let mut fields = serde_json::Map::new();
        if !has_c && !p.copyright.trim().is_empty() {
            fields.insert("copyright".into(), json!(p.copyright.trim()));
        }
        if !has_n && !p.creator.trim().is_empty() {
            fields.insert("creator".into(), json!(p.creator.trim()));
        }
        if !fields.is_empty() {
            fields.insert("ids".into(), json!([id.0]));
            app.run("photo.setMeta", serde_json::Value::Object(fields))?;
        }
        // "Nothing" would strip the notice that was just asked for
        if metadata == "none" {
            metadata = "copyright".into();
        }
    }
    let mut params = json!({
        "ids": [id.0], "format": p.format, "quality": p.quality, "colorSpace": p.color_space,
        "metadata": metadata, "removeLocation": p.remove_location, "dir": p.folder, "background": true,
    });
    if p.long_edge > 0 {
        params["longEdge"] = json!(p.long_edge);
    }
    app.run("app.export", params)?;
    Ok(p.folder)
}
