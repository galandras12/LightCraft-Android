//! The Crop tool sheet of the phone layout.
//!
//! Two tabs, like Lightroom for Android:
//! * **Aspect**: the crop ratio (free, original, common print and screen ratios, and the formats of
//!   popular social networks), rotating and flipping the photo, straightening and resetting.
//! * **Geometry**: Upright (automatic perspective correction) and the manual transform sliders
//!   (vertical, horizontal, rotate, aspect, scale, offset).
//!
//! The crop frame itself is drawn over the photo by the upstream loupe while this tool is open.

use egui::{Align, Color32, Frame, Layout, Margin, RichText, Sense, vec2};
use lightcraft_develop::{Upright, controls};
use lightcraft_ui_egui::LightcraftApp;
use lightcraft_ui_egui::panels::edit::{apply_slider_out, control};
use lightcraft_ui_egui::theme::Tokens;
use lightcraft_ui_egui::widgets;
use crate::i18n::tr;
use serde_json::json;

/// The two tabs of the sheet.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CropTab {
    #[default]
    Aspect,
    Geometry,
}

/// What the Crop sheet remembers between frames.
#[derive(Clone, Default)]
pub struct CropState {
    /// The open tab.
    pub tab: CropTab,
}

/// A crop ratio offered as a button: its name and its width : height.
struct Ratio {
    name: &'static str,
    w: f64,
    h: f64,
}

/// Common ratios, landscape first and then the portrait ones.
const RATIOS: &[Ratio] = &[
    Ratio { name: "1 : 1", w: 1.0, h: 1.0 },
    Ratio { name: "4 : 5", w: 4.0, h: 5.0 },
    Ratio { name: "5 : 7", w: 5.0, h: 7.0 },
    Ratio { name: "2 : 3", w: 2.0, h: 3.0 },
    Ratio { name: "3 : 2", w: 3.0, h: 2.0 },
    Ratio { name: "4 : 3", w: 4.0, h: 3.0 },
    Ratio { name: "3 : 4", w: 3.0, h: 4.0 },
    Ratio { name: "16 : 9", w: 16.0, h: 9.0 },
    Ratio { name: "9 : 16", w: 9.0, h: 16.0 },
    Ratio { name: "16 : 10", w: 16.0, h: 10.0 },
    Ratio { name: "21 : 9", w: 21.0, h: 9.0 },
    Ratio { name: "3 : 1", w: 3.0, h: 1.0 },
    Ratio { name: "A4 / A-series", w: 210.0, h: 297.0 },
    Ratio { name: "US Letter", w: 8.5, h: 11.0 },
];

/// The picture formats of popular social networks (the crop only fixes the ratio; the pixel size is
/// chosen when exporting).
const SOCIAL: &[Ratio] = &[
    Ratio { name: "Instagram post  1 : 1", w: 1.0, h: 1.0 },
    Ratio { name: "Instagram portrait  4 : 5", w: 4.0, h: 5.0 },
    Ratio { name: "Instagram landscape  1.91 : 1", w: 191.0, h: 100.0 },
    Ratio { name: "Instagram story / Reel  9 : 16", w: 9.0, h: 16.0 },
    Ratio { name: "TikTok  9 : 16", w: 9.0, h: 16.0 },
    Ratio { name: "Facebook post  1.91 : 1", w: 191.0, h: 100.0 },
    Ratio { name: "Facebook cover  205 : 78", w: 205.0, h: 78.0 },
    Ratio { name: "Facebook story  9 : 16", w: 9.0, h: 16.0 },
    Ratio { name: "X (Twitter) post  16 : 9", w: 16.0, h: 9.0 },
    Ratio { name: "X (Twitter) header  3 : 1", w: 3.0, h: 1.0 },
    Ratio { name: "YouTube thumbnail  16 : 9", w: 16.0, h: 9.0 },
    Ratio { name: "LinkedIn post  1.91 : 1", w: 191.0, h: 100.0 },
    Ratio { name: "LinkedIn cover  4 : 1", w: 4.0, h: 1.0 },
    Ratio { name: "Pinterest pin  2 : 3", w: 2.0, h: 3.0 },
    Ratio { name: "Snapchat / WhatsApp status  9 : 16", w: 9.0, h: 16.0 },
];

