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
    /// Keep HDR in exported files of photos that were edited in HDR (JPEG gain map, PNG / AVIF / JPEG XL in PQ).
    pub keep_hdr: bool,
    /// Let the gallery apps of the device show the exported photos (off: a `.nomedia` file hides the folder).
    pub show_in_gallery: bool,
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
    /// The watermark written into exported photos.
    pub watermark: WmPrefs,
}

/// The watermark settings.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WmPrefs {
    /// Draw the watermark into exported photos.
    pub enabled: bool,
    /// Use a picture (`image`) instead of the text.
    pub use_image: bool,
    /// The text of the watermark.
    pub text: String,
    /// Path of the picture (PNG with transparency, JPEG...).
    pub image: String,
    /// Where a single watermark sits: `bottomLeft`, `topLeft`, `bottomRight`, `topRight`, `bottom` or `top`.
    pub position: String,
    /// Repeat the watermark over the whole photo as a mosaic.
    pub tile: bool,
    /// The direction of the text (the rows of a mosaic follow it): `horizontal`, `vertical`, `diagonalUp` or `diagonalDown`.
    pub direction: String,
    /// The gap between the tiles of a mosaic, as a fraction of the short edge.
    pub gap: f32,
    /// The colour of the text.
    pub color: [u8; 3],
    /// Opacity, 0.05 to 1.
    pub opacity: f32,
    /// Text height as a fraction of the short edge.
    pub size: f32,
    /// Width of the picture as a fraction of the photo's width.
    pub image_width: f32,
    /// The text style (see [`FONTS`]).
    pub font: String,
    /// A soft dark shadow under the text.
    pub shadow: bool,
}

impl Default for WmPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            use_image: false,
            text: "© Your Name".into(),
            image: String::new(),
            position: "bottomRight".into(),
            tile: false,
            direction: "horizontal".into(),
            gap: 0.12,
            color: [255; 3],
            opacity: 0.6,
            size: 0.05,
            image_width: 0.25,
            font: String::new(),
            shadow: true,
        }
    }
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
            keep_hdr: true,
            show_in_gallery: true,
            folder: platform().storage_root.join("DCIM").join("LightCraft").to_string_lossy().into_owned(),
            creator: String::new(),
            copyright: String::new(),
            write_copyright: true,
            copyright_on_import: false,
            watermark: WmPrefs::default(),
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
const FORMATS: [(&str, &str); 6] = [("jpeg", "JPEG"), ("png", "PNG"), ("tiff", "TIFF"), ("webp", "WebP"), ("avif", "AVIF"), ("jxl", "JPEG XL")];
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

