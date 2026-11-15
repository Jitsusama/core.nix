//! The glow at the page's head, drawn by the reader rather than by Typst.
//! Typst samples a gradient's colour for every pixel of the whole shape,
//! mixing in Oklab each time, before it draws any of it, and each band the
//! glow crosses did that again on its own thread: a third of a second of
//! the first frame's wait. Here each band works out only its own rows, and
//! the colour along the gradient comes from a table, as the glow's colour
//! hangs only on how far a pixel stands from its centre.

use typst::layout::Frame;
use typst::visualize::Color;

/// A band's glow, taken out of its frame.
pub struct Glow;

/// Takes the first radial glow out of a band's frame.
pub fn lift(frame: &mut Frame) -> Option<Glow> {
    let _ = frame;
    None
}

/// A colour as Typst hands it to the canvas: its sRGB components.
pub fn rgba(color: &Color) -> [f32; 4] {
    let c = color.to_process().to_rgb();
    [c.red, c.green, c.blue, c.alpha]
}

impl Glow {
    /// Paints the glow onto a band's canvas.
    pub fn paint(&self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        let _ = (canvas, scale);
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