/// The automatic Upright modes of the Geometry tab, with their button labels.
const UPRIGHT: &[(Upright, &str)] =
    &[(Upright::Off, "Off"), (Upright::Auto, "Auto"), (Upright::Level, "Level"), (Upright::Vertical, "Vertical"), (Upright::Full, "Full")];

/// The manual transform sliders of the Geometry tab.
const GEOMETRY: &[&str] = &["geometry.vertical", "geometry.horizontal", "geometry.rotate", "geometry.aspect", "geometry.scale", "geometry.offsetX", "geometry.offsetY"];

/// The whole Crop sheet: the two tab buttons and the open tab below them.
pub fn sheet(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut CropState) {
    let t = Tokens::get(ui.ctx());
    let Some(id) = app.session.active() else {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| ui.label(RichText::new(tr("Select a photo to crop")).color(t.text_dim)));
        return;
    };
    // The tab buttons.
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        for (tab, label) in [(CropTab::Aspect, "Aspect"), (CropTab::Geometry, "Geometry")] {
            let selected = st.tab == tab;
            let button = egui::Button::new(RichText::new(tr(label)).size(15.0).color(if selected { Color32::WHITE } else { t.text })).fill(if selected {
                t.accent
            } else {
                t.button
            });
            if ui.add_sized(vec2(110.0, 38.0), button).clicked() {
                st.tab = tab;
            }
        }
    });
    ui.add_space(4.0);
    egui::ScrollArea::vertical().id_salt(("m_crop_scroll", st.tab as u8)).auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        match st.tab {
            CropTab::Aspect => aspect_tab(app, ui, id),
            CropTab::Geometry => geometry_tab(app, ui, id),
        }
        ui.add_space(16.0);
    });
}

/// The Aspect tab: ratio buttons, rotate / flip, straighten and reset.
fn aspect_tab(app: &mut LightcraftApp, ui: &mut egui::Ui, id: lightcraft_catalog::PhotoId) {
    let t = Tokens::get(ui.ctx());
    let d = app.session.develop_of(id).unwrap_or_default();
    let locked = app.crop_tool_aspect();

    // The crop ratio: a drop-down list (Free, Original, common ratios, social media formats) and a button
    // that swaps portrait and landscape.
    heading(ui, "Ratio");
    let original = app.session.catalog.photo(id).map(|p| {
        let (w, h) = (f64::from(p.width.max(1)), f64::from(p.height.max(1)));
        if d.orientation.swaps_axes() { h / w } else { w / h }
    });
    let mut chosen: Option<serde_json::Value> = None;
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        egui::ComboBox::from_id_salt("m_crop_ratio")
            .selected_text(ratio_label(locked, original))
            .width((ui.available_width() - 190.0).max(140.0))
            .show_ui(ui, |ui| {
                ui.set_min_width(240.0);
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    if ui.selectable_label(locked.is_none(), tr("Free")).clicked() {
                        chosen = Some(json!("free"));
                    }
                    if ui.selectable_label(is_original(locked, original), "Original").clicked() {
                        chosen = Some(json!("original"));
                    }
                    ui.separator();
                    ui.label(RichText::new(tr("Ratios")).size(12.0).color(t.text_dim));
                    for r in RATIOS {
                        if ui.selectable_label(is_locked(locked, r), r.name).clicked() {
                            chosen = Some(json!([r.w, r.h]));
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new(tr("Social media")).size(12.0).color(t.text_dim));
                    for r in SOCIAL {
                        if ui.selectable_label(is_locked(locked, r), r.name).clicked() {
                            chosen = Some(json!([r.w, r.h]));
                        }
                    }
                });
            });
        // Swaps portrait and landscape of the chosen ratio.
        if chip(ui, "Swap", false) {
            let _ = app.run("crop.rotateAspect", json!({}));
        }
    });
    if let Some(aspect) = chosen {
        let _ = app.run("crop.aspect", json!({"aspect": aspect}));
    }

    // Turning and flipping the photo, and resetting the crop.
    heading(ui, "Rotate and flip");
    wrap(ui, |ui| {
        if chip(ui, "Rotate left", false) {
            let _ = app.run("photo.rotateLeft", json!({}));
        }
        if chip(ui, "Rotate right", false) {
            let _ = app.run("photo.rotateRight", json!({}));
        }
        if chip(ui, "Flip horizontal", d.crop.flip_h) {
            let _ = app.run("photo.flipHorizontal", json!({}));
        }
        if chip(ui, "Flip vertical", d.crop.flip_v) {
            let _ = app.run("photo.flipVertical", json!({}));
        }
        if chip(ui, "Reset crop", false) {
            let _ = app.run("crop.reset", json!({}));
        }
    });

    // Straighten: a slider for the angle and an automatic horizon detection.
    heading(ui, "Straighten");
    wrap(ui, |ui| {
        if chip(ui, "Auto straighten", false) {
            let _ = app.run("crop.autoStraighten", json!({}));
        }
    });
    if let Some(spec) = controls::find("crop.angle").copied() {
        let out = widgets::slider(ui, &spec, d.crop.geometry.angle, true, Some("Angle"));
        apply_slider_out(app, &spec, out, |app, v| app.run("crop.straighten", json!({"angle": v})));
    }
    let _ = t;
}