/// The Settings screen. Returns what the layout should do next (the back arrow was tapped, or the file browser is wanted).
pub fn screen(app: &mut LightcraftApp, ui: &mut egui::Ui, insets: Insets) -> Option<Action> {
    let t_ = Tokens::get(ui.ctx());
    let mut action = None;
    egui::Panel::top("m_set_top").exact_size(BAR_H + insets.top).frame(bar_frame(&t_)).show(ui, |ui| {
        ui.add_space(insets.top);
        ui.horizontal_centered(|ui| {
            if icon_btn(ui, "m_set_back", Icon::Back, false, true, "Back").clicked() {
                action = Some(Action::Close);
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
            if watermark_section(ui, &mut p) {
                action = Some(Action::PickWatermarkImage);
            }
            ui.add_space(insets.bottom + 24.0);
            update(|stored| *stored = p);
        });
    });
    action
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
    if matches!(p.format.as_str(), "jpeg" | "webp" | "avif" | "jxl") {
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
    // photos edited in HDR stay HDR in the file (JPEG with a gain map, PNG / AVIF / JPEG XL in PQ); other photos are unaffected
    ui.checkbox(&mut p.keep_hdr, tr("Keep HDR in the file for photos that have it"));
    ui.checkbox(&mut p.show_in_gallery, tr("Show exported photos in the gallery"));
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

/// The watermark settings: text or picture, place or mosaic, direction, colour, opacity, size and text style, with a
/// small preview. Returns true when the watermark picture should be chosen with the file browser.
fn watermark_section(ui: &mut egui::Ui, p: &mut Prefs) -> bool {
    let t_ = Tokens::get(ui.ctx());
    let mut pick_image = false;
    heading(ui, "Watermark");
    let w = &mut p.watermark;
    ui.checkbox(&mut w.enabled, tr("Add a watermark to exported photos"));
    if !w.enabled {
        return false;
    }
    // text or picture
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut w.use_image, false, tr("Text"));
        ui.selectable_value(&mut w.use_image, true, tr("Picture"));
    });
    ui.add_space(4.0);
    if w.use_image {
        let name = std::path::Path::new(&w.image).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| tr("No picture chosen"));
        ui.label(RichText::new(name).color(t_.text_dim));
        if ui.button(tr("Choose a picture...")).clicked() {
            pick_image = true;
        }
        ui.label(RichText::new(tr("A PNG with a transparent background works best.")).size(12.0).color(t_.text_dim));
        row(ui, "Width", |ui| {
            let mut v = f64::from(w.image_width) * 100.0;
            if ui.add(egui::Slider::new(&mut v, 5.0..=100.0).suffix(" %").integer()).changed() {
                w.image_width = (v / 100.0) as f32;
            }
        });
    } else {
        ui.add(egui::TextEdit::singleline(&mut w.text).hint_text("© 2026 Your Name").desired_width(f32::INFINITY));
        ui.add_space(4.0);
        let fonts: Vec<(&str, String)> = FONTS.iter().map(|(c, l)| (*c, tr(l))).collect();
        row(ui, "Text style", |ui| {
            if let Some(code) = choice(ui, "m_wm_font", &w.font, &fonts) {
                w.font = code;
            }
        });
        row(ui, "Text size", |ui| {
            let mut v = f64::from(w.size) * 100.0;
            if ui.add(egui::Slider::new(&mut v, 1.0..=20.0).suffix(" %").integer()).changed() {
                w.size = (v / 100.0) as f32;
            }
        });
        // the colour: a few swatches and the full colour picker
        row(ui, "Colour", |ui| {
            ui.color_edit_button_srgb(&mut w.color);
            for swatch in [[255u8, 255, 255], [0, 0, 0], [200, 200, 200], [230, 60, 60], [250, 200, 40]] {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(swatch[0], swatch[1], swatch[2]));
                ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(if w.color == swatch { 2.5 } else { 1.0 }, t_.text_dim), egui::StrokeKind::Inside);
                if resp.clicked() {
                    w.color = swatch;
                }
            }
        });
        ui.checkbox(&mut w.shadow, tr("Soft shadow under the text"));
    }
    row(ui, "Opacity", |ui| {
        let mut v = f64::from(w.opacity) * 100.0;
        if ui.add(egui::Slider::new(&mut v, 5.0..=100.0).suffix(" %").integer()).changed() {
            w.opacity = (v / 100.0) as f32;
        }
    });
    // direction
    let directions: Vec<(&str, String)> = vec![
        ("horizontal", tr("Horizontal")),
        ("vertical", tr("Vertical")),
        ("diagonalUp", tr("Diagonal, rising")),
        ("diagonalDown", tr("Diagonal, falling")),
    ];
    row(ui, "Direction", |ui| {
        if let Some(code) = choice(ui, "m_wm_direction", &w.direction, &directions) {
            w.direction = code;
        }
    });
    // one mark in a place, or a mosaic over the whole photo
    ui.horizontal(|ui| {
        ui.selectable_value(&mut w.tile, false, tr("One watermark"));
        ui.selectable_value(&mut w.tile, true, tr("Mosaic"));
    });
    ui.add_space(4.0);
    if w.tile {
        row(ui, "Space between", |ui| {
            let mut v = f64::from(w.gap) * 100.0;
            if ui.add(egui::Slider::new(&mut v, 0.0..=50.0).suffix(" %").integer()).changed() {
                w.gap = (v / 100.0) as f32;
            }
        });
    } else {
        let places: Vec<(&str, String)> = vec![
            ("bottomLeft", tr("Bottom left")),
            ("topLeft", tr("Top left")),
            ("bottomRight", tr("Bottom right")),
            ("topRight", tr("Top right")),
            ("bottom", tr("Bottom middle")),
            ("top", tr("Top middle")),
        ];
        row(ui, "Position", |ui| {
            if let Some(code) = choice(ui, "m_wm_position", &w.position, &places) {
                w.position = code;
            }
        });
    }
    watermark_preview(ui, w);
    pick_image
}

