//! Pictures: one beside the page put in the world at its own size, and
//! one that can't be read, or is on the web, said in its place.

use std::fmt::Write;

use markup5ever_rcdom::Handle;

use super::{Walker, attr, caption_of, literal, picture_kind, tag};

impl Walker<'_> {
    /// A picture: a file beside the page is put in the world and set at its
    /// own size, as GitHub shows it, no wider than the measure. One that
    /// can't be read, or is on the web, says what it shows in its place.
    pub(super) fn picture(&mut self, node: &Handle) {
        let alt = attr(node, "alt").unwrap_or_default();
        let src = attr(node, "src").unwrap_or_default();
        let given_w =
            attr(node, "width").and_then(|w| w.trim_end_matches("px").parse::<f64>().ok());
        let given_h =
            attr(node, "height").and_then(|h| h.trim_end_matches("px").parse::<f64>().ok());
        if let Some((path, w, h)) = self.local_picture(&src) {
            // CSS pixels are 0.75 pt; a width the page gives wins, and a
            // height alone scales the width with it.
            let width = match (given_w, given_h, w, h) {
                (Some(gw), _, _, _) => Some(gw),
                (None, Some(gh), Some(w), Some(h)) if h > 0.0 => Some(gh * w / h),
                (None, _, w, _) => w,
            };
            let width = width.map_or("none".to_string(), |w| format!("{:.1}pt", w * 0.75));
            self.caption = Some(alt.clone());
            self.set_picture(
                node,
                &format!("#pic({}, {}, {})", literal(&path), literal(&alt), width),
            );
            return;
        }
        self.caption = None;
        self.set_picture(node, &format!("#picture({})", literal(&alt)));
    }

    /// A picture in a paragraph stays in its line; one standing among the
    /// blocks, as an HTML picture does, is a paragraph of its own, so it
    /// keeps the page's edge and the space between paragraphs.
    fn set_picture(&mut self, node: &Handle, picture: &str) {
        let parent = node.parent.take();
        let among_blocks = parent
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .and_then(|parent| tag(&parent))
            .is_none_or(|name| {
                ["body", "div", "section", "details", "blockquote"].contains(&name.as_str())
            });
        node.parent.set(parent);
        if among_blocks {
            let caption = self.caption.take().unwrap_or_default();
            let _ = write!(self.out, "\n#alone[{picture}]{}\n", caption_of(&caption));
        } else {
            self.out.push_str(picture);
        }
    }

    /// Reads a picture beside the page into the world: its path there, named
    /// for its kind as Typst reads it, and its size in pixels where its
    /// header gives one. None for a picture on the web, a missing file, or one
    /// whose kind Typst can't set.
    fn local_picture(&mut self, src: &str) -> Option<(String, Option<f64>, Option<f64>)> {
        // A picture on the web is set once fetched; until then it's a box,
        // and it's asked for.
        if crate::web::is_web(src) {
            let bytes = crate::web::cached(src);
            if bytes.is_none() {
                crate::web::want(src);
            }
            return self.put_picture(bytes?);
        }
        let src = src.strip_prefix("file://").unwrap_or(src);
        if src.is_empty() || src.contains("://") || src.starts_with("data:") {
            return None;
        }
        let src = src.split(['?', '#']).next().unwrap_or(src);
        let path = std::path::Path::new(src);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base?.join(path)
        };
        self.put_picture(std::fs::read(&path).ok()?)
    }

    /// Puts a picture's bytes into the world: its path there, named for its
    /// kind as Typst reads it, and its size in pixels where its header
    /// gives one.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a picture's size in pixels, far below a mantissa's 52 bits"
    )]
    fn put_picture(&mut self, bytes: Vec<u8>) -> Option<(String, Option<f64>, Option<f64>)> {
        let ext = picture_kind(&bytes)?;
        let (w, h) = if ext == "svg" || ext == "pdf" {
            (None, None)
        } else {
            let size = imagesize::blob_size(&bytes).ok()?;
            (Some(size.width as f64), Some(size.height as f64))
        };
        let name = format!("/pictures/{}.{ext}", self.pictures);
        self.pictures += 1;
        self.world.put_file(&name, bytes);
        Some((name, w, h))
    }
}
