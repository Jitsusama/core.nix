//! The glow at the page's head, drawn by the reader rather than by Typst.
//! Typst samples a gradient's colour for every pixel of the whole shape,
//! mixing in Oklab each time, before it draws any of it, and each band the
//! glow crosses did that again on its own thread: a third of a second of
//! the first frame's wait. Here each band works out only its own rows, and
//! the colour along the gradient comes from a table, as the glow's colour
//! hangs only on how far a pixel stands from its centre.

use std::sync::Arc;

use typst::layout::{Frame, FrameItem, Point, Ratio, Size};
use typst::visualize::{
    Color, Geometry, Gradient, Paint, RadialGradient, RatioOrAngle, RelativeTo,
};

/// A band's glow, taken out of its frame: a rect filled with a radial
/// gradient relative to itself, where the band's frame places it.
pub struct Glow {
    radial: Arc<RadialGradient>,
    table: Vec<[f32; 4]>,
    at: Point,
    size: Size,
}

/// How many steps the table holds along the gradient: colour changes so
/// little across one that it rounds to the same 8 bits as Typst's own mix,
/// and a stop at a half, a quarter or an eighth falls on a step.
const STEPS: u32 = 1024;

/// Takes the first radial glow out of a band's frame, searching groups that
/// neither move nor clip what they hold, so where it lands is plain.
pub fn lift(frame: &mut Frame) -> Option<Glow> {
    // Taking an item out copies what the frame shares, so a band the glow
    // doesn't cross is only looked through.
    holds(frame)
        .then(|| lift_at(frame, Point::zero()))
        .flatten()
}

fn holds(frame: &Frame) -> bool {
    frame.items().any(|(_, item)| match item {
        FrameItem::Shape(shape, _) => glow_of(shape).is_some(),
        FrameItem::Group(g) => g.clip.is_none() && g.transform.is_identity() && holds(&g.frame),
        _ => false,
    })
}

fn lift_at(frame: &mut Frame, offset: Point) -> Option<Glow> {
    // The glows and plain groups in the frame's order, which is the order
    // Typst draws in, tried in turn until one holds a glow.
    let found: Vec<_> = frame
        .items()
        .enumerate()
        .filter_map(|(i, (pos, item))| match item {
            FrameItem::Shape(shape, _) => glow_of(shape).map(|g| (i, offset + *pos, Some(g))),
            FrameItem::Group(g) if g.clip.is_none() && g.transform.is_identity() => {
                Some((i, offset + *pos, None))
            }
            _ => None,
        })
        .collect();
    for (i, at, shape) in found {
        let mut index = 0;
        let mut glow = None;
        frame.retain(|item| {
            let this = index == i;
            index += 1;
            match (this, &shape, item) {
                (true, Some((radial, size)), _) => {
                    glow = Some(Glow::new(Arc::clone(radial), at, *size));
                    false
                }
                (true, None, FrameItem::Group(g)) => {
                    glow = lift_at(&mut g.frame, at);
                    true
                }
                _ => true,
            }
        });
        if glow.is_some() {
            return glow;
        }
    }
    None
}

/// A shape is a glow the reader draws when it's a rect, unstroked, filled
/// opaquely with a radial gradient sized by the rect itself.
fn glow_of(shape: &typst::visualize::Shape) -> Option<(Arc<RadialGradient>, Size)> {
    let Geometry::Rect(size) = shape.geometry else {
        return None;
    };
    let Some(Paint::Gradient(gradient @ Gradient::Radial(radial))) = &shape.fill else {
        return None;
    };
    let opaque = radial
        .stops
        .iter()
        .all(|(c, _)| c.alpha().is_some_and(|a| a >= 1.0));
    let plain = shape.stroke.is_none()
        && gradient.unwrap_relative(false) == RelativeTo::Self_
        && size.x.to_pt() > 0.0
        && size.y.to_pt() > 0.0;
    (opaque && plain).then(|| (Arc::clone(radial), size))
}

/// A colour as Typst hands it to the canvas: its sRGB components.
pub fn components(color: &Color) -> [f32; 4] {
    let c = color.to_process().to_rgb();
    [c.red, c.green, c.blue, c.alpha]
}

impl Glow {
    fn new(radial: Arc<RadialGradient>, at: Point, size: Size) -> Self {
        let gradient = Gradient::Radial(Arc::clone(&radial));
        let table = (0..=STEPS)
            .map(|i| {
                let t = Ratio::new(f64::from(i) / f64::from(STEPS));
                components(&gradient.sample(RatioOrAngle::Ratio(t)))
            })
            .collect();
        Glow {
            radial,
            table,
            at,
            size,
        }
    }

