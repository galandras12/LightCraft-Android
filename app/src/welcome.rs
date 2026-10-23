//! The welcome screen shown while the library has no photos.
//!
//! It is a full-screen nature picture that is generated in code (no image asset is shipped):
//! mountains with snow caps, a small lake that mirrors them, a lavender field in the foreground
//! and a few clouds. A diagonal line splits the screen: one side shows the picture in black and
//! white, the other side in colour, like a before/after view. A large button with a small cast
//! shadow sits in the middle and asks the user to import photos.

use egui::epaint::{Mesh, Shadow, Vertex};
use egui::{Align2, Color32, ColorImage, CornerRadius, Id, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions, pos2, vec2};

/// The text on the button in the middle of the screen.
const BUTTON_TEXT: &str = "Import images before use...";

/// Button height in points.
const BUTTON_HEIGHT: f32 = 76.0;

/// Draws the welcome screen into the whole of `ui` and returns `true` when the button was tapped.
pub fn show(ui: &mut egui::Ui) -> bool {
    let rect = ui.max_rect();
    let ctx = ui.ctx().clone();
    let pictures = pictures(&ctx, rect.width() / rect.height().max(1.0));
    let painter = ui.painter_at(rect);

    // 1. The whole picture in colour.
    painter.add(cover_mesh(&pictures.color, rect, &[rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()]));

    // 2. The same picture in black and white over the left side of the diagonal. The diagonal
    //    runs through the centre of the screen, from the upper right to the lower left.
    let centre = rect.center();
    let top = pos2(centre.x + rect.width() * 0.28, rect.top());
    let bottom = pos2(centre.x - rect.width() * 0.28, rect.bottom());
    painter.add(cover_mesh(&pictures.gray, rect, &[rect.left_top(), top, bottom, rect.left_bottom()]));

    // 3. The dividing line: a soft dark edge under a thin white line.
    painter.line_segment([top, bottom], Stroke::new(7.0, Color32::from_black_alpha(55)));
    painter.line_segment([top, bottom], Stroke::new(2.5, Color32::from_white_alpha(235)));

    // 4. The button, in the middle of the screen.
    let width = (rect.width() - 48.0).clamp(220.0, 420.0);
    let button = Rect::from_center_size(centre, vec2(width, BUTTON_HEIGHT));
    let response = ui.interact(button, Id::new("welcome-import-button"), Sense::click());
    // the button sinks a little while it is pressed
    let pressed = response.is_pointer_button_down_on();
    let body = if pressed { button.translate(vec2(0.0, 2.0)) } else { button };
    let radius = CornerRadius::same((BUTTON_HEIGHT / 2.0) as u8);
    // a small shadow cast below the button
    let shadow = Shadow { offset: [0, if pressed { 2 } else { 5 }], blur: 12, spread: 0, color: Color32::from_black_alpha(110) };
    painter.add(shadow.as_shape(body, radius));
    let fill = if response.hovered() || pressed { Color32::from_rgb(52, 126, 236) } else { Color32::from_rgb(38, 110, 222) };
    painter.rect_filled(body, radius, fill);
    painter.rect_stroke(body, radius, Stroke::new(1.0, Color32::from_white_alpha(70)), egui::StrokeKind::Inside);
    painter.text(body.center(), Align2::CENTER_CENTER, BUTTON_TEXT, egui::FontId::proportional(18.0), Color32::WHITE);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, RichText::new(BUTTON_TEXT).text()));
    response.clicked()
}

// ---------------------------------------------------------------------------------------------
// drawing the picture

/// The colour and the black-and-white texture of the picture.
#[derive(Clone)]
struct Pictures {
    color: TextureHandle,
    gray: TextureHandle,
}

/// The textures for a screen of this aspect ratio (width / height). They are made on first use,
/// kept in the egui context, and made again when the screen turns between portrait and landscape.
fn pictures(ctx: &egui::Context, aspect: f32) -> Pictures {
    let landscape = aspect >= 1.0;
    let key = Id::new(("welcome-pictures", landscape));
    if let Some(p) = ctx.data(|d| d.get_temp::<Pictures>(key)) {
        return p;
    }
    // a modest size: the picture is soft, and the GPU scales it up smoothly
    let (w, h) = if landscape { (960, (960.0 / aspect.max(1.0)) as usize) } else { ((1000.0 * aspect.max(0.3)) as usize, 1000) };
    let color = scene(w.max(64), h.max(64));
    let gray = grayscale(&color);
    let p = Pictures {
        color: ctx.load_texture("welcome-color", color, TextureOptions::LINEAR),
        gray: ctx.load_texture("welcome-gray", gray, TextureOptions::LINEAR),
    };
    ctx.data_mut(|d| d.insert_temp(key, p.clone()));
    p
}

