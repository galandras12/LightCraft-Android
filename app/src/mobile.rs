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

use egui::{Align, Color32, Frame, Id, Layout, Margin, Rect, RichText, Sense, Stroke, vec2};
use lightcraft_catalog::PhotoId;
use lightcraft_develop::{ControlSpec, Section, Track, WbMode};
use lightcraft_ui_egui::LightcraftApp;
use lightcraft_ui_egui::icons::{Icon, paint};
use lightcraft_ui_egui::panels;
use lightcraft_ui_egui::panels::edit::{apply_slider_out, control};
use lightcraft_ui_egui::state::{RightPanel, ViewMode};
use lightcraft_ui_egui::theme::Tokens;
use lightcraft_ui_egui::widgets;

use crate::welcome;
use serde_json::json;

/// UI scale on top of the device's density: fingers need bigger targets than a mouse.
pub const ZOOM: f32 = 1.15;
/// Width (points, after [`ZOOM`]) from which the edit tools sit beside the photo.
const WIDE: f32 = 760.0;
/// Height of the top bar in points (the status bar inset is added on top of it).
const BAR_H: f32 = 56.0;
/// Height of the bottom tool bar (Auto, Presets, Crop, Edit, Masking, Remove) in points.
const TOOLBAR_H: f32 = 64.0;
/// Height of the row of edit tabs (Light, Color, Effects, Detail, Optics) in points.
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

/// The platform services, set once by the host at start-up ([`set_platform`]).
static PLATFORM: std::sync::OnceLock<Platform> = std::sync::OnceLock::new();

/// Installs the platform services. Only the first call has an effect.
pub fn set_platform(p: Platform) {
    let _ = PLATFORM.set(p);
}

/// The platform services; desktop defaults (no insets, storage always readable) when the host set none.
fn platform() -> &'static Platform {
    PLATFORM.get_or_init(|| Platform {
        insets_px: || [0.0; 4],
        storage_access: || true,
        request_storage_access: || {},
        storage_root: std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(|| "/".into()),
    })
}

/// Switches a `LightcraftApp` to the phone / tablet layout and hides the desktop filmstrip.
pub fn install(app: &mut LightcraftApp) {
    app.mobile_layout = Some(layout);
    app.ui.filmstrip = false;
}

/// Switches the selection mode of the library on or off, for the snapshots.
pub fn set_select_mode(ctx: &egui::Context, on: bool) {
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    st.browser = None;
    st.select_mode = on;
    ctx.data_mut(|d| d.insert_temp(key, st));
}

/// Closes the file browser and the tool sheet, for the snapshots.
pub fn close_sheets(ctx: &egui::Context) {
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    st.browser = None;
    st.tool = Tool::None;
    ctx.data_mut(|d| d.insert_temp(key, st));
}

/// Chooses the histogram display, for the snapshots: `lum`, `rgb`, `r`, `g`, `b` or `parade`.
pub fn set_histogram_mode(ctx: &egui::Context, name: &str) {
    use crate::histogram::HistMode;
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    st.hist.mode = match name {
        "rgb" => HistMode::Rgb,
        "r" => HistMode::Red,
        "g" => HistMode::Green,
        "b" => HistMode::Blue,
        "parade" => HistMode::Parade,
        _ => HistMode::Luminosity,
    };
    ctx.data_mut(|d| d.insert_temp(key, st));
}

/// Opens a tool sheet, for the snapshots: an edit tab (`light`, `color`, `effects`, `detail`, `optics`),
/// `crop` (aspect tab), `crop-geometry`, or `presets`.
pub fn open_edit_tab(ctx: &egui::Context, name: &str) {
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    st.browser = None;
    match name {
        "crop" | "crop-geometry" => {
            st.tool = Tool::Crop;
            st.crop.tab = if name == "crop" { crate::crop::CropTab::Aspect } else { crate::crop::CropTab::Geometry };
        }
        "presets" => st.tool = Tool::Presets,
        _ => {
            st.tool = Tool::Edit;
            st.tab = Tab::ALL.into_iter().find(|t| t.label().eq_ignore_ascii_case(name)).unwrap_or(Tab::Light);
        }
    }
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

/// Labels and icons of the edit tabs.
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
    /// The Crop sheet (open tab).
    crop: crate::crop::CropState,
    /// The histogram card (chosen display).
    hist: crate::histogram::HistState,
    /// The order the Add photos browser uses (kept between visits).
    browser_sort: SortKey,
    /// Selection mode of the library: a tap on a photo ticks it (as Ctrl+click does on a desktop) instead of opening it.
    select_mode: bool,
}