    /// Where along the gradient a point of the rect falls, both as
    /// fractions of its width and height, as Typst works it out.
    fn t(&self, x: f64, y: f64) -> f64 {
        let r = &self.radial;
        let (cx, cy) = (r.center.x.get(), r.center.y.get());
        let (qx, qy) = (r.focal_center.x.get(), r.focal_center.y.get());
        let (cr, fr) = (r.radius.get(), r.focal_radius.get());
        let (zqx, zqy) = (x - qx, y - qy);
        let zq = (zqx * zqx + zqy * zqy).sqrt();
        let zp = ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt();
        if zq < fr {
            return 0.0;
        }
        if zp > cr {
            return 1.0;
        }
        let (ux, uy) = (zqx / zq, zqy / zq);
        let (qpx, qpy) = (qx - cx, qy - cy);
        let az = qpx * ux + qpy * uy;
        let rho = cr * cr - (qpx * qpx + qpy * qpy);
        let bz = (az * az + rho).sqrt() - az;
        // At the very centre the direction is undefined, and Typst's mix
        // then falls to its first stop.
        let t = (zq - fr) / (bz - fr);
        if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) }
    }

    /// The glow's colour at a point along it, premultiplied in 8 bits as
    /// the canvas holds it.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "t is within 0 and 1, so its step is within the table"
    )]
    fn color(&self, t: f64) -> tiny_skia::PremultipliedColorU8 {
        let at = t * f64::from(STEPS);
        let i = (at.floor() as usize).min(STEPS as usize - 1);
        #[expect(clippy::cast_precision_loss, reason = "a step is a thousand at most")]
        let frac = (at - i as f64) as f32;
        let (a, b) = (self.table[i], self.table[i + 1]);
        let mix = |k: usize| a[k] + (b[k] - a[k]) * frac;
        tiny_skia::Color::from_rgba(mix(0), mix(1), mix(2), mix(3))
            .unwrap_or(tiny_skia::Color::BLACK)
            .premultiply()
            .to_color_u8()
    }

    /// Paints the glow onto a band's canvas, drawn at `scale` device pixels
    /// a point, pixel for pixel as Typst would: the rect covers the pixels
    /// whose centres fall inside it, and each takes the gradient's sample
    /// nearest it, the gradient sampled once a device pixel of the rect.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a band and its glow are whole pixels within a few thousand"
    )]
    pub fn paint(&self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        let (w_pt, h_pt) = (self.size.x.to_pt() as f32, self.size.y.to_pt() as f32);
        let floor = scale.ceil() as u32;
        let w = ((w_pt * scale).ceil() as u32).max(floor);
        let h = ((h_pt * scale).ceil() as u32).max(floor);
        let (ox, oy) = (
            self.at.x.to_pt() as f32 * scale,
            self.at.y.to_pt() as f32 * scale,
        );
        let (cw, ch) = (i64::from(canvas.width()), i64::from(canvas.height()));
        let cols = (ox.round() as i64).max(0)..((ox + w_pt * scale).round() as i64).min(cw);
        let rows = (oy.round() as i64).max(0)..((oy + h_pt * scale).round() as i64).min(ch);
        if cols.is_empty() {
            return;
        }
        // Typst draws its samples with the nearest one to a pixel's centre.
        let nearest = |d: i64, o: f32, n: u32| {
            ((d as f32 + 0.5 - o).floor() as i64).clamp(0, i64::from(n) - 1) as f32 / n as f32
        };
        let xs: Vec<f64> = cols.clone().map(|c| f64::from(nearest(c, ox, w))).collect();
        let pixels = canvas.pixels_mut();
        for row in rows {
            let y = f64::from(nearest(row, oy, h));
            let start = (row * cw + cols.start) as usize;
            for (pixel, &x) in pixels[start..start + xs.len()].iter_mut().zip(&xs) {
                *pixel = self.color(self.t(x, y));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use typst::layout::{Frame, FrameItem};
    use typst::visualize::{Gradient, Paint};

    use crate::page::Setter;

    fn page() -> typst_layout::Page {
        let dir = std::env::temp_dir().join(format!("reader-glow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, "# A Title\n\nA paragraph under it.\n").expect("the file");
        let (page, _) = Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        page
    }

    fn gradients(frame: &Frame) -> usize {
        frame
            .items()
            .map(|(_, item)| match item {
                FrameItem::Shape(s, _) => {
                    usize::from(matches!(s.fill, Some(Paint::Gradient(Gradient::Radial(_)))))
                }
                FrameItem::Group(g) => gradients(&g.frame),
                _ => 0,
            })
            .sum()
    }

    #[test]
    fn the_head_band_gives_up_its_glow_for_the_reader_to_draw() {
        let page = page();
        let mut band = crate::tiles::band(&page, 0.0, 192.0);
        assert_eq!(gradients(&band), 1, "the page's head holds one glow");
        let glow = super::lift(&mut band);
        assert!(glow.is_some(), "no glow was taken from the head band");
        assert_eq!(gradients(&band), 0, "the glow was left for Typst to sample");
    }
}
