//! A band of the one tall page, drawn as a screen-sized tile: only what
//! crosses the band is drawn, so a tile costs what it shows, not the page.

use typst::layout::{Abs, Frame, FrameItem, FrameKind, Point, Size};
use typst_layout::Page;

/// How far ink may stand past the frame of a group it's set in, which ends
/// at its last line's baseline and starts at its first line's capitals:
/// more than the page's title, its largest type at about 52 pt, hangs its
/// descenders (a quarter em) under its baseline or lifts its accents over
/// its capitals.
const OVERHANG_PT: f64 = 24.0;

fn extent(pos: Point, item: &FrameItem) -> (f64, f64) {
    let y = pos.y.to_pt();
    match item {
        // A clipped group draws nothing past its frame; any other may.
        FrameItem::Group(g) if g.clip.is_some() => (y, y + g.frame.size().y.to_pt()),
        FrameItem::Group(g) => (y - OVERHANG_PT, y + g.frame.size().y.to_pt() + OVERHANG_PT),
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

#[cfg(test)]
mod tests {
    use crate::page::Setter;

    /// The device rows of a tile that hold ink: a pixel standing well off
    /// the ground at its row's left edge, where no words reach.
    fn inked_rows(tile: &tiny_skia::Pixmap) -> Vec<u32> {
        let width = tile.width();
        let pixels = tile.pixels();
        (0..tile.height())
            .filter(|y| {
                let row = &pixels[(y * width) as usize..((y + 1) * width) as usize];
                let ground = row[0];
                row.iter().any(|p| {
                    let far = |a: u8, b: u8| a.abs_diff(b) > 64;
                    far(p.red(), ground.red())
                        || far(p.green(), ground.green())
                        || far(p.blue(), ground.blue())
                })
            })
            .collect()
    }

    #[test]
    fn a_band_starting_under_a_baseline_draws_the_descenders_hanging_into_it() {
        // A paragraph's frame ends at its last baseline, and its g, p and
        // comma hang below that. The reader's own shots cut them where a
        // band started a pixel under the page's last baseline.
        let dir = std::env::temp_dir().join(format!("reader-tiles-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, "Another, and another, a paragraph of its own.\n").expect("the file");
        let (page, _) = Setter::new(file.to_str().expect("a path"))
            .set()
            .expect("set");
        std::fs::remove_dir_all(&dir).expect("cleaned");
        let runs = crate::text::runs(&page.frame);
        let last = runs
            .iter()
            .filter(|run| run.chars.iter().any(|c| c.0 == 'g'))
            .map(|run| run.baseline_pt)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(last.is_finite(), "no line holding a g was set");
        let scale = 2.0_f32;
        // A whole device row under the baseline, so the tile's rows are
        // the page's own.
        let top = ((last * f64::from(scale)).ceil() + 1.0) / f64::from(scale);
        let tile = super::draw(&page, top, 12.0, scale);
        let inked = inked_rows(&tile);
        assert!(
            inked.first() == Some(&0),
            "a band starting under the baseline at {last} pt drew no descenders: ink on rows {inked:?}"
        );
    }
}
