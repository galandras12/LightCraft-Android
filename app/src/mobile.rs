//! Phone / tablet layout for the LightCraft UI, modelled on Lightroom for Android.
//!
//! It replaces the desktop chrome (top bar, tool strip, docked panels) through the
//! `LightcraftApp::mobile_layout` hook (see `patches/0001-mobile-layout-hook.patch`) and reuses the
//! upstream widgets: the photo grid, the loupe (`panels::detail`), the slider row and the tool
//! panels (presets, crop, masking, remove).
//!
//! * Library: a top bar over the photo grid; a tap on a photo opens it.
//! * Edit, portrait: top bar, photo, then at the bottom a tool bar (Presets, Crop, Edit, Masking,
//!   Remove), the Edit tab row (Light, Color, Effects, Detail, Optics) and the sliders sheet.
//! * Edit, wide (tablet / landscape): the tool sheet moves to a panel on the right.

use egui::{Align, Color32, Frame, Id, Layout, Margin, RichText, Sense, Stroke, vec2};
use lightcraft_catalog::PhotoId;
use lightcraft_develop::{ControlSpec, Section, Track, WbMode};
use lightcraft_ui_egui::LightcraftApp;
use lightcraft_ui_egui::icons::{Icon, paint};
use lightcraft_ui_egui::panels;
use lightcraft_ui_egui::panels::edit::{apply_slider_out, control};
use lightcraft_ui_egui::state::{RightPanel, ViewMode};
use lightcraft_ui_egui::theme::Tokens;
use lightcraft_ui_egui::widgets;
use serde_json::json;

/// UI scale on top of the device's density: fingers need bigger targets than a mouse.
pub const ZOOM: f32 = 1.15;
/// Width (points, after [`ZOOM`]) from which the edit tools sit beside the photo.
const WIDE: f32 = 760.0;
const BAR_H: f32 = 56.0;
const TOOLBAR_H: f32 = 64.0;
const TABS_H: f32 = 52.0;

pub fn install(app: &mut LightcraftApp) {
    app.mobile_layout = Some(layout);
    app.ui.filmstrip = false;
}

/// Opens an edit tab, for the snapshots: `light`, `color`, `effects`, `detail`, `optics`.
pub fn open_edit_tab(ctx: &egui::Context, tab: &str) {
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    st.tool = Tool::Edit;
    st.tab = Tab::ALL.into_iter().find(|t| t.label().eq_ignore_ascii_case(tab)).unwrap_or(Tab::Light);
    ctx.data_mut(|d| d.insert_temp(key, st));
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tool {
    #[default]
    None,
    Presets,
    Crop,
    Edit,
    Masking,
    Remove,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Light,
    Color,
    Effects,
    Detail,
    Optics,
}

impl Tab {
    const ALL: [Tab; 5] = [Tab::Light, Tab::Color, Tab::Effects, Tab::Detail, Tab::Optics];
    fn label(self) -> &'static str {
        match self {
            Tab::Light => "Light",
            Tab::Color => "Color",
            Tab::Effects => "Effects",
            Tab::Detail => "Detail",
            Tab::Optics => "Optics",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Tab::Light => Icon::Sliders,
            Tab::Color => Icon::Radial,
            Tab::Effects => Icon::Eye,
            Tab::Detail => Icon::Target,
            Tab::Optics => Icon::Circle,
        }
    }
}

#[derive(Clone, Default)]
struct State {
    tool: Tool,
    tab: Tab,
    /// The active photo when the grid last ran, to tell a tap that selected a photo.
    grid_active: Option<PhotoId>,
}

fn layout(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    if !matches!(app.ui.view, ViewMode::PhotoGrid | ViewMode::SquareGrid | ViewMode::Detail) {
        let _ = app.run("view.photoGrid", json!({}));
    }
    let detail = app.ui.view == ViewMode::Detail;
    let wide = ui.max_rect().width() >= WIDE;
    top_bar(app, ui, &mut st, detail);
    if detail {
        edit_screen(app, ui, &mut st, wide);
    } else {
        grid_screen(app, ui, &mut st);
    }
    ctx.data_mut(|d| d.insert_temp(key, st));
}

