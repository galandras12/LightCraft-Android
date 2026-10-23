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

/// What the host platform provides to the layout.
pub struct Platform {
    /// System bars / cut-outs as `[left, top, right, bottom]` in physical pixels.
    pub insets_px: fn() -> [f32; 4],
    /// The app may read the shared storage.
    pub storage_access: fn() -> bool,
    /// Ask the user for it (opens the system settings or a permission prompt).
    pub request_storage_access: fn(),
    /// Where the file browser starts.
    pub storage_root: std::path::PathBuf,
}

static PLATFORM: std::sync::OnceLock<Platform> = std::sync::OnceLock::new();

pub fn set_platform(p: Platform) {
    let _ = PLATFORM.set(p);
}

fn platform() -> &'static Platform {
    PLATFORM.get_or_init(|| Platform {
        insets_px: || [0.0; 4],
        storage_access: || true,
        request_storage_access: || {},
        storage_root: std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(|| "/".into()),
    })
}

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
    /// The file browser of Add photos, while it is open.
    browser: Option<Browser>,
}

fn layout(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    if !matches!(app.ui.view, ViewMode::PhotoGrid | ViewMode::SquareGrid | ViewMode::Detail) {
        let _ = app.run("view.photoGrid", json!({}));
    }
    let ppp = ctx.pixels_per_point().max(0.5);
    let [l, t, r, b] = (platform().insets_px)().map(|v| v / ppp);
    let insets = Insets { left: l, top: t, right: r, bottom: b };
    let t_ = Tokens::get(&ctx);
    // the system bars' area is painted in the chrome colour, so content never runs under it
    if insets.left > 0.5 {
        egui::Panel::left("m_inset_l").exact_size(insets.left).frame(Frame::NONE.fill(t_.chrome)).show(ui, |_| {});
    }
    if insets.right > 0.5 {
        egui::Panel::right("m_inset_r").exact_size(insets.right).frame(Frame::NONE.fill(t_.chrome)).show(ui, |_| {});
    }
    let detail = app.ui.view == ViewMode::Detail;
    let wide = ui.max_rect().width() >= WIDE;
    if st.browser.is_some() {
        browser_screen(app, ui, &mut st, insets);
    } else {
        top_bar(app, ui, &mut st, detail, insets);
        if detail {
            edit_screen(app, ui, &mut st, wide, insets);
        } else {
            grid_screen(app, ui, &mut st, insets);
        }
    }
    ctx.data_mut(|d| d.insert_temp(key, st));
}