/// The whole screen: picks the file browser, the library grid (or welcome screen) or the edit screen,
/// and paints the system bar insets in the chrome colour. Called by the upstream UI in place of its desktop layout.
fn layout(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    // Only the grid and the single-photo view exist here; an empty library has no photo to
    // show, so it always uses the grid (where the welcome screen is).
    let empty = panels::empty_library::shown(&app.session);
    if !matches!(app.ui.view, ViewMode::PhotoGrid | ViewMode::SquareGrid | ViewMode::Detail) || (empty && app.ui.view == ViewMode::Detail) {
        let _ = app.run("view.photoGrid", json!({}));
    }
    let ppp = ctx.pixels_per_point().max(0.5);
    let [l, t, r, b] = (platform().insets_px)().map(|v| v / ppp);
    let insets = Insets { left: l, top: t, right: r, bottom: b };
    let t_ = Tokens::get(&ctx);
    // the system bars' area is painted in the chrome colour, so content never runs under it
    if insets.left > 0.5 {
        // Left cut-out / system bar (landscape): a plain strip in the chrome colour.
        egui::Panel::left("m_inset_l").exact_size(insets.left).frame(Frame::NONE.fill(t_.chrome)).show(ui, |_| {});
    }
    if insets.right > 0.5 {
        // Right cut-out / system bar (landscape).
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

/// The histogram button of the top bar: five bars of different heights, highlighted while the
/// histogram card is shown. Drawn here because the upstream icon set has no histogram icon.
fn histogram_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    if active {
        ui.painter().rect_filled(Rect::from_center_size(rect.center(), vec2(32.0, 32.0)), 4.0, t.tool_active);
    }
    let color = if active { t.text } else { t.icon };
    for (i, height) in [0.35f32, 0.65, 1.0, 0.7, 0.4].into_iter().enumerate() {
        let x = rect.center().x - 10.0 + i as f32 * 5.0;
        let bar = Rect::from_min_max(egui::pos2(x - 1.6, rect.center().y + 9.0 - 18.0 * height), egui::pos2(x + 1.6, rect.center().y + 9.0));
        ui.painter().rect_filled(bar, 1.0, color);
    }
    response
}

/// A 44 x 44 point icon button of the top bar.
fn icon_btn(ui: &mut egui::Ui, id: &str, icon: Icon, active: bool, enabled: bool, tip: &str) -> egui::Response {
    widgets::icon_button(ui, id, icon, vec2(44.0, 44.0), active, enabled, tip)
}

/// The frame (background and padding) of the top bars.
fn bar_frame(t: &Tokens) -> Frame {
    Frame::NONE.fill(t.chrome).inner_margin(Margin { left: 6, right: 6, top: 0, bottom: 0 })
}

/// The top bar. In the library it shows the title, the photo count, the Add photos (+) button and a menu;
/// in the edit screen the back arrow, the position and file name of the photo, Undo and a menu.
fn top_bar(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, detail: bool, insets: Insets) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("m_top").exact_size(BAR_H + insets.top).frame(bar_frame(&t)).show(ui, |ui| {
        ui.add_space(insets.top);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if detail {
                // Back arrow: leaves the photo and returns to the library grid.
                if icon_btn(ui, "m_back", Icon::Back, false, true, "Back to library").clicked() {
                    st.tool = Tool::None;
                    app.ui.right = RightPanel::Edit;
                    let _ = app.run("view.photoGrid", json!({}));
                }
                // Position of the active photo in the visible list ("3 of 24") and its file name.
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
                    // Overflow menu ("...").
                    let more = icon_btn(ui, "m_more", Icon::More, false, true, "More");
                    egui::Popup::menu(&more).show(|ui| detail_menu(app, ui));
                    // Undo: a text button, disabled look when there is nothing to undo.
                    let can_undo = !app.session.undo.is_empty();
                    if widgets::text_button(ui, "m_undo", "Undo", false).clicked() && can_undo {
                        let _ = app.run("edit.undo", json!({}));
                    }
                    // Histogram: shows or hides the histogram card over the photo (its chips switch the display).
                    let shown = app.ui.histogram;
                    if histogram_button(ui, shown).clicked() {
                        app.ui.histogram = !shown;
                    }
                });
            } else if st.select_mode {
                // Selection mode: a cancel button and how many photos are ticked (the actions are at the bottom).
                if icon_btn(ui, "m_sel_cancel", Icon::Close, false, true, "Cancel selection").clicked() {
                    st.select_mode = false;
                }
                let n = app.session.selection.ids.len();
                ui.label(RichText::new(format!("{n} selected")).size(17.0).strong().color(t.text));
            } else {
                ui.add_space(10.0);
                // Number of photos shown under the title.
                let total = app.session.visible_shared().1.len();
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new("LightCraft").size(18.0).strong().color(t.text));
                    ui.label(RichText::new(format!("{total} photos")).size(12.0).color(t.text_dim));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Overflow menu ("...") of the library.
                    let more = icon_btn(ui, "m_more", Icon::More, false, true, "More");
                    egui::Popup::menu(&more).show(|ui| grid_menu(app, ui));
                    // "+" opens the file browser to add photos.
                    if icon_btn(ui, "m_import", Icon::Plus, false, true, "Add photos").clicked() {
                        st.browser = Some(Browser::open(&start_dir(), Purpose::Photos, st.browser_sort));
                    }
                    // "Select" starts the selection mode: several photos can be ticked, for example to delete them.
                    if widgets::text_button(ui, "m_select", "Select", false).clicked() {
                        st.select_mode = true;
                    }
                });
            }
        });
    });
}

/// The overflow menu of the edit screen: redo, previous / next photo, reset, copy / paste edits, before / after.
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
    // Moves the photo to Recently Deleted (the file on the device is not touched).
    if ui.button("Delete photo").clicked() {
        let _ = app.run("photo.delete", json!({}));
        app.toast(ui.ctx(), "Moved to Recently Deleted");
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

/// The overflow menu of the library: grid style and Select all.
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
    // Moves the selected photos (long-press a photo to select it) to Recently Deleted.
    if ui.button("Delete selected photos").clicked() {
        let _ = app.run("photo.delete", json!({}));
        app.toast(ui.ctx(), "Moved to Recently Deleted");
        ui.close();
    }
    if ui.button("Select all").clicked() {
        let _ = app.run("library.selectAll", json!({}));
        ui.close();
    }
}

// ---------------------------------------------------------------------------------------------
// library