fn icon_btn(ui: &mut egui::Ui, id: &str, icon: Icon, active: bool, enabled: bool, tip: &str) -> egui::Response {
    widgets::icon_button(ui, id, icon, vec2(44.0, 44.0), active, enabled, tip)
}

fn bar_frame(t: &Tokens) -> Frame {
    Frame::NONE.fill(t.chrome).inner_margin(Margin { left: 6, right: 6, top: 0, bottom: 0 })
}

fn top_bar(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, detail: bool) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("m_top").exact_size(BAR_H).frame(bar_frame(&t)).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if detail {
                if icon_btn(ui, "m_back", Icon::Back, false, true, "Back to library").clicked() {
                    st.tool = Tool::None;
                    app.ui.right = RightPanel::Edit;
                    let _ = app.run("view.photoGrid", json!({}));
                }
                let active = app.session.active();
                let visible = app.session.visible_shared().1;
                let pos = active.and_then(|a| visible.iter().position(|p| *p == a));
                let name = active.and_then(|a| app.session.catalog.photo(a).map(|p| p.file_name.clone())).unwrap_or_default();
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let count = pos.map(|p| format!("{} of {}", p + 1, visible.len())).unwrap_or_default();
                    ui.label(RichText::new(count).size(13.0).color(t.text_dim));
                    ui.add(egui::Label::new(RichText::new(name).size(15.0).color(t.text)).truncate());
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let more = icon_btn(ui, "m_more", Icon::More, false, true, "More");
                    egui::Popup::menu(&more).show(|ui| detail_menu(app, ui));
                    let can_undo = !app.session.undo.is_empty();
                    if widgets::text_button(ui, "m_undo", "Undo", false).clicked() && can_undo {
                        let _ = app.run("edit.undo", json!({}));
                    }
                });
            } else {
                ui.add_space(10.0);
                let total = app.session.visible_shared().1.len();
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new("LightCraft").size(18.0).strong().color(t.text));
                    ui.label(RichText::new(format!("{total} photos")).size(12.0).color(t.text_dim));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let more = icon_btn(ui, "m_more", Icon::More, false, true, "More");
                    egui::Popup::menu(&more).show(|ui| grid_menu(app, ui));
                    if icon_btn(ui, "m_import", Icon::Plus, false, true, "Add photos").clicked() {
                        // The Storage Access Framework picker is not wired up yet.
                        app.toast(ui.ctx(), "Adding photos from the device is not available yet");
                    }
                });
            }
        });
    });
}

fn detail_menu(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    ui.set_min_width(180.0);
    if ui.add_enabled(!app.session.redo.is_empty(), egui::Button::new("Redo")).clicked() {
        let _ = app.run("edit.redo", json!({}));
        ui.close();
    }
    if ui.button("Previous photo").clicked() {
        let _ = app.run("library.previous", json!({}));
        ui.close();
    }
    if ui.button("Next photo").clicked() {
        let _ = app.run("library.next", json!({}));
        ui.close();
    }
    if ui.button("Reset edits").clicked() {
        let _ = app.run("develop.reset", json!({}));
        ui.close();
    }
    if ui.button("Copy edits").clicked() {
        let _ = app.run("develop.copy", json!({}));
        ui.close();
    }
    if ui.button("Paste edits").clicked() {
        let _ = app.run("develop.paste", json!({}));
        ui.close();
    }
    if ui.button("Before / After").clicked() {
        let _ = app.run("view.beforeAfter", json!({}));
        ui.close();
    }
}