/// A mesh that paints `texture` so that it covers `screen` (cropping what does not fit), but only
/// inside the convex polygon `outline`, whose corners are in screen coordinates.
fn cover_mesh(texture: &TextureHandle, screen: Rect, outline: &[Pos2]) -> egui::Shape {
    let [tw, th] = texture.size();
    let scale = (screen.width() / tw as f32).max(screen.height() / th as f32);
    let centre = screen.center();
    let mut mesh = Mesh::with_texture(texture.id());
    for p in outline {
        // where the point lies in the texture: the picture is centred on the screen
        let uv = pos2(0.5 + (p.x - centre.x) / (tw as f32 * scale), 0.5 + (p.y - centre.y) / (th as f32 * scale));
        mesh.vertices.push(Vertex { pos: *p, uv, color: Color32::WHITE });
    }
    // a triangle fan over the polygon
    for i in 1..outline.len() - 1 {
        mesh.add_triangle(0, i as u32, i as u32 + 1);
    }
    egui::Shape::mesh(mesh)
}

/// The picture in black and white, with a little more contrast than a plain grey conversion.
fn grayscale(image: &ColorImage) -> ColorImage {
    let pixels = image
        .pixels
        .iter()
        .map(|c| {
            let luma = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
            let v = (((luma / 255.0 - 0.5) * 1.12 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
            Color32::from_gray(v)
        })
        .collect();
    ColorImage { size: image.size, pixels, source_size: image.source_size }
}

// ---------------------------------------------------------------------------------------------
// the scene
//
// Coordinates: `x` runs from 0 (left) to 1 (right) and `y` from 0 (top) to 1 (bottom) over the
// picture. `a` is the aspect ratio (width / height); noise is sampled in `x * a` so that clouds
// and mountains keep their shape whatever the screen's proportions are.

/// Where the ground (meadow, lake and lavender) begins; above it are the sky and the mountains.
const GROUND: f32 = 0.56;

/// An RGB colour with components from 0 to 1.
type Rgb = [f32; 3];

/// Generates the picture: `width` x `height` pixels.
fn scene(width: usize, height: usize) -> ColorImage {
    let a = width as f32 / height as f32;
    let mut pixels = Vec::with_capacity(width * height);
    for j in 0..height {
        for i in 0..width {
            let x = (i as f32 + 0.5) / width as f32;
            let y = (j as f32 + 0.5) / height as f32;
            let c = if y < GROUND { backdrop(x, y, a) } else { foreground(x, y, a, i, j) };
            // a very light grain keeps the large smooth areas from banding
            let grain = (hash(i as i32, j as i32) - 0.5) * 0.018;
            pixels.push(to_color32([c[0] + grain, c[1] + grain, c[2] + grain]));
        }
    }
    ColorImage { size: [width, height], pixels, source_size: vec2(width as f32, height as f32) }
}

/// Everything above the ground line: sky, sun glow, clouds and three ranges of mountains.
fn backdrop(x: f32, y: f32, a: f32) -> Rgb {
    // sky: deep blue on top, pale blue, then a warm glow along the horizon
    let t = (y / GROUND).clamp(0.0, 1.0);
    let mut c = mix3([0.17, 0.35, 0.72], [0.52, 0.72, 0.93], smooth(0.0, 0.65, t));
    c = mix3(c, [0.99, 0.83, 0.70], smooth(0.62, 1.0, t));
    // the sun: a soft glow low on the right
    let sun = ((x - 0.78) * a).hypot(y - 0.40);
    c = mix3(c, [1.0, 0.93, 0.78], (1.0 - smooth(0.0, 0.42, sun)).powi(2) * 0.8);

    // clouds: noise stretched sideways, thinning out towards the horizon, plus one big cloud
    let n = fbm(x * a * 2.4 + 7.0, y * 6.5 + 2.0);
    let cover = smooth(0.58, 0.86, n) * (1.0 - smooth(0.10, 0.36, y)) * 0.85;
    let blob = {
        let d = (((x - 0.30) * a) / 0.34).hypot((y - 0.15) / 0.07);
        smooth(1.0, 0.25, d) * (0.55 + 0.35 * fbm(x * a * 6.0, y * 14.0))
    };
    let cloud = (cover.max(blob)).clamp(0.0, 1.0);
    // lit on top, a little blue-grey underneath
    let cloud_color = mix3([0.80, 0.84, 0.93], [1.0, 1.0, 1.0], smooth(0.4, 0.9, n));
    c = mix3(c, cloud_color, cloud * 0.92);

    // three mountain ranges, from far to near; each hides what is behind it
    let ranges: [(f32, f32, f32, f32, Rgb); 3] = [
        (0.36, 0.22, 2.3, 3.0, [0.58, 0.65, 0.82]),
        (0.45, 0.17, 3.4, 11.0, [0.34, 0.42, 0.58]),
        (0.53, 0.11, 4.8, 23.0, [0.16, 0.30, 0.28]),
    ];
    for (n, (base, amp, freq, seed, color)) in ranges.into_iter().enumerate() {
        // the noise is stretched so that there are real peaks and low saddles
        let shape = ((fbm1(x * a * freq + seed) - 0.30) * 2.2).clamp(0.0, 1.2);
        let ridge = base - amp * shape;
        if y > ridge {
            // lighter near the ridge, darker towards the foot
            let depth = smooth(0.0, 0.22, y - ridge);
            let mut m = mix3(mix3(color, [1.0, 1.0, 1.0], 0.22 * (1.0 - depth)), mix3(color, [0.0, 0.0, 0.0], 0.45), depth * 0.55);
            // snow on the high peaks of the farthest range: the higher the peak, the deeper the cap
            if n == 0 {
                let cap = ((0.27 - ridge) * 1.6).max(0.0);
                // only peaks high enough carry snow (this also keeps the smooth step below
                // from dividing by zero)
                if cap > 0.004 {
                    let ragged = cap * (0.7 + 0.6 * fbm(x * a * 40.0, y * 40.0));
                    let snow = smooth(ragged, ragged * 0.55, y - ridge);
                    m = mix3(m, [0.97, 0.98, 1.0], snow);
                }
            }
            c = m;
        }
    }
    // the strip of meadow between the last hills and the ground line
    if y > 0.54 {
        c = mix3(c, [0.33, 0.48, 0.22], smooth(0.54, GROUND, y) * 0.8);
    }
    c
}

/// Everything below the ground line: a small lake that mirrors the backdrop, and the lavender
/// field with rows that run towards a vanishing point on the horizon.
fn foreground(x: f32, y: f32, a: f32, i: usize, j: usize) -> Rgb {
    // the lake: a flat ellipse left of the middle
    let d = ((x - 0.40) / 0.36).powi(2) + ((y - 0.665) / 0.060).powi(2);
    let field = lavender(x, y, a, i, j);
    if d >= 1.0 {
        return field;
    }
    // the mirror image of the sky and the mountains, a little darker and bluer, with ripples
    let mirrored = (GROUND - (y - GROUND) * 2.2).max(0.02);
    let ripple = (y * 520.0 + fbm(x * a * 9.0, y * 3.0) * 9.0).sin() * 0.035;
    let mut water = backdrop(x, mirrored, a);
    water = mix3(water, [0.12, 0.28, 0.45], 0.35);
    water = [water[0] + ripple, water[1] + ripple, water[2] + ripple];
    // a soft bank where the water meets the field
    mix3(water, field, smooth(0.78, 1.0, d))
}

/// The lavender field: purple and green rows that widen towards the viewer.
fn lavender(x: f32, y: f32, a: f32, i: usize, j: usize) -> Rgb {
    let dy = (y - GROUND).max(0.0005);
    // the row a point belongs to: the angle seen from the vanishing point on the horizon
    let ray = (x - 0.5) / dy;
    let rows = (ray * 9.0).sin();
    let near = smooth(0.0, 0.40, dy);
    // rows far away are narrower than a pixel: they fade into haze instead of aliasing
    let detail = smooth(0.012, 0.11, dy);
    let purple = mix3([0.46, 0.27, 0.72], [0.66, 0.46, 0.90], fbm(x * a * 14.0, y * 40.0));
    let green = [0.27, 0.38, 0.20];
    let mut c = mix3(mix3(green, purple, 0.55), mix3(green, purple, smooth(-0.15, 0.25, rows)), detail);
    // flower tips: sparse bright specks on the purple rows
    if hash(i as i32 / 2, j as i32 / 2) > 0.975 - 0.02 * near && rows > 0.0 {
        c = mix3(c, [0.88, 0.76, 1.0], 0.5 * detail);
    }
    // sunlit near the lake and hazy in the distance
    c = mix3([0.78, 0.70, 0.78], c, 0.55 + 0.45 * near);
    // a little shade at the very bottom
    mix3(c, [0.0, 0.0, 0.0], smooth(0.82, 1.0, y) * 0.28)
}

// ---------------------------------------------------------------------------------------------
// small helpers

/// Linear blend of two colours: `t` = 0 gives `a`, `t` = 1 gives `b`.
fn mix3(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Smooth step from 0 at `e0` to 1 at `e1` (works for `e0 > e1` too, which flips it).
fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    if e0 == e1 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Converts a 0..1 colour to an 8-bit colour.
fn to_color32(c: Rgb) -> Color32 {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    Color32::from_rgb(q(c[0]), q(c[1]), q(c[2]))
}

/// A pseudo-random number from 0 to 1 for an integer position.
fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// Smooth value noise (0..1) on a 2D lattice.
fn noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (fx, fy) = (x - xi, y - yi);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (xi, yi) = (xi as i32, yi as i32);
    let top = hash(xi, yi) + (hash(xi + 1, yi) - hash(xi, yi)) * sx;
    let bottom = hash(xi, yi + 1) + (hash(xi + 1, yi + 1) - hash(xi, yi + 1)) * sx;
    top + (bottom - top) * sy
}

/// Fractal noise: four octaves of [`noise`], from 0 to about 1.
fn fbm(x: f32, y: f32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for _ in 0..4 {
        sum += amp * noise(x * freq, y * freq);
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / 0.9375
}

/// Fractal noise along one axis, used for the outline of a mountain range (0..1).
fn fbm1(x: f32) -> f32 {
    fbm(x, 0.37)
}