/// The library: the photo grid, or the welcome screen while the library is empty.
/// A tap on a photo opens it in the edit screen.
fn grid_screen(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, insets: Insets) {
    let t = Tokens::get(ui.ctx());
    if st.select_mode {
        selection_bar(app, ui, st, insets.bottom);
    } else if insets.bottom > 0.5 {
        // Keeps the grid above the navigation bar.
        egui::Panel::bottom("m_nav_pad").exact_size(insets.bottom).frame(Frame::NONE.fill(t.grid_bg)).show(ui, |_| {});
    }
    // a thumbnail about a third of the width: three columns on a phone
    let width = ui.max_rect().width();
    app.ui.thumb_size = (width / 3.0).clamp(96.0, 240.0);
    // An empty library shows the welcome screen, whose button opens the file browser.
    if panels::empty_library::shown(&app.session) {
        // The welcome screen fills the whole body.
        let tapped = egui::CentralPanel::default().frame(Frame::NONE.fill(t.grid_bg)).show(ui, welcome::show).inner;
        if tapped {
            st.browser = Some(Browser::open(&start_dir(), Purpose::Photos, st.browser_sort));
        }
        return;
    }
    let before = app.session.active();
    // The photo grid fills the whole body.
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
    // (not in selection mode, where a tap only ticks the photo)
    if st.select_mode && app.ui.view == ViewMode::Detail {
        // a quick double tap asks the grid to open the photo: stay in the library
        let _ = app.run("view.photoGrid", json!({}));
    }
    if !st.select_mode && released && pos.is_some_and(|p| rect.inner.contains(p)) && after.is_some() && (after != before || after != st.grid_active) {
        let moved = ui.input(|i| i.pointer.press_origin().zip(i.pointer.interact_pos()).is_some_and(|(a, b)| a.distance(b) > 12.0));
        if !moved && app.ui.view != ViewMode::Detail && after != st.grid_active {
            let _ = app.run("view.detail", json!({}));
        }
    }
    st.grid_active = after;
}

/// The bar at the bottom of the selection mode: Select all (or none) and Delete.
fn selection_bar(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, inset_bottom: f32) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("m_sel_bar").exact_size(TOOLBAR_H + inset_bottom).frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(ui, |ui| {
        let n = app.session.selection.ids.len();
        let total = app.session.visible_shared().1.len();
        ui.horizontal_centered(|ui| {
            ui.add_space(16.0);
            ui.spacing_mut().item_spacing.x = 10.0;
            // Select all, or none when everything is ticked already.
            let all = n > 0 && n >= total;
            if ui.add(egui::Button::new(if all { "Select none" } else { "Select all" }).min_size(vec2(120.0, 44.0))).clicked() {
                let _ = app.run(if all { "library.selectNone" } else { "library.selectAll" }, json!({}));
            }
            // Delete: moves the ticked photos to Recently Deleted (the files on the device are not touched).
            let delete = egui::Button::new(RichText::new(format!("Delete {n}")).size(16.0).color(Color32::WHITE)).fill(t.reject).min_size(vec2(ui.available_width() - 16.0, 44.0));
            if ui.add_enabled(n > 0, delete).clicked() {
                let _ = app.run("photo.delete", json!({}));
                app.toast(ui.ctx(), format!("Moved {n} photo{} to Recently Deleted", if n == 1 { "" } else { "s" }));
                st.select_mode = false;
            }
        });
    });
}

/// Called by the host for every frame's raw input. In selection mode every pointer press counts as made with
/// Ctrl held, which the upstream grid reads as "add this photo to the selection".
pub fn input_hook(ctx: &egui::Context, raw: &mut egui::RawInput) {
    let st: State = ctx.data(|d| d.get_temp(Id::new("lightcraft-mobile"))).unwrap_or_default();
    let on = st.select_mode;
    // egui keeps the modifiers it was last told about, so the change is announced once, when the mode switches
    let flag = Id::new("lightcraft-select-modifiers");
    let was: bool = ctx.data(|d| d.get_temp(flag)).unwrap_or(false);
    if on != was {
        let modifiers = if on { egui::Modifiers { ctrl: true, command: true, ..Default::default() } } else { egui::Modifiers::default() };
        raw.events.insert(0, egui::Event::ModifiersChanged(modifiers));
        ctx.data_mut(|d| d.insert_temp(flag, on));
    }
    if on {
        for event in raw.events.iter_mut() {
            if let egui::Event::PointerButton { modifiers, .. } = event {
                modifiers.ctrl = true;
                modifiers.command = true;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// edit

/// The edit screen: the photo, the tool bar at the bottom and, for the chosen tool, a sheet above it
/// (portrait) or a panel on the right (wide screens).
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
    tool_bar(ui, st, insets.bottom);
    if st.tool != Tool::None {
        if wide {
            // Wide screens: the tool sheet is a panel on the right, with the tabs on top of it.
            egui::Panel::right("m_side").exact_size(380.0).frame(Frame::NONE.fill(t.chrome)).show(ui, |ui| {
                if st.tool == Tool::Edit {
                    tab_row(ui, st);
                }
                sheet(app, ui, st);
            });
        } else {
            if st.tool == Tool::Edit {
                // Narrow screens: the tabs sit just above the tool bar...
                egui::Panel::bottom("m_tabs").exact_size(TABS_H).frame(Frame::NONE.fill(t.chrome)).show(ui, |ui| tab_row(ui, st));
            }
            // ...and the tool sheet (sliders, presets, crop...) above the tabs, about a third of the screen high.
            egui::Panel::bottom("m_sheet")
                .exact_size((height * 0.36).clamp(180.0, 420.0))
                .frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider)))
                .show(ui, |ui| sheet(app, ui, st));
        }
    }
    // The photo itself (the upstream loupe: pinch / scroll to zoom, drag to pan).
    egui::CentralPanel::default().frame(Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        let area = ui.max_rect();
        panels::detail::show(app, ui);
        // The histogram card floats over the photo's top left corner while it is switched on.
        if app.ui.histogram {
            crate::histogram::show(ui.ctx(), app, area, &mut st.hist);
        }
    });
}