#[derive(Clone, Copy, Default)]
struct Insets {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

fn icon_btn(ui: &mut egui::Ui, id: &str, icon: Icon, active: bool, enabled: bool, tip: &str) -> egui::Response {
    widgets::icon_button(ui, id, icon, vec2(44.0, 44.0), active, enabled, tip)
}

fn bar_frame(t: &Tokens) -> Frame {
    Frame::NONE.fill(t.chrome).inner_margin(Margin { left: 6, right: 6, top: 0, bottom: 0 })
}

fn top_bar(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, detail: bool, insets: Insets) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("m_top").exact_size(BAR_H + insets.top).frame(bar_frame(&t)).show(ui, |ui| {
        ui.add_space(insets.top);
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
                        st.browser = Some(Browser::open(&start_dir()));
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

fn grid_screen(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, insets: Insets) {
    let t = Tokens::get(ui.ctx());
    if insets.bottom > 0.5 {
        egui::Panel::bottom("m_nav_pad").exact_size(insets.bottom).frame(Frame::NONE.fill(t.grid_bg)).show(ui, |_| {});
    }
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

fn edit_screen(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, wide: bool, insets: Insets) {
    let t = Tokens::get(ui.ctx());
    let height = ui.max_rect().height();
    // Tools that draw into the photo (crop frame, masks) need their panel's state set.
    app.ui.right = match st.tool {
        Tool::Crop => RightPanel::Crop,
        Tool::Remove => RightPanel::Remove,
        Tool::Masking => RightPanel::Masking,
        _ => RightPanel::Edit,
    };
    tool_bar(app, ui, st, insets.bottom);
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

fn tool_bar(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, inset_bottom: f32) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("m_tools").exact_size(TOOLBAR_H + inset_bottom).frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(ui, |ui| {
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

// ---------------------------------------------------------------------------------------------
// add photos: a file browser over the device's storage

#[derive(Clone)]
struct Entry {
    path: std::path::PathBuf,
    name: String,
    dir: bool,
    size: u64,
}

#[derive(Clone)]
struct Browser {
    dir: std::path::PathBuf,
    entries: Vec<Entry>,
    selected: std::collections::BTreeSet<std::path::PathBuf>,
    error: Option<String>,
}

fn start_dir() -> std::path::PathBuf {
    let root = platform().storage_root.clone();
    let dcim = root.join("DCIM");
    if dcim.is_dir() { dcim } else { root }
}

fn is_photo(name: &str) -> bool {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    lightcraft_engine::import::EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(&ext))
}

impl Browser {
    fn open(dir: &std::path::Path) -> Browser {
        let mut b = Browser { dir: dir.to_path_buf(), entries: vec![], selected: Default::default(), error: None };
        b.read();
        b
    }

    fn read(&mut self) {
        self.entries.clear();
        self.error = None;
        match std::fs::read_dir(&self.dir) {
            Ok(rd) => {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') {
                        continue;
                    }
                    let Ok(meta) = e.metadata() else { continue };
                    if meta.is_dir() || is_photo(&name) {
                        self.entries.push(Entry { path: e.path(), name, dir: meta.is_dir(), size: meta.len() });
                    }
                }
                self.entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
            }
            Err(e) => self.error = Some(format!("{}: {e}", self.dir.display())),
        }
    }

    fn go(&mut self, dir: &std::path::Path) {
        self.dir = dir.to_path_buf();
        self.selected.clear();
        self.read();
    }

    fn files(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| !e.dir)
    }
}

fn size_text(bytes: u64) -> String {
    if bytes >= 1 << 20 { format!("{:.1} MB", bytes as f64 / (1 << 20) as f64) } else { format!("{} kB", bytes.div_ceil(1024)) }
}

fn browser_screen(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, insets: Insets) {
    let t = Tokens::get(ui.ctx());
    let Some(mut b) = st.browser.take() else { return };
    let access = (platform().storage_access)();
    let root = platform().storage_root.clone();
    let mut close = false;
    let mut import: Option<Vec<String>> = None;
    let mut go: Option<std::path::PathBuf> = None;

    egui::Panel::top("m_b_top").exact_size(BAR_H + insets.top).frame(bar_frame(&t)).show(ui, |ui| {
        ui.add_space(insets.top);
        ui.horizontal_centered(|ui| {
            if icon_btn(ui, "m_b_back", Icon::Back, false, true, "Back").clicked() {
                match b.dir.parent() {
                    Some(parent) if access && b.dir != root && b.dir.starts_with(&root) => go = Some(parent.to_path_buf()),
                    _ => close = true,
                }
            }
            ui.vertical(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new("Add photos").size(13.0).color(t.text_dim));
                let name = b.dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| b.dir.display().to_string());
                ui.add(egui::Label::new(RichText::new(name).size(16.0).color(t.text)).truncate());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if access && b.files().next().is_some() && widgets::text_button(ui, "m_b_all", "Select all", false).clicked() {
                    let all: Vec<_> = b.files().map(|e| e.path.clone()).collect();
                    if all.iter().all(|p| b.selected.contains(p)) {
                        b.selected.clear();
                    } else {
                        b.selected.extend(all);
                    }
                }
            });
        });
    });

