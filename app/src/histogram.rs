//! The histogram card of the edit screen: an overview of the light and colour balance of the photo
//! as it is being edited.
//!
//! The card floats over the top left corner of the photo. Its chips switch the display:
//! * **Lum**: the luminosity histogram, to judge exposure, contrast and clipping;
//! * **RGB**: the red, green and blue curves over each other, to judge the colour balance (a cast
//!   shows as one curve sitting left or right of the others);
//! * **R / G / B**: one channel on its own;
//! * **Parade**: the three channels side by side on a common scale.
//!
//! Under the plot a line gives the average level of each channel, and the two triangles in the
//! top corners show whether the shadows or the highlights are clipped (a tap on one shows the
//! clipped areas on the photo).

use egui::epaint::{Mesh, Vertex};
use egui::{Color32, FontId, Frame, Id, Margin, Pos2, Rect, RichText, Sense, Stroke, pos2, vec2};
use lightcraft_raster::Histogram;
use lightcraft_ui_egui::LightcraftApp;
use lightcraft_ui_egui::render::Slot;
use lightcraft_ui_egui::theme::Tokens;
use serde_json::json;

/// What the histogram card shows.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum HistMode {
    /// Luminosity (brightness) only.
    #[default]
    Luminosity,
    /// Red, green and blue over each other.
    Rgb,
    /// The red channel.
    Red,
    /// The green channel.
    Green,
    /// The blue channel.
    Blue,
    /// Red, green and blue side by side.
    Parade,
}

impl HistMode {
    /// All modes, in the order of the chips.
    const ALL: [HistMode; 6] = [HistMode::Luminosity, HistMode::Rgb, HistMode::Red, HistMode::Green, HistMode::Blue, HistMode::Parade];

    /// The label of the mode's chip.
    fn label(self) -> &'static str {
        match self {
            HistMode::Luminosity => "Lum",
            HistMode::Rgb => "RGB",
            HistMode::Red => "R",
            HistMode::Green => "G",
            HistMode::Blue => "B",
            HistMode::Parade => "Parade",
        }
    }
}

/// What the card remembers between frames.
#[derive(Clone, Default)]
pub struct HistState {
    /// The chosen display.
    pub mode: HistMode,
}

/// The colours of the three channels.
const RED: Color32 = Color32::from_rgb(235, 70, 70);
const GREEN: Color32 = Color32::from_rgb(70, 190, 110);
const BLUE: Color32 = Color32::from_rgb(80, 130, 240);

/// Height of the plot in points.
const PLOT_H: f32 = 84.0;

/// Draws the card over the top left corner of `area` (the photo's rectangle), when the photo has a
/// histogram yet. Does nothing while no render of the photo is available.
pub fn show(ctx: &egui::Context, app: &mut LightcraftApp, area: Rect, st: &mut HistState) {
    let Some(id) = app.session.active() else { return };
    // the histogram of the loupe's render, else of the photo's thumbnail
    let hist = app
        .renderer
        .textures
        .get(&Slot::Main)
        .filter(|t| t.photo == id)
        .and_then(|t| t.histogram.clone())
        .or_else(|| app.renderer.textures.get(&Slot::Thumb(id)).and_then(|t| t.histogram.clone()));
    let Some(hist) = hist else { return };

    let t = Tokens::get(ctx);
    let width = (area.width() - 32.0).clamp(220.0, 320.0);
    let clipping = hist.clipping();
    let show_clipping = app.ui.show_clipping;
    let mut toggle_clipping = false;
    egui::Area::new(Id::new("m_histogram")).fixed_pos(area.left_top() + vec2(8.0, 8.0)).order(egui::Order::Foreground).show(ctx, |ui| {
        Frame::NONE.fill(Color32::from_black_alpha(190)).corner_radius(8.0).inner_margin(Margin::same(8)).show(ui, |ui| {
            ui.set_width(width);
            // The chips that switch the display.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for mode in HistMode::ALL {
                    let selected = st.mode == mode;
                    let text = RichText::new(mode.label()).size(12.5).color(if selected { Color32::WHITE } else { t.text_dim });
                    let button = egui::Button::new(text).fill(if selected { t.accent } else { Color32::TRANSPARENT }).min_size(vec2(0.0, 30.0));
                    if ui.add(button).clicked() {
                        st.mode = mode;
                    }
                }
            });
            // The plot.
            let (plot, _) = ui.allocate_exact_size(vec2(width, PLOT_H), Sense::hover());
            let painter = ui.painter_at(plot.expand(1.0));
            painter.rect_filled(plot, 4.0, Color32::from_gray(28));
            zone_lines(&painter, plot);
            match st.mode {
                HistMode::Luminosity => curve(&painter, plot, &hist.luma, Color32::from_gray(205), peak(&[&hist.luma]), true),
                HistMode::Rgb => {
                    let top = peak(&[&hist.r, &hist.g, &hist.b]);
                    for (bins, color) in [(&hist.r, RED), (&hist.g, GREEN), (&hist.b, BLUE)] {
                        curve(&painter, plot, bins, color, top, false);
                    }
                }
                HistMode::Red => curve(&painter, plot, &hist.r, RED, peak(&[&hist.r]), true),
                HistMode::Green => curve(&painter, plot, &hist.g, GREEN, peak(&[&hist.g]), true),
                HistMode::Blue => curve(&painter, plot, &hist.b, BLUE, peak(&[&hist.b]), true),
                HistMode::Parade => {
                    let top = peak(&[&hist.r, &hist.g, &hist.b]);
                    let third = plot.width() / 3.0;
                    for (i, (bins, color)) in [(&hist.r, RED), (&hist.g, GREEN), (&hist.b, BLUE)].into_iter().enumerate() {
                        let part = Rect::from_min_size(plot.min + vec2(third * i as f32, 0.0), vec2(third - 2.0, plot.height()));
                        curve(&painter, part, bins, color, top, true);
                    }
                }
            }
            // The clipping triangles in the top corners: grey when nothing is clipped, red when it is;
            // a tap shows the clipped areas on the photo.
            for (left, clipped) in [(true, clipping.0 > 0.003), (false, clipping.1 > 0.003)] {
                let corner = if left { plot.left_top() + vec2(14.0, 14.0) } else { plot.right_top() + vec2(-14.0, 14.0) };
                let rect = Rect::from_center_size(corner, vec2(28.0, 28.0));
                let resp = ui.interact(rect, Id::new(("m_hist_clip", left)), Sense::click());
                let color = if clipped { Color32::from_rgb(235, 80, 70) } else if show_clipping { t.accent } else { Color32::from_gray(90) };
                let (a, b, c) = if left {
                    (rect.left_top() + vec2(4.0, 4.0), rect.left_top() + vec2(20.0, 4.0), rect.left_top() + vec2(4.0, 20.0))
                } else {
                    (rect.right_top() + vec2(-4.0, 4.0), rect.right_top() + vec2(-20.0, 4.0), rect.right_top() + vec2(-4.0, 20.0))
                };
                painter.add(egui::Shape::convex_polygon(vec![a, b, c], color, Stroke::NONE));
                if resp.clicked() {
                    toggle_clipping = true;
                }
            }
            // The average level of each channel, in percent: a quick check of the colour balance.
            ui.add_space(4.0);
            let line = if matches!(st.mode, HistMode::Luminosity) {
                format!("Lum {:.0}%    shadows {} / highlights {}", mean(&hist.luma), clip_text(clipping.0), clip_text(clipping.1))
            } else {
                format!("R {:.0}%   G {:.0}%   B {:.0}%", mean(&hist.r), mean(&hist.g), mean(&hist.b))
            };
            ui.label(RichText::new(line).font(FontId::proportional(12.0)).color(t.text_dim));
        });
    });
    if toggle_clipping {
        let _ = app.run("view.clipping", json!({}));
    }
}