/// The bottom tool bar: one button per tool; a tap opens or closes the tool sheet.
fn tool_bar(ui: &mut egui::Ui, st: &mut State, inset_bottom: f32) {
    let t = Tokens::get(ui.ctx());
    // The tool bar is always visible at the bottom, above the navigation bar inset.
    egui::Panel::bottom("m_tools").exact_size(TOOLBAR_H + inset_bottom).frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(ui, |ui| {
        let w = ui.available_width();
        let tools: [(Tool, Icon, &str); 5] = [
            (Tool::Presets, Icon::Presets, "Presets"),
            (Tool::Crop, Icon::Crop, "Crop"),
            (Tool::Edit, Icon::Sliders, "Edit"),
            (Tool::Masking, Icon::Mask, "Masking"),
            (Tool::Remove, Icon::Eraser, "Remove"),
        ];
        let cell = w / tools.len() as f32;
        let (row, _) = ui.allocate_exact_size(vec2(w, TOOLBAR_H), Sense::hover());
        for (i, (tool, icon, label)) in tools.into_iter().enumerate() {
            let r = egui::Rect::from_min_size(row.min + vec2(cell * i as f32, 0.0), vec2(cell, TOOLBAR_H));
            let resp = ui.interact(r, Id::new(("m_tool", i)), Sense::click());
            let active = tool == st.tool;
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
            // a tap opens the tool's sheet; a second tap closes it
            if resp.clicked() {
                st.tool = if st.tool == tool { Tool::None } else { tool };
            }
        }
    });
}

/// The row of tabs of the Edit tool (Light, Color, Effects, Detail, Optics); it scrolls sideways if it does not fit.
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
        Tool::Edit => edit_sliders(app, ui, st),
        Tool::Presets => panels::presets::show(app, ui),
        Tool::Crop => crate::crop::sheet(app, ui, &mut st.crop),
        Tool::Masking | Tool::Remove => panels::right::show(app, ui),
    }
}

/// The sliders of the Light tab (control ids of the engine).
const LIGHT: &[&str] = &["light.exposure", "light.contrast", "light.highlights", "light.shadows", "light.whites", "light.blacks"];
/// The parametric tone curve sliders (under "Tone curve" in the Light tab).
const CURVE: &[&str] = &["curve.highlights", "curve.lights", "curve.darks", "curve.shadows"];
/// The sliders of the Effects tab: texture, clarity, dehaze, vignette and grain.
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
/// The sliders of the Detail tab: sharpening and noise reduction.
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
/// The sliders of the Optics tab: lens corrections and defringe.
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
/// The tint slider for rendered files (same scale as [`REL_TEMP`]).
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

/// Converts a colour temperature in Kelvin to the relative -100..100 scale (mired shift around 6500 K).
fn k_to_rel(k: f64) -> f64 {
    ((1e6 / 6500.0 - 1e6 / k) / 0.8).clamp(-100.0, 100.0)
}

/// Converts the relative -100..100 temperature back to Kelvin.
fn rel_to_k(r: f64) -> f64 {
    1e6 / (1e6 / 6500.0 - r * 0.8)
}

/// The sliders of the chosen Edit tab for the active photo. A slider drag is one undo step.
fn edit_sliders(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State) {
    let tab = st.tab;
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
            Tab::Light => {
                // Auto sets exposure, contrast, highlights, shadows, whites and blacks from the photo.
                tab_actions(ui, |ui| {
                    if widgets::text_button(ui, "m_light_auto", "Auto", false).clicked() {
                        let _ = app.run("develop.auto", json!({}));
                        app.toast(ui.ctx(), "Auto light applied");
                    }
                    if widgets::text_button(ui, "m_light_reset", "Reset", false).clicked() {
                        let _ = app.run("develop.resetSection", json!({"section": "light"}));
                    }
                });
                list(app, ui, &d, LIGHT);
                // The parametric tone curve: four sliders that bend the curve in its highlight, light,
                // dark and shadow regions.
                egui::CollapsingHeader::new(RichText::new("Tone curve").size(14.0)).default_open(false).show(ui, |ui| {
                    list(app, ui, &d, CURVE);
                });
            }
            Tab::Color => {
                // White balance: a preset list, an automatic button and the black & white switch.
                tab_actions(ui, |ui| {
                    egui::ComboBox::from_id_salt("m_wb_mode").selected_text(d.wb.mode.label()).width(140.0).show_ui(ui, |ui| {
                        for mode in WbMode::ALL {
                            if mode == WbMode::Custom {
                                continue;
                            }
                            if ui.selectable_label(d.wb.mode == mode, mode.label()).clicked() {
                                let _ = app.run("develop.wb", json!({"mode": serde_json::to_value(mode).unwrap_or_default()}));
                            }
                        }
                    });
                    // Auto white balance: the engine estimates the neutral point of the photo.
                    if widgets::text_button(ui, "m_wb_auto", "Auto", d.wb.mode == WbMode::Auto).clicked() {
                        let _ = app.run("develop.wb", json!({"mode": "auto"}));
                        app.toast(ui.ctx(), "Auto white balance applied");
                    }
                    // Pick: the white balance selector. A ring appears on the photo; drag it onto something that
                    // should be white or grey and tap the check mark.
                    let picking = app.ui.tool == "wbPicker";
                    if widgets::text_button(ui, "m_wb_pick", "Pick", picking).clicked() {
                        if picking {
                            app.ui.tool.clear();
                        } else if let Err(e) = app.run("tool.wbPicker", json!({})) {
                            app.toast(ui.ctx(), e);
                        }
                    }
                    if widgets::text_button(ui, "m_bw", "B&W", lightcraft_ui_egui::is_bw(&d)).clicked() {
                        let _ = app.run("develop.treatment", json!({}));
                    }
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
            Tab::Optics => {
                lens_profile_ui(app, ui, st);
                // Automatic chromatic aberration removal: the engine measures the lateral colour fringes of
                // the picture itself; the Red/Cyan and Blue/Yellow sliders below fine-tune the estimate.
                let mut auto_ca = d.optics.remove_ca;
                ui.add_space(4.0);
                if ui.checkbox(&mut auto_ca, "Remove chromatic aberration (automatic)").changed() {
                    let _ = app.run("develop.merge", json!({"settings": {"optics": {"remove_ca": auto_ca}}, "label": "Remove CA"}));
                }
                ui.add_space(4.0);
                list(app, ui, &d, OPTICS);
            }
        }
        ui.add_space(12.0);
    });
}

