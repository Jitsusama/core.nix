//! A band of the one tall page, drawn as a screen-sized tile: only what
//! crosses the band is drawn, so a tile costs what it shows, not the page.

use typst::layout::{Abs, Frame, FrameItem, FrameKind, Point, Size};
use typst_layout::Page;

fn extent(pos: Point, item: &FrameItem) -> (f64, f64) {
    let y = pos.y.to_pt();
    match item {
        FrameItem::Group(g) => (y, y + g.frame.size().y.to_pt()),
        FrameItem::Text(t) => (y - t.size.to_pt() * 1.2, y + t.size.to_pt() * 0.4),
        FrameItem::Shape(s, _) => {
            let b = s.bbox(true);
            (y + b.min.y.to_pt(), y + b.max.y.to_pt())
        }
        FrameItem::Image(_, size, _) => (y, y + size.y.to_pt()),
        _ => (y, y),
    }
}

/// Copies what crosses the band into it, stepping into a plain group taller
/// than the band rather than drawing all of it.
fn cull(frame: &Frame, offset: Point, top: f64, height: f64, band: &mut Frame) {
    for (pos, item) in frame.items() {
        let at = offset + *pos;
        let (a, b) = extent(at, item);
        if b < top || a > top + height {
            continue;
        }
        if let FrameItem::Group(g) = item {
            let plain = g.clip.is_none() && g.transform.is_identity();
            if plain && b - a > height {
                cull(&g.frame, at, top, height, band);
                continue;
            }
        }
        band.push(Point::new(at.x, at.y - Abs::pt(top)), item.clone());
    }
}

pub fn draw(page: &Page, top: f64, height: f64, scale: f32) -> tiny_skia::Pixmap {
    let width = page.frame.size().x;
    let mut band = Frame::new(Size::new(width, Abs::pt(height)), FrameKind::Hard);
    cull(&page.frame, Point::zero(), top, height, &mut band);
    let tile = Page {
        frame: band,
        bleed: page.bleed,
        fill: page.fill.clone(),
        numbering: None,
        supplement: page.supplement.clone(),
        number: page.number,
    };
    typst_render::render(
        &tile,
        &typst_render::RenderOptions {
            pixel_per_pt: f64::from(scale).into(),
            ..Default::default()
        },
    )
}