fn grid_menu(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    ui.set_min_width(180.0);
    if ui.button("Photo grid").clicked() {
        let _ = app.run("view.photoGrid", json!({}));
        ui.close();
    }
    if ui.button("Square grid").clicked() {
        let _ = app.run("view.squareGrid", json!({}));
        ui.close();
    }
    if ui.button("Select all").clicked() {
        let _ = app.run("library.selectAll", json!({}));
        ui.close();
    }
}

// ---------------------------------------------------------------------------------------------
// library

fn grid_screen(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State) {
    let t = Tokens::get(ui.ctx());
    // a thumbnail about a third of the width: three columns on a phone
    let width = ui.max_rect().width();
    app.ui.thumb_size = (width / 3.0).clamp(96.0, 240.0);
    let before = app.session.active();
    let rect = egui::CentralPanel::default().frame(Frame::NONE.fill(t.grid_bg)).show(ui, |ui| {
        let r = ui.max_rect();
        panels::grid::show(app, ui);
        r
    });
    // A tap that selected a photo opens it (a second tap on the open one is the grid's own
    // double-click).
    let released = ui.input(|i| i.pointer.any_released() && !i.pointer.is_decidedly_dragging());
    let pos = ui.input(|i| i.pointer.interact_pos());
    let after = app.session.active();
    if released && pos.is_some_and(|p| rect.inner.contains(p)) && after.is_some() && (after != before || after != st.grid_active) {
        let moved = ui.input(|i| i.pointer.press_origin().zip(i.pointer.interact_pos()).is_some_and(|(a, b)| a.distance(b) > 12.0));
        if !moved && app.ui.view != ViewMode::Detail && after != st.grid_active {
            let _ = app.run("view.detail", json!({}));
        }
    }
    st.grid_active = after;
}

// ---------------------------------------------------------------------------------------------
// edit

fn edit_screen(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, wide: bool) {
    let t = Tokens::get(ui.ctx());
    let height = ui.max_rect().height();
    // Tools that draw into the photo (crop frame, masks) need their panel's state set.
    app.ui.right = match st.tool {
        Tool::Crop => RightPanel::Crop,
        Tool::Remove => RightPanel::Remove,
        Tool::Masking => RightPanel::Masking,
        _ => RightPanel::Edit,
    };
    tool_bar(app, ui, st);
    if st.tool != Tool::None {
        if wide {
            egui::Panel::right("m_side").exact_size(380.0).frame(Frame::NONE.fill(t.chrome)).show(ui, |ui| {
                if st.tool == Tool::Edit {
                    tab_row(ui, st);
                }
                sheet(app, ui, st);
            });
        } else {
            if st.tool == Tool::Edit {
                egui::Panel::bottom("m_tabs").exact_size(TABS_H).frame(Frame::NONE.fill(t.chrome)).show(ui, |ui| tab_row(ui, st));
            }
            egui::Panel::bottom("m_sheet")
                .exact_size((height * 0.36).clamp(180.0, 420.0))
                .frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider)))
                .show(ui, |ui| sheet(app, ui, st));
        }
    }
    egui::CentralPanel::default().frame(Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        panels::detail::show(app, ui);
    });
}