/// The lens profile chooser at the top of the Optics tab: a drop-down list of the lenses of the loaded database
/// (with a filter, the lens that matches the photo first, Automatic and None) and a button that loads profile files.
fn lens_profile_ui(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State) {
    let t = Tokens::get(ui.ctx());
    // what is loaded, the lens that matches the photo and the photo's current choice (no list: it is read below, open)
    let info = app.run("optics.lensProfiles", json!({"limit": 0})).unwrap_or(serde_json::Value::Null);
    let loaded = info["loaded"].as_bool().unwrap_or(false);
    let name = |v: &serde_json::Value| v["model"].as_str().unwrap_or_default().to_string();
    let current = info["current"].as_object().map(|_| name(&info["current"]));
    let suggested = info["suggested"].as_object().map(|_| name(&info["suggested"]));
    let has_lens = app.session.active().and_then(|id| app.session.catalog.photo(id)).is_some_and(|p| p.embedded_lens.is_some());
    let label = match (&current, has_lens) {
        (Some(n), _) => n.clone(),
        (None, true) => "Profile from the file".to_string(),
        (None, false) => "None".to_string(),
    };

    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.label(RichText::new("Lens profile").size(13.0).color(t.text_dim));
    });
    ui.add_space(4.0);
    let mut chosen: Option<serde_json::Value> = None;
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        // The drop-down: it stays open while the filter is typed and closes after a choice or a tap outside.
        egui::ComboBox::from_id_salt("m_lens_profile")
            .selected_text(label)
            .width(ui.available_width() - 48.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show_ui(ui, |ui| {
                ui.set_min_width(260.0);
                if !loaded {
                    ui.label(RichText::new("No lens profiles loaded yet.").color(t.text_dim));
                    ui.label(RichText::new("Load a lensfun database (XML files) below.").size(12.0).color(t.text_dim));
                    return;
                }
                // The filter: every word must be in the maker or the model.
                let filter_id = Id::new("m_lens_filter");
                let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
                ui.add(egui::TextEdit::singleline(&mut filter).hint_text("Filter lenses...").desired_width(f32::INFINITY));
                ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
                if ui.selectable_label(current.is_none() && !has_lens, "None").clicked() {
                    chosen = Some(json!("none"));
                }
                if let Some(model) = &suggested {
                    if ui.selectable_label(false, format!("Automatic: {model}")).clicked() {
                        chosen = Some(json!("auto"));
                    }
                }
                ui.separator();
                let list = app.run("optics.lensProfiles", json!({"filter": filter, "limit": 80})).unwrap_or(serde_json::Value::Null);
                let lenses = list["lenses"].as_array().cloned().unwrap_or_default();
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for lens in &lenses {
                        let model = lens["model"].as_str().unwrap_or_default();
                        let maker = lens["maker"].as_str().unwrap_or_default();
                        let text = if model.to_lowercase().contains(&maker.to_lowercase()) || maker.is_empty() { model.to_string() } else { format!("{maker} {model}") };
                        let selected = current.as_deref() == Some(model);
                        if ui.selectable_label(selected, text).clicked() {
                            chosen = lens["id"].as_u64().map(|id| json!(id));
                        }
                    }
                    if lenses.is_empty() {
                        ui.label(RichText::new("No lens matches").color(t.text_dim));
                    } else if list["total"].as_u64().unwrap_or(0) > lenses.len() as u64 {
                        ui.label(RichText::new("Type to narrow the list...").size(12.0).color(t.text_dim));
                    }
                });
            });
    });
    if let Some(choice) = chosen {
        if let Err(e) = app.run("optics.lensProfile", json!({"id": choice})) {
            app.toast(ui.ctx(), e);
        }
    }
    ui.add_space(6.0);
    // Loading profile files: opens the file browser on the XML files.
    tab_actions(ui, |ui| {
        if widgets::text_button(ui, "m_lens_load", "Load profiles...", false).clicked() {
            st.browser = Some(Browser::open(&download_dir(), Purpose::LensProfiles, SortKey::NameAsc));
        }
        if loaded {
            let total = info["total"].as_u64().unwrap_or(0);
            ui.label(RichText::new(format!("{total} lenses loaded")).size(12.0).color(t.text_dim));
        }
    });
}

/// A row of buttons at the top of an edit tab (Auto, Reset, white balance preset...).
fn tab_actions(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.spacing_mut().item_spacing.x = 8.0;
        add(ui);
    });
    ui.add_space(8.0);
}

/// Draws one slider per control id.
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
    /// When the file was last changed (seconds since 1970; 0 when unknown): for sorting by date.
    modified: u64,
}

/// How the entries of the file browser are ordered. Folders always come first.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SortKey {
    /// Newest file first (by the date the file was changed).
    #[default]
    Newest,
    /// Oldest file first.
    Oldest,
    /// Name A to Z.
    NameAsc,
    /// Name Z to A.
    NameDesc,
    /// Largest file first.
    Largest,
    /// Smallest file first.
    Smallest,
}

impl SortKey {
    const ALL: [SortKey; 6] = [SortKey::Newest, SortKey::Oldest, SortKey::NameAsc, SortKey::NameDesc, SortKey::Largest, SortKey::Smallest];

