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
        ui.vertical_centered(|ui| ui.label(RichText::new("Select a photo to crop").color(t.text_dim)));
        return;
    };
    // The tab buttons.
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        for (tab, label) in [(CropTab::Aspect, "Aspect"), (CropTab::Geometry, "Geometry")] {
            let selected = st.tab == tab;
            let button = egui::Button::new(RichText::new(label).size(15.0).color(if selected { Color32::WHITE } else { t.text })).fill(if selected {
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

    // Free / Original, then the turning and flipping buttons.
    heading(ui, "Ratio");
    wrap(ui, |ui| {
        if chip(ui, "Free", locked.is_none()) {
            let _ = app.run("crop.aspect", json!({"aspect": "free"}));
        }
        if chip(ui, "Original", false) {
            let _ = app.run("crop.aspect", json!({"aspect": "original"}));
        }
        if chip(ui, "Swap orientation", false) {
            let _ = app.run("crop.rotateAspect", json!({}));
        }
    });

    // Common ratios.
    wrap(ui, |ui| {
        for r in RATIOS {
            if chip(ui, r.name, is_locked(locked, r)) {
                let _ = app.run("crop.aspect", json!({"aspect": [r.w, r.h]}));
            }
        }
    });

    // The formats of social networks.
    heading(ui, "Social media");
    wrap(ui, |ui| {
        for r in SOCIAL {
            if chip(ui, r.name, is_locked(locked, r)) {
                let _ = app.run("crop.aspect", json!({"aspect": [r.w, r.h]}));
            }
        }
    });

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
        ui.label(RichText::new(text).size(13.0).strong().color(t.text_dim));
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
    let text = RichText::new(label).size(14.0).color(if selected { Color32::WHITE } else { t.text });
    let button = egui::Button::new(text).fill(if selected { t.accent } else { t.button }).min_size(vec2(0.0, 38.0)).sense(Sense::click());
    ui.add(button).clicked()
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