    let n = b.selected.len();
    if access {
        egui::Panel::bottom("m_b_bottom").exact_size(TOOLBAR_H + insets.bottom).frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(16.0);
                let label = if n > 0 { format!("Import {n} photo{}", if n == 1 { "" } else { "s" }) } else { "Import this folder".to_string() };
                let enabled = n > 0 || b.entries.iter().any(|e| !e.dir) || b.entries.iter().any(|e| e.dir);
                let btn = egui::Button::new(RichText::new(label).size(16.0).color(Color32::WHITE)).fill(t.accent).min_size(vec2(ui.available_width() - 16.0, 44.0));
                if ui.add_enabled(enabled, btn).clicked() {
                    import = Some(if n > 0 {
                        b.selected.iter().map(|p| p.to_string_lossy().into_owned()).collect()
                    } else {
                        vec![b.dir.to_string_lossy().into_owned()]
                    });
                }
            });
        });
    }

    egui::CentralPanel::default().frame(Frame::NONE.fill(t.grid_bg)).show(ui, |ui| {
        if !access {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(ui.available_width() - 48.0);
                ui.label(RichText::new("LightCraft needs access to the device's files to add photos.").size(16.0).color(t.text));
                ui.add_space(8.0);
                ui.label(RichText::new("Allow it in the settings page that opens, then come back.").size(13.0).color(t.text_dim));
                ui.add_space(16.0);
                if ui.add(egui::Button::new(RichText::new("Allow file access").color(Color32::WHITE)).fill(t.accent).min_size(vec2(220.0, 44.0))).clicked() {
                    (platform().request_storage_access)();
                }
                ui.add_space(8.0);
                if ui.add(egui::Button::new("Cancel").min_size(vec2(220.0, 44.0))).clicked() {
                    close = true;
                }
            });
            return;
        }
        if let Some(e) = &b.error {
            ui.add_space(24.0);
            ui.label(RichText::new(e).color(t.reject));
            return;
        }
        if b.entries.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| ui.label(RichText::new("No folders or photos here").color(t.text_dim)));
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (i, e) in b.entries.clone().iter().enumerate() {
                let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::click());
                let chosen = b.selected.contains(&e.path);
                if chosen {
                    ui.painter().rect_filled(row, 0.0, t.hover);
                }
                let icon_rect = egui::Rect::from_center_size(egui::pos2(row.left() + 32.0, row.center().y), vec2(24.0, 24.0));
                paint(ui.painter(), icon_rect, if e.dir { Icon::Folder } else { Icon::Photos }, t.icon);
                ui.painter().text(
                    egui::pos2(row.left() + 64.0, row.center().y - if e.dir { 0.0 } else { 9.0 }),
                    egui::Align2::LEFT_CENTER,
                    &e.name,
                    egui::FontId::proportional(15.0),
                    t.text,
                );
                if !e.dir {
                    ui.painter().text(egui::pos2(row.left() + 64.0, row.center().y + 11.0), egui::Align2::LEFT_CENTER, size_text(e.size), egui::FontId::proportional(12.0), t.text_dim);
                }
                if chosen {
                    paint(ui.painter(), egui::Rect::from_center_size(egui::pos2(row.right() - 32.0, row.center().y), vec2(22.0, 22.0)), Icon::Check, t.accent);
                } else if e.dir {
                    paint(ui.painter(), egui::Rect::from_center_size(egui::pos2(row.right() - 32.0, row.center().y), vec2(18.0, 18.0)), Icon::ChevronRight, t.text_dim);
                }
                ui.painter().line_segment([egui::pos2(row.left() + 64.0, row.bottom()), row.right_bottom()], Stroke::new(1.0, t.divider));
                if resp.clicked() {
                    if e.dir {
                        go = Some(e.path.clone());
                    } else if !b.selected.remove(&e.path) {
                        b.selected.insert(e.path.clone());
                    }
                }
                let _ = i;
            }
        });
    });

    if let Some(dir) = go {
        b.go(&dir);
    }
    if let Some(paths) = import {
        match lightcraft_ui_egui::import::start_paths(app, paths) {
            Ok(_) => {
                app.ui.view = ViewMode::PhotoGrid;
                close = true;
            }
            Err(e) => app.toast(ui.ctx(), e),
        }
    }
    if !close {
        st.browser = Some(b);
    }
}

/// Opens the file browser at `dir`, for the snapshots.
pub fn open_browser(ctx: &egui::Context, dir: &std::path::Path) {
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    st.browser = Some(Browser::open(dir));
    ctx.data_mut(|d| d.insert_temp(key, st));
}