    /// The text of the sort menu entry and of the sort button.
    fn label(self) -> &'static str {
        match self {
            SortKey::Newest => "Date: newest first",
            SortKey::Oldest => "Date: oldest first",
            SortKey::NameAsc => "Name: A to Z",
            SortKey::NameDesc => "Name: Z to A",
            SortKey::Largest => "Size: largest first",
            SortKey::Smallest => "Size: smallest first",
        }
    }
}

/// What the file browser is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Purpose {
    /// Adding photos to the library.
    Photos,
    /// Loading lens profile files (lensfun XML).
    LensProfiles,
}

impl Purpose {
    /// The title of the browser screen.
    fn title(self) -> &'static str {
        match self {
            Purpose::Photos => "Add photos",
            Purpose::LensProfiles => "Load lens profiles",
        }
    }

    /// Whether a file of this name is listed.
    fn accepts(self, name: &str) -> bool {
        match self {
            Purpose::Photos => is_photo(name),
            Purpose::LensProfiles => name.to_ascii_lowercase().ends_with(".xml"),
        }
    }
}

#[derive(Clone)]
struct Browser {
    /// What the chosen files are used for.
    purpose: Purpose,
    /// Photos are shown as a grid of thumbnails (else as a list with a small thumbnail).
    grid: bool,
    /// The order of the entries.
    sort: SortKey,
    dir: std::path::PathBuf,
    entries: Vec<Entry>,
    selected: std::collections::BTreeSet<std::path::PathBuf>,
    error: Option<String>,
}

/// Where the browser starts when loading lens profiles: the Download folder when there is one.
fn download_dir() -> std::path::PathBuf {
    let root = platform().storage_root.clone();
    let downloads = root.join("Download");
    if downloads.is_dir() { downloads } else { root }
}

/// Where the file browser starts: the camera folder (DCIM) when there is one, else the storage root.
fn start_dir() -> std::path::PathBuf {
    let root = platform().storage_root.clone();
    let dcim = root.join("DCIM");
    if dcim.is_dir() { dcim } else { root }
}

/// Whether a file name has an extension the engine can import.
fn is_photo(name: &str) -> bool {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    lightcraft_engine::import::EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(&ext))
}