fn tool_bar(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("m_tools").exact_size(TOOLBAR_H).frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(ui, |ui| {
        let w = ui.available_width();
        let tools: [(Option<Tool>, Icon, &str); 6] = [
            (None, Icon::Edited, "Auto"),
            (Some(Tool::Presets), Icon::Presets, "Presets"),
            (Some(Tool::Crop), Icon::Crop, "Crop"),
            (Some(Tool::Edit), Icon::Sliders, "Edit"),
            (Some(Tool::Masking), Icon::Mask, "Masking"),
            (Some(Tool::Remove), Icon::Eraser, "Remove"),
        ];
        let cell = w / tools.len() as f32;
        let (row, _) = ui.allocate_exact_size(vec2(w, TOOLBAR_H), Sense::hover());
        for (i, (tool, icon, label)) in tools.into_iter().enumerate() {
            let r = egui::Rect::from_min_size(row.min + vec2(cell * i as f32, 0.0), vec2(cell, TOOLBAR_H));
            let resp = ui.interact(r, Id::new(("m_tool", i)), Sense::click());
            let active = tool.is_some_and(|x| x == st.tool);
            if active {
                let pill = egui::Rect::from_center_size(r.center() - vec2(0.0, 9.0), vec2(52.0, 32.0));
                ui.painter().rect_filled(pill, 16.0, t.accent);
            }
            let color = if active { Color32::WHITE } else { t.icon };
            paint(ui.painter(), egui::Rect::from_center_size(r.center() - vec2(0.0, 9.0), vec2(22.0, 22.0)), icon, color);
            ui.painter().text(
                egui::pos2(r.center().x, r.bottom() - 11.0),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(11.5),
                if active { t.text } else { t.text_dim },
            );
            if resp.clicked() {
                match tool {
                    None => {
                        let _ = app.run("develop.auto", json!({}));
                        app.toast(ui.ctx(), "Auto settings applied");
                    }
                    Some(x) => st.tool = if st.tool == x { Tool::None } else { x },
                }
            }
        }
    });
}

fn tab_row(ui: &mut egui::Ui, st: &mut State) {
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width();
    let cell = (w / Tab::ALL.len() as f32).max(68.0);
    egui::ScrollArea::horizontal().id_salt("m_tabs_scroll").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
        let (row, _) = ui.allocate_exact_size(vec2(cell * Tab::ALL.len() as f32, TABS_H), Sense::hover());
        for (i, tab) in Tab::ALL.into_iter().enumerate() {
            let r = egui::Rect::from_min_size(row.min + vec2(cell * i as f32, 0.0), vec2(cell, TABS_H));
            let resp = ui.interact(r, Id::new(("m_tab", i)), Sense::click());
            let active = st.tab == tab;
            if active {
                ui.painter().rect_filled(r.shrink2(vec2(4.0, 5.0)), 8.0, t.hover);
            }
            paint(ui.painter(), egui::Rect::from_center_size(r.center() - vec2(0.0, 9.0), vec2(20.0, 20.0)), tab.icon(), if active { t.text } else { t.icon });
            ui.painter().text(
                egui::pos2(r.center().x, r.bottom() - 11.0),
                egui::Align2::CENTER_CENTER,
                tab.label(),
                egui::FontId::proportional(12.0),
                if active { t.text } else { t.text_dim },
            );
            if resp.clicked() {
                st.tab = tab;
            }
        }
    });
}

/// The panel of the chosen tool.
fn sheet(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State) {
    match st.tool {
        Tool::None => {}
        Tool::Edit => edit_sliders(app, ui, st.tab),
        Tool::Presets => panels::presets::show(app, ui),
        Tool::Crop | Tool::Masking | Tool::Remove => panels::right::show(app, ui),
    }
}

const LIGHT: &[&str] = &["light.exposure", "light.contrast", "light.highlights", "light.shadows", "light.whites", "light.blacks"];
const EFFECTS: &[&str] = &[
    "effects.texture",
    "effects.clarity",
    "effects.dehaze",
    "vignette.amount",
    "vignette.midpoint",
    "vignette.feather",
    "vignette.roundness",
    "vignette.highlights",
    "grain.amount",
    "grain.size",
    "grain.roughness",
];
const DETAIL: &[&str] = &[
    "detail.sharpenAmount",
    "detail.sharpenRadius",
    "detail.sharpenDetail",
    "detail.sharpenMasking",
    "detail.nrLuminance",
    "detail.nrDetail",
    "detail.nrContrast",
    "detail.nrColor",
    "detail.nrColorDetail",
    "detail.nrColorSmoothness",
];
const OPTICS: &[&str] = &[
    "optics.profileDistortion",
    "optics.profileVignetting",
    "optics.distortion",
    "optics.vignetting",
    "optics.vignettingMidpoint",
    "optics.defringePurple",
    "optics.defringeGreen",
    "optics.caRed",
    "optics.caBlue",
];