/// A small sample picture with the watermark drawn on it as the export would draw it.
fn watermark_preview(ui: &mut egui::Ui, w: &WmPrefs) {
    let Some(json) = watermark_json(w) else { return };
    let Ok(mark) = serde_json::from_value::<lightcraft_engine::export::Watermark>(json) else { return };
    // the sample: a sky fading to dark hills
    let (sw, sh) = (480usize, 320usize);
    let mut img = lightcraft_raster::Rgba8::from_fn(sw, sh, |x, y| {
        let t = y as f32 / sh as f32;
        let hill = 0.62 + 0.08 * ((x as f32 / 70.0).sin() + 0.5 * (x as f32 / 23.0).sin());
        if t > hill {
            [28, 44 + (t * 60.0) as u8, 36, 255]
        } else {
            [(110.0 + 120.0 * t) as u8, (150.0 + 60.0 * t) as u8, (210.0 - 40.0 * t) as u8, 255]
        }
    });
    // the sample is smaller than an export: the text keeps its share of the short edge
    lightcraft_engine::export::draw_watermark(&mut img, &mark);
    let pixels: Vec<u8> = img.data.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    let image = egui::ColorImage::from_rgba_unmultiplied([sw, sh], &pixels);
    let key = egui::Id::new("m_wm_preview");
    let texture = ui.ctx().load_texture("m_wm_preview", image, egui::TextureOptions::LINEAR);
    ui.data_mut(|d| d.insert_temp(key, texture.id()));
    ui.add_space(8.0);
    ui.add(egui::Image::new(&texture).max_width(ui.available_width()).corner_radius(6.0));
}

/// Which photos a group export takes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The photos ticked in the library.
    Selected,
    /// Every photo shown in the library.
    All,
    /// The shown photos that have been edited.
    Edited,
}

/// The photos of `scope`, in library order.
pub fn photos_of(app: &mut LightcraftApp, scope: Scope) -> Vec<lightcraft_catalog::PhotoId> {
    let visible = app.session.visible_shared().1;
    match scope {
        Scope::All => visible.to_vec(),
        Scope::Selected => visible.iter().copied().filter(|id| app.session.selection.ids.contains(id)).collect(),
        Scope::Edited => visible.iter().copied().filter(|id| app.session.catalog.photo(*id).is_some_and(|p| p.is_edited())).collect(),
    }
}

/// The text styles of the watermark: (name registered with the engine, label). The first is the engine's own
/// Inter SemiBold; the others are bundled with the app (`app/fonts`, `register_fonts`).
pub const FONTS: [(&str, &str); 6] =
    [("", "Modern"), ("clean", "Clean"), ("heavy", "Heavy"), ("serif", "Serif"), ("serif-italic", "Serif italic"), ("mono", "Monospace")];

/// Registers the bundled text styles with the engine. Called once at start-up.
pub fn register_fonts() {
    lightcraft_engine::text::register_font("clean", include_bytes!("../../third_party/lightcraft/assets/fonts/Inter-Regular.ttf"));
    lightcraft_engine::text::register_font("heavy", include_bytes!("../fonts/Inter-Black.otf"));
    lightcraft_engine::text::register_font("serif", include_bytes!("../fonts/LiberationSerif-Regular.ttf"));
    lightcraft_engine::text::register_font("serif-italic", include_bytes!("../fonts/LiberationSerif-BoldItalic.ttf"));
    lightcraft_engine::text::register_font("mono", include_bytes!("../fonts/JetBrainsMono-Medium.ttf"));
}

/// The watermark as the engine's `watermark` export parameter; `None` when it is off or has nothing to draw.
pub fn watermark_json(w: &WmPrefs) -> Option<serde_json::Value> {
    if !w.enabled {
        return None;
    }
    let image = w.use_image && !w.image.trim().is_empty();
    if !image && w.text.trim().is_empty() {
        return None;
    }
    // the direction turns the text; the engine takes degrees clockwise
    let angle = match w.direction.as_str() {
        "vertical" => -90.0,
        "diagonalUp" => -45.0,
        "diagonalDown" => 45.0,
        _ => 0.0,
    };
    Some(json!({
        "text": if image { "" } else { w.text.trim() },
        "image": if image { w.image.trim() } else { "" },
        "imageWidth": w.image_width.clamp(0.01, 1.0),
        "size": w.size.clamp(0.005, 0.5),
        "opacity": w.opacity.clamp(0.0, 1.0),
        "color": w.color,
        "anchor": w.position,
        "angle": angle,
        "tile": w.tile,
        "gap": w.gap.clamp(0.0, 1.0),
        "font": w.font,
        "shadow": w.shadow,
    }))
}