/// Reading and navigating the folder of the file browser.
impl Browser {
    fn open(dir: &std::path::Path, purpose: Purpose, sort: SortKey) -> Browser {
        let mut b = Browser { purpose, grid: purpose == Purpose::Photos, sort, dir: dir.to_path_buf(), entries: vec![], selected: Default::default(), error: None };
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
                    if meta.is_dir() || self.purpose.accepts(&name) {
                        let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
                        self.entries.push(Entry { path: e.path(), name, dir: meta.is_dir(), size: meta.len(), modified });
                    }
                }
                self.sort_entries();
            }
            Err(e) => self.error = Some(format!("{}: {e}", self.dir.display())),
        }
    }

    /// Puts the entries in the chosen order: folders first, then by name, date or size. Folders have no size, so
    /// they are ordered by name when sorting by size.
    fn sort_entries(&mut self) {
        let sort = self.sort;
        self.entries.sort_by(|a, b| {
            let by_name = || a.name.to_lowercase().cmp(&b.name.to_lowercase());
            b.dir.cmp(&a.dir).then_with(|| match sort {
                SortKey::NameAsc => by_name(),
                SortKey::NameDesc => b.name.to_lowercase().cmp(&a.name.to_lowercase()),
                SortKey::Newest => b.modified.cmp(&a.modified).then_with(by_name),
                SortKey::Oldest => a.modified.cmp(&b.modified).then_with(by_name),
                SortKey::Largest if !a.dir => b.size.cmp(&a.size).then_with(by_name),
                SortKey::Smallest if !a.dir => a.size.cmp(&b.size).then_with(by_name),
                SortKey::Largest | SortKey::Smallest => by_name(),
            })
        });
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

/// A file size as text: kB below 1 MB, MB above.
fn size_text(bytes: u64) -> String {
    if bytes >= 1 << 20 { format!("{:.1} MB", bytes as f64 / (1 << 20) as f64) } else { format!("{} kB", bytes.div_ceil(1024)) }
}

/// The Add photos screen: a browser of the device storage with multi-select and an import button;
/// or, when the app has no access to the files yet, a page that asks for it.
fn browser_screen(app: &mut LightcraftApp, ui: &mut egui::Ui, st: &mut State, insets: Insets) {
    let t = Tokens::get(ui.ctx());
    let Some(mut b) = st.browser.take() else { return };
    let access = (platform().storage_access)();
    let root = platform().storage_root.clone();
    let mut close = false;
    let mut import: Option<Vec<String>> = None;
    let mut go: Option<std::path::PathBuf> = None;

    // Top bar of the browser: back / up one folder and the folder name.
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
                ui.label(RichText::new(b.purpose.title()).size(13.0).color(t.text_dim));
                let name = b.dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| b.dir.display().to_string());
                ui.add(egui::Label::new(RichText::new(name).size(16.0).color(t.text)).truncate());
            });
            // A cross at the right end leaves the browser at once, from any folder.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_btn(ui, "m_b_close", Icon::Close, false, true, "Close").clicked() {
                    close = true;
                }
            });
        });
    });

    // The row under the top bar: sort order, grid or list, Select all.
    if access {
        egui::Panel::top("m_b_tools").exact_size(52.0).frame(Frame::NONE.fill(t.chrome).inner_margin(Margin { left: 12, right: 12, top: 0, bottom: 0 })).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                // The sort order: a button that opens the list of orders (newest first, name, size...).
                let sort_button = widgets::text_button(ui, "m_b_sort", &format!("Sort: {}", b.sort.label()), false);
                egui::Popup::menu(&sort_button).show(|ui| {
                    ui.set_min_width(200.0);
                    for key in SortKey::ALL {
                        if ui.selectable_label(b.sort == key, key.label()).clicked() {
                            b.sort = key;
                            b.sort_entries();
                            // the thumbnails are asked for again in the new order
                            app.renderer.forget_imports();
                            ui.close();
                        }
                    }
                });
                // Thumbnails as a grid or as a list.
                if b.purpose == Purpose::Photos && widgets::text_button(ui, "m_b_view", if b.grid { "List" } else { "Grid" }, false).clicked() {
                    b.grid = !b.grid;
                }
                // "Select all" selects every file of the folder, or clears the selection when all are selected.
                if b.files().next().is_some() && widgets::text_button(ui, "m_b_all", "Select all", false).clicked() {
                    let all: Vec<_> = b.files().map(|e| e.path.clone()).collect();
                    if all.iter().all(|p| b.selected.contains(p)) {
                        b.selected.clear();
                    } else {
                        b.selected.extend(all);
                    }
                }
            });
        });
    }

    // Number of photos ticked in the current folder.
    let n = b.selected.len();
    if access {
        // Bottom bar: the blue Import button (the ticked photos, or the whole folder when none is ticked).
        egui::Panel::bottom("m_b_bottom").exact_size(TOOLBAR_H + insets.bottom).frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(16.0);
                // Cancel leaves the browser without importing anything.
                if ui.add(egui::Button::new(RichText::new("Cancel").size(16.0)).min_size(vec2(96.0, 44.0))).clicked() {
                    close = true;
                }
                let (noun, verb) = match b.purpose {
                    Purpose::Photos => ("photo", "Import"),
                    Purpose::LensProfiles => ("file", "Load"),
                };
                let label = if n > 0 { format!("{verb} {n} {noun}{}", if n == 1 { "" } else { "s" }) } else { format!("{verb} this folder") };
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
            ui.vertical_centered(|ui| ui.label(RichText::new(if b.purpose == Purpose::Photos { "No folders or photos here" } else { "No folders or XML files here" }).color(t.text_dim)));
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let entries = b.entries.clone();
            // The folders: one row each; a tap opens the folder.
            for e in entries.iter().filter(|e| e.dir) {
                let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::click());
                paint(ui.painter(), egui::Rect::from_center_size(egui::pos2(row.left() + 32.0, row.center().y), vec2(24.0, 24.0)), Icon::Folder, t.icon);
                ui.painter().text(egui::pos2(row.left() + 64.0, row.center().y), egui::Align2::LEFT_CENTER, &e.name, egui::FontId::proportional(15.0), t.text);
                paint(ui.painter(), egui::Rect::from_center_size(egui::pos2(row.right() - 32.0, row.center().y), vec2(18.0, 18.0)), Icon::ChevronRight, t.text_dim);
                ui.painter().line_segment([egui::pos2(row.left() + 64.0, row.bottom()), row.right_bottom()], Stroke::new(1.0, t.divider));
                if resp.clicked() {
                    go = Some(e.path.clone());
                }
            }
            // The files, with thumbnails for photos.
            let files: Vec<(usize, &Entry)> = entries.iter().enumerate().filter(|(_, e)| !e.dir).collect();
            if b.grid && b.purpose == Purpose::Photos {
                browser_grid(app, ui, &mut b, &files);
            } else {
                browser_list(app, ui, &mut b, &files);
            }
        });
    });

    if let Some(dir) = go {
        // the thumbnails belong to the folder that is left
        app.renderer.forget_imports();
        b.go(&dir);
    }
    if close || import_started(&b, close) {
        app.renderer.forget_imports();
    }
    if let Some(paths) = import {
        match b.purpose {
            // photos go to the import (the engine lists folders itself)
            Purpose::Photos => match lightcraft_ui_egui::import::start_paths(app, paths) {
                Ok(_) => {
                    app.ui.view = ViewMode::PhotoGrid;
                    close = true;
                }
                Err(e) => app.toast(ui.ctx(), e),
            },
            // lens profile files go to the lens database
            Purpose::LensProfiles => {
                let mut lenses = 0;
                let mut failure = None;
                for path in paths {
                    match app.run("optics.loadLensProfiles", json!({"path": path})) {
                        Ok(r) => lenses += r["lenses"].as_u64().unwrap_or(0),
                        Err(e) => failure = Some(e),
                    }
                }
                match failure {
                    Some(e) => app.toast(ui.ctx(), e),
                    None => {
                        app.toast(ui.ctx(), format!("Loaded {lenses} lens profiles"));
                        close = true;
                    }
                }
            }
        }
    }
    st.browser_sort = b.sort;
    if !close {
        st.browser = Some(b);
    }
}

/// Whether the browser is about to close (kept as a function so that the thumbnails are dropped on every way out).
fn import_started(_b: &Browser, close: bool) -> bool {
    close
}

/// The thumbnail of a photo file in the browser: asked for once (a raw's embedded preview, else a small render),
/// returned when it is ready. `index` is the entry's place in the folder; it picks the render slot.
fn browser_thumb(app: &mut LightcraftApp, index: usize, e: &Entry) -> Option<(egui::TextureId, [usize; 2])> {
    use lightcraft_ui_egui::render::Slot;
    let slot = Slot::Import(index as u32);
    if !app.renderer.textures.contains_key(&slot) {
        let ext = e.name.rsplit_once('.').map(|(_, x)| x.to_ascii_lowercase()).unwrap_or_default();
        const RAW: &[&str] = &["dng", "cr2", "cr3", "crw", "nef", "nrw", "arw", "sr2", "srf", "raf", "orf", "rw2", "rwl", "raw", "pef", "srw", "iiq", "mrw", "x3f", "kdc", "mos", "erf", "3fr", "fff", "mef"];
        let kind = if RAW.contains(&ext.as_str()) { lightcraft_catalog::MediaKind::Raw } else { lightcraft_catalog::MediaKind::Image };
        let candidate = lightcraft_engine::import::ImportCandidate {
            path: e.path.to_string_lossy().into_owned(),
            name: e.name.clone(),
            format: ext.to_ascii_uppercase(),
            kind,
            width: 1,
            height: 1,
            file_size: e.size,
            ..Default::default()
        };
        if let Some(job) = app.session.candidate_thumb_job(&candidate, 192, index as u64) {
            app.renderer.request_quick(slot, job, 4);
        }
    }
    app.renderer.textures.get(&slot).map(|t| (t.tex.id(), t.size))
}