/// White balance of rendered files is a −100…100 scale (mireds around 6500 K), raws use Kelvin.
const REL_TEMP: ControlSpec = ControlSpec {
    id: "wb.tempRel",
    label: "Temp",
    section: Section::Color,
    min: -100.0,
    max: 100.0,
    default: 0.0,
    step: 1.0,
    decimals: 0,
    track: Track::Temp,
};
const REL_TINT: ControlSpec = ControlSpec {
    id: "wb.tintRel",
    label: "Tint",
    section: Section::Color,
    min: -100.0,
    max: 100.0,
    default: 0.0,
    step: 1.0,
    decimals: 0,
    track: Track::Tint,
};

fn k_to_rel(k: f64) -> f64 {
    ((1e6 / 6500.0 - 1e6 / k) / 0.8).clamp(-100.0, 100.0)
}

fn rel_to_k(r: f64) -> f64 {
    1e6 / (1e6 / 6500.0 - r * 0.8)
}

fn edit_sliders(app: &mut LightcraftApp, ui: &mut egui::Ui, tab: Tab) {
    let t = Tokens::get(ui.ctx());
    let Some(id) = app.session.active() else {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| ui.label(RichText::new("Select a photo to edit").color(t.text_dim)));
        return;
    };
    let mut d = app.session.develop_of(id).unwrap_or_default();
    let relative = app.session.catalog.photo(id).is_some_and(|p| p.relative_wb());
    let raw = app.session.catalog.photo(id).is_some_and(|p| p.develops_raw() && !relative);
    // the Kelvin sliders show the white balance in effect, as the desktop panel does
    if d.wb.mode == WbMode::AsShot && relative {
        let wb = &mut std::sync::Arc::make_mut(&mut d).wb;
        wb.temp = 6500.0;
        wb.tint = 0.0;
    } else if raw {
        let info = app.session.source_info(id);
        let (temp, tint) = lightcraft_engine::pipeline::local::effective_wb(&info, &d);
        let wb = &mut std::sync::Arc::make_mut(&mut d).wb;
        wb.temp = temp;
        wb.tint = tint;
    }
    egui::ScrollArea::vertical().id_salt(("m_edit_scroll", tab as u8)).auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(6.0);
        match tab {
            Tab::Light => list(app, ui, &d, LIGHT),
            Tab::Color => {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    if widgets::text_button(ui, "m_bw", "B&W", lightcraft_ui_egui::is_bw(&d)).clicked() {
                        let _ = app.run("develop.treatment", json!({}));
                    }
                    ui.label(RichText::new(format!("White balance: {}", d.wb.mode.label())).color(t.text_dim));
                });
                ui.add_space(6.0);
                if raw {
                    list(app, ui, &d, &["wb.temp", "wb.tint"]);
                } else {
                    let out = widgets::slider(ui, &REL_TEMP, k_to_rel(d.wb.temp), true, None);
                    apply_slider_out(app, &REL_TEMP, out, |app, v| app.run("develop.set", json!({"control": "wb.temp", "value": rel_to_k(v)})));
                    let out = widgets::slider(ui, &REL_TINT, d.wb.tint.clamp(-100.0, 100.0), true, None);
                    apply_slider_out(app, &REL_TINT, out, |app, v| app.run("develop.set", json!({"control": "wb.tint", "value": v})));
                }
                list(app, ui, &d, &["color.vibrance", "color.saturation"]);
            }
            Tab::Effects => list(app, ui, &d, EFFECTS),
            Tab::Detail => list(app, ui, &d, DETAIL),
            Tab::Optics => list(app, ui, &d, OPTICS),
        }
        ui.add_space(12.0);
    });
}

fn list(app: &mut LightcraftApp, ui: &mut egui::Ui, d: &lightcraft_develop::DevelopSettings, ids: &[&str]) {
    for id in ids {
        control(app, ui, d, id, true);
    }
}