/// The Geometry tab: Upright modes and the manual transform sliders.
fn geometry_tab(app: &mut LightcraftApp, ui: &mut egui::Ui, id: lightcraft_catalog::PhotoId) {
    let d = app.session.develop_of(id).unwrap_or_default();
    heading(ui, "Upright (automatic perspective correction)");
    wrap(ui, |ui| {
        for (mode, label) in UPRIGHT {
            if chip(ui, label, d.geometry.upright == *mode) {
                if let Err(e) = app.run("geometry.upright", json!({"mode": mode})) {
                    app.toast(ui.ctx(), e);
                }
            }
        }
    });
    heading(ui, "Transform");
    for c in GEOMETRY {
        control(app, ui, &d, c, true);
    }
}

/// A small section heading.
fn heading(ui: &mut egui::Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(RichText::new(tr(text)).size(13.0).strong().color(t.text_dim));
    });
    ui.add_space(4.0);
}

/// Lays buttons out in rows that wrap, with the sheet's side margins.
fn wrap(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    Frame::NONE.inner_margin(Margin { left: 16, right: 16, top: 0, bottom: 0 }).show(ui, |ui| {
        ui.with_layout(Layout::left_to_right(Align::Center).with_main_wrap(true), |ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            add(ui);
        });
    });
}

/// A touch-sized button; returns `true` when it was tapped. A selected one is highlighted.
fn chip(ui: &mut egui::Ui, label: &str, selected: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let text = RichText::new(tr(label)).size(14.0).color(if selected { Color32::WHITE } else { t.text });
    let button = egui::Button::new(text).fill(if selected { t.accent } else { t.button }).min_size(vec2(0.0, 38.0)).sense(Sense::click());
    ui.add(button).clicked()
}

/// Whether the crop is locked to the photo's own shape (`original`: its width / height).
fn is_original(locked: Option<(u32, u32)>, original: Option<f64>) -> bool {
    match (locked, original) {
        (Some((w, h)), Some(o)) if w > 0 && h > 0 => (f64::from(w) / f64::from(h) - o).abs() < 0.01 * o,
        _ => false,
    }
}

/// What the ratio drop-down says: Free, Original, the name of the ratio in the lists, or the numbers.
fn ratio_label(locked: Option<(u32, u32)>, original: Option<f64>) -> String {
    let Some((w, h)) = locked else { return "Free".to_string() };
    if w == 0 || h == 0 {
        return "Free".to_string();
    }
    if is_original(locked, original) {
        return "Original".to_string();
    }
    if let Some(r) = RATIOS.iter().chain(SOCIAL).find(|r| is_locked(locked, r)) {
        return r.name.to_string();
    }
    format!("{:.2} : 1", f64::from(w) / f64::from(h))
}

/// Whether the crop is locked to `ratio` (in its own orientation: 2 : 3 is not 3 : 2). `locked` is the crop's width and
/// height in hundredths, as the upstream UI stores it.
fn is_locked(locked: Option<(u32, u32)>, ratio: &Ratio) -> bool {
    let Some((w, h)) = locked else { return false };
    if w == 0 || h == 0 {
        return false;
    }
    let have = f64::from(w) / f64::from(h);
    let want = ratio.w / ratio.h;
    (have - want).abs() < 0.01 * want
}