/// Draws a thumbnail fitted into `rect` (the picture keeps its shape).
fn paint_thumb(ui: &egui::Ui, rect: egui::Rect, texture: Option<(egui::TextureId, [usize; 2])>) {
    ui.painter().rect_filled(rect, 4.0, Color32::from_gray(30));
    if let Some((id, [w, h])) = texture {
        let scale = (rect.width() / w.max(1) as f32).min(rect.height() / h.max(1) as f32);
        let fit = egui::Rect::from_center_size(rect.center(), vec2(w as f32 * scale, h as f32 * scale));
        ui.painter().image(id, fit, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    }
}

/// The files of the folder as rows: a small thumbnail (photos), the name and the size; a tap ticks the file.
fn browser_list(app: &mut LightcraftApp, ui: &mut egui::Ui, b: &mut Browser, files: &[(usize, &Entry)]) {
    let t = Tokens::get(ui.ctx());
    for (index, e) in files {
        let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 72.0), Sense::click());
        let chosen = b.selected.contains(&e.path);
        if chosen {
            ui.painter().rect_filled(row, 0.0, t.hover);
        }
        // the thumbnail is asked for only while the row is on screen
        let thumb_rect = egui::Rect::from_center_size(egui::pos2(row.left() + 42.0, row.center().y), vec2(56.0, 56.0));
        if b.purpose == Purpose::Photos {
            let texture = if ui.is_rect_visible(row) { browser_thumb(app, *index, e) } else { None };
            paint_thumb(ui, thumb_rect, texture);
        } else {
            paint(ui.painter(), egui::Rect::from_center_size(thumb_rect.center(), vec2(24.0, 24.0)), Icon::Tag, t.icon);
        }
        ui.painter().text(egui::pos2(row.left() + 84.0, row.center().y - 9.0), egui::Align2::LEFT_CENTER, &e.name, egui::FontId::proportional(15.0), t.text);
        ui.painter().text(egui::pos2(row.left() + 84.0, row.center().y + 11.0), egui::Align2::LEFT_CENTER, size_text(e.size), egui::FontId::proportional(12.0), t.text_dim);
        if chosen {
            paint(ui.painter(), egui::Rect::from_center_size(egui::pos2(row.right() - 32.0, row.center().y), vec2(22.0, 22.0)), Icon::Check, t.accent);
        }
        ui.painter().line_segment([egui::pos2(row.left() + 84.0, row.bottom()), row.right_bottom()], Stroke::new(1.0, t.divider));
        if resp.clicked() && !b.selected.remove(&e.path) {
            b.selected.insert(e.path.clone());
        }
    }
}

/// The photos of the folder as a grid of square thumbnails; a tap ticks the photo (a check mark and a frame).
fn browser_grid(app: &mut LightcraftApp, ui: &mut egui::Ui, b: &mut Browser, files: &[(usize, &Entry)]) {
    let t = Tokens::get(ui.ctx());
    let width = ui.available_width();
    let gap = 4.0;
    let columns = ((width / 130.0).floor() as usize).clamp(3, 8);
    let cell = (width - gap * (columns as f32 + 1.0)) / columns as f32;
    let rows = files.len().div_ceil(columns);
    let (area, _) = ui.allocate_exact_size(vec2(width, gap + rows as f32 * (cell + gap)), Sense::hover());
    for (k, (index, e)) in files.iter().enumerate() {
        let (row, column) = (k / columns, k % columns);
        let rect = egui::Rect::from_min_size(area.min + vec2(gap + column as f32 * (cell + gap), gap + row as f32 * (cell + gap)), vec2(cell, cell));
        // the thumbnail is asked for only while the cell is on screen
        if !ui.is_rect_visible(rect) {
            continue;
        }
        let texture = browser_thumb(app, *index, e);
        paint_thumb(ui, rect, texture);
        let chosen = b.selected.contains(&e.path);
        // the file name over the bottom of the cell
        let band = egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 18.0), rect.max);
        ui.painter().rect_filled(band, 0.0, Color32::from_black_alpha(130));
        ui.painter().text(band.left_center() + vec2(4.0, 0.0), egui::Align2::LEFT_CENTER, &e.name, egui::FontId::proportional(10.5), Color32::WHITE);
        if chosen {
            ui.painter().rect_stroke(rect, 4.0, Stroke::new(3.0, t.accent), egui::StrokeKind::Inside);
            let badge = egui::Rect::from_center_size(rect.right_top() + vec2(-16.0, 16.0), vec2(24.0, 24.0));
            ui.painter().circle_filled(badge.center(), 12.0, t.accent);
            paint(ui.painter(), badge.shrink(5.0), Icon::Check, Color32::WHITE);
        }
        if ui.interact(rect, Id::new(("m_b_cell", *index)), Sense::click()).clicked() && !b.selected.remove(&e.path) {
            b.selected.insert(e.path.clone());
        }
    }
}

/// Opens the file browser at `dir`, for the snapshots.
pub fn open_browser(ctx: &egui::Context, dir: &std::path::Path) {
    let key = Id::new("lightcraft-mobile");
    let mut st: State = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    st.browser = Some(Browser::open(dir, Purpose::Photos, st.browser_sort));
    ctx.data_mut(|d| d.insert_temp(key, st));
}