/// What the Settings screen asks the layout to do.
pub enum Action {
    /// Leave the Settings screen.
    Close,
    /// Open the file browser to choose the watermark picture.
    PickWatermarkImage,
}

/// The export folder and the start time of an export whose files the gallery apps have not been told about yet.
static PENDING_SCAN: Mutex<Option<(std::path::PathBuf, std::time::SystemTime)>> = Mutex::new(None);

/// Once a background export has finished, tells the device's media scanner about the new files so that the
/// gallery apps show them. Called every frame; does nothing while an export runs or when none is pending.
pub fn poll_media_scan(app: &LightcraftApp) {
    if app.export.is_some() {
        return;
    }
    let Some((dir, since)) = PENDING_SCAN.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take() else { return };
    let since = since.checked_sub(std::time::Duration::from_secs(5)).unwrap_or(since);
    let files: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t >= since))
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();
    if !files.is_empty() {
        (platform().scan_media)(&files);
    }
}

/// Exports the active photo with the saved export settings, in the background. Returns the folder written to, or the error text.
pub fn export_active(app: &mut LightcraftApp) -> Result<String, String> {
    let id = app.session.active().ok_or_else(|| tr("Select a photo to edit"))?;
    export_ids(app, &[id]).map(|(_, dir)| dir)
}

/// Exports `ids` with the saved export settings, in the background. The copyright and creator are first added to the
/// photos that have none. Returns the number of photos and the folder written to, or the error text.
pub fn export_ids(app: &mut LightcraftApp, ids: &[lightcraft_catalog::PhotoId]) -> Result<(usize, String), String> {
    let p = get();
    if ids.is_empty() {
        return Err(tr("Nothing to export"));
    }
    std::fs::create_dir_all(&p.folder).map_err(|e| e.to_string())?;
    let mut metadata = p.metadata.clone();
    let (copyright, creator) = (p.copyright.trim(), p.creator.trim());
    if p.write_copyright && (!copyright.is_empty() || !creator.is_empty()) {
        // each field only goes to the photos that have none, so a photo's own text is never replaced
        for (field, value) in [("copyright", copyright), ("creator", creator)] {
            if value.is_empty() {
                continue;
            }
            let lacking: Vec<u64> = ids
                .iter()
                .filter(|id| {
                    app.session.catalog.photo(**id).is_some_and(|ph| if field == "copyright" { ph.meta.copyright.is_empty() } else { ph.meta.creator.is_empty() })
                })
                .map(|id| id.0)
                .collect();
            if !lacking.is_empty() {
                app.run("photo.setMeta", json!({"ids": lacking, field: value}))?;
            }
        }
        // "Nothing" would strip the notice that was just asked for
        if metadata == "none" {
            metadata = "copyright".into();
        }
    }
    let mut params = json!({
        "ids": ids.iter().map(|id| id.0).collect::<Vec<_>>(), "format": p.format, "quality": p.quality, "colorSpace": p.color_space,
        "metadata": metadata, "removeLocation": p.remove_location, "dir": p.folder, "background": true, "hdr": p.keep_hdr,
    });
    if p.long_edge > 0 {
        params["longEdge"] = json!(p.long_edge);
    }
    if let Some(wm) = watermark_json(&p.watermark) {
        params["watermark"] = wm;
    }
    // the gallery apps skip a folder that holds a `.nomedia` file
    let nomedia = std::path::Path::new(&p.folder).join(".nomedia");
    if p.show_in_gallery {
        let _ = std::fs::remove_file(&nomedia);
    } else {
        let _ = std::fs::write(&nomedia, b"");
    }
    app.run("app.export", params)?;
    if p.show_in_gallery {
        *PENDING_SCAN.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some((std::path::PathBuf::from(&p.folder), std::time::SystemTime::now()));
    }
    Ok((ids.len(), p.folder))
}