/// "clipped 1.2%" or "ok": the share of clipped pixels.
fn clip_text(fraction: f32) -> String {
    if fraction > 0.003 { format!("{:.1}%", fraction * 100.0) } else { "ok".to_string() }
}

/// The average level (0..100 %) of a 256-bin histogram.
fn mean(bins: &[u32]) -> f32 {
    let total: f64 = bins.iter().map(|&b| f64::from(b)).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let sum: f64 = bins.iter().enumerate().map(|(i, &b)| i as f64 * f64::from(b)).sum();
    (sum / total / 255.0 * 100.0) as f32
}

/// The tallest smoothed bin of the given histograms, ignoring the two outermost bins at each end
/// (where clipped pixels pile up and would flatten everything else).
fn peak(all: &[&Vec<u32>]) -> f32 {
    all.iter().flat_map(|bins| smooth(bins).into_iter().skip(2).take(252)).fold(1.0f32, f32::max)
}

/// The bins averaged over five neighbours, so the curve is smooth.
fn smooth(bins: &[u32]) -> Vec<f32> {
    let n = bins.len();
    (0..n)
        .map(|i| {
            let (a, b) = (i.saturating_sub(2), (i + 3).min(n));
            let slice = bins.get(a..b).unwrap_or(&[]);
            slice.iter().map(|&x| x as f32).sum::<f32>() / slice.len().max(1) as f32
        })
        .collect()
}

/// Faint vertical lines that divide the tonal range into five zones (blacks, shadows, mid-tones,
/// highlights, whites).
fn zone_lines(painter: &egui::Painter, plot: Rect) {
    for k in 1..5 {
        let x = plot.left() + plot.width() * k as f32 / 5.0;
        painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(1.0, Color32::from_white_alpha(22)));
    }
}

/// One channel's curve inside `plot`; with `fill` the area under it is filled too. The height
/// follows the square root of the count, so small populations stay visible.
fn curve(painter: &egui::Painter, plot: Rect, bins: &[u32], color: Color32, top: f32, fill: bool) {
    let values = smooth(bins);
    let n = values.len().max(2);
    let to = |i: usize, v: f32| -> Pos2 {
        let x = plot.left() + plot.width() * i as f32 / (n - 1) as f32;
        pos2(x, plot.bottom() - (plot.height() - 2.0) * (v / top).max(0.0).sqrt().min(1.0))
    };
    if fill {
        let mut mesh = Mesh::default();
        let tint = color.gamma_multiply(0.35);
        for (i, v) in values.iter().enumerate() {
            let up = to(i, *v);
            mesh.vertices.push(Vertex { pos: up, uv: Pos2::ZERO, color: tint });
            mesh.vertices.push(Vertex { pos: pos2(up.x, plot.bottom()), uv: Pos2::ZERO, color: tint });
            if i > 0 {
                let k = i as u32 * 2;
                mesh.indices.extend([k - 2, k - 1, k, k - 1, k + 1, k]);
            }
        }
        painter.add(mesh);
    }
    let points: Vec<Pos2> = values.iter().enumerate().map(|(i, v)| to(i, *v)).collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.6, color)));
}
