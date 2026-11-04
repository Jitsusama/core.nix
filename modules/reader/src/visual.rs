//! Visual mode in the window: v and V start a selection with the theme's
//! cursor on the first word in view, the keys move it and its text objects
//! take what's round it (select.rs), and y or Ctrl+C copies it as the
//! file's markdown through wl-copy, flashing what it took as his Neovim's
//! yank does. At rest the page has no cursor, so y y and y i p copy the
//! first paragraph, item or line of code wholly in view, or its paragraph.
//! The selection's ground is drawn over the page's bands, as search's
//! marks are, recolouring the page under it in one pass.

use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};

use crate::keys::Act;
use crate::select::{Kind, Motion, Selection};
use crate::{App, FLASH, LINE_PT, SCROLL_OFF_LINES, baseline, gl, keys_logged, prompt, select};

impl App {
    /// The view the cursor keeps to, its top and foot in points down the
    /// page: where the view is heading, above the prompt's row while the
    /// prompt shows; the lines H, M and L pick from, and the page's cursor
    /// stays among.
    pub(crate) fn view_pt(&self) -> (f64, f64) {
        let px = self.px_per_pt();
        let top = self.heading() / px;
        let prompt = if self.prompt_shown {
            prompt::ROW_PT * self.prompt_px_per_pt()
        } else {
            0.0
        };
        (top, top + (self.screen_px() - prompt) / px)
    }

    /// Reads the page's words for selecting on a thread of its own, with
    /// the file as it is now.
    pub(crate) fn read_layout(&self) {
        let page = self.painter.page();
        let version = self.painter.version;
        let file = self.file.clone();
        let to = self.layout_to.clone();
        let _ = std::thread::Builder::new()
            .name("layout".into())
            .spawn(move || {
                let source = std::fs::read_to_string(&file).unwrap_or_default();
                let _ = to.send((version, select::Layout::read(&page.frame, source)));
            });
    }

    /// The page's words for selecting have been read. A selection holds
    /// while its lines are the same on the new page, and ends otherwise.
    pub(crate) fn layout_read(&mut self, version: u32, layout: select::Layout) {
        if version != self.painter.version {
            return;
        }
        if let (Some(selection), Some(old)) = (&self.selection, &self.layout) {
            let (from, to) = selection.ends();
            if !(layout.same_line(old, from.line) && layout.same_line(old, to.line)) {
                self.end_selection();
            }
        }
        self.layout = Some(Arc::new(layout));
    }

    fn end_selection(&mut self) {
        if self.selection.take().is_some() && keys_logged() {
            eprintln!("selected: nothing");
        }
    }

    /// Does what a key asks of a selection, or starts one or copies at
    /// rest; false for a key that does what it does at rest.
    pub(crate) fn select_act(&mut self, act: Act, count: Option<u32>) -> bool {
        let times = count.map_or(1, |n| usize::try_from(n).unwrap_or(usize::MAX));
        let Some(layout) = self.layout.clone() else {
            return matches!(
                act,
                Act::Select | Act::SelectPieces | Act::CopyPiece | Act::CopyParagraph
            );
        };
        let (top, foot) = self.view_pt();
        match act {
            Act::Select | Act::SelectPieces => {
                let kind = if act == Act::Select {
                    Kind::Chars
                } else {
                    Kind::Pieces
                };
                match self.selection.as_ref().map(|selection| selection.kind) {
                    Some(now) if now == kind => self.end_selection(),
                    Some(_) => {
                        if let Some(selection) = self.selection.as_mut() {
                            selection.kind = kind;
                        }
                    }
                    None => {
                        self.selection = layout
                            .first_in_view(top, foot)
                            .map(|at| Selection::new(kind, at));
                    }
                }
                return true;
            }
            Act::CopyPiece | Act::CopyParagraph => {
                let taken = layout.first_piece_in_view(top, foot).and_then(|unit| {
                    if act == Act::CopyPiece {
                        layout.pieces_from(unit, times)
                    } else {
                        layout.paragraph_of(unit)
                    }
                });
                if let Some(taken) = taken {
                    self.copy(&layout, taken);
                }
                return true;
            }
            _ => {}
        }
        let Some(selection) = self.selection.as_mut() else {
            return false;
        };
        let motion = match act {
            Act::Down => Motion::Down,
            Act::Up => Motion::Up,
            Act::Left => Motion::Left,
            Act::Right => Motion::Right,
            Act::WordOn => Motion::WordOn,
            Act::WordBack => Motion::WordBack,
            Act::WordEnd => Motion::WordEnd,
            Act::LineStart => Motion::LineStart,
            Act::LineEnd => Motion::LineEnd,
            Act::ParagraphBack => Motion::ParagraphBack,
            Act::ParagraphOn => Motion::ParagraphOn,
            Act::First => Motion::First,
            Act::Last => Motion::Last,
            Act::OtherEnd => {
                selection.swap();
                self.follow_cursor();
                return true;
            }
            Act::Object(object) => {
                layout.object(selection, object);
                self.follow_cursor();
                return true;
            }
            Act::Copy => {
                let taken = selection.clone();
                self.end_selection();
                self.copy(&layout, taken);
                return true;
            }
            Act::Rest => {
                self.end_selection();
                return true;
            }
            _ => return false,
        };
        layout.moved(selection, motion, times);
        self.follow_cursor();
        true
    }

    /// After a key that moved the view while selecting: the cursor keeps to
    /// it, or for a search's n and N goes to the match, as Neovim's does.
    pub(crate) fn after_view_moved(&mut self, to_match: bool) {
        let (Some(layout), true) = (self.layout.clone(), self.selection.is_some()) else {
            return;
        };
        let matched = self
            .current
            .filter(|_| to_match)
            .and_then(|i| self.found.get(i))
            .and_then(|found| found.pieces.first())
            .and_then(|piece| layout.place_of(piece.baseline_pt, piece.from_pt));
        let (top, foot) = self.view_pt();
        if let Some(selection) = self.selection.as_mut() {
            match matched {
                Some(at) => selection.put(at),
                None => layout.keep_in_view(selection, top, foot),
            }
        }
    }

    /// Moves the view to keep the cursor in it, four lines from its edge
    /// as his Neovim's scrolloff: at once for a step, as a jump for
    /// further than half a screen.
    fn follow_cursor(&mut self) {
        let (Some(layout), Some(selection)) = (self.layout.as_ref(), self.selection.as_ref())
        else {
            return;
        };
        let (top, foot) = layout.line_extent(selection.cursor.line);
        let (view_top, view_foot) = self.view_pt();
        let room = SCROLL_OFF_LINES * LINE_PT;
        let to_pt = if top - room < view_top {
            top - room
        } else if foot + room > view_foot {
            view_top + foot + room - view_foot
        } else {
            return;
        };
        let to = to_pt * self.px_per_pt();
        if (to - self.heading()).abs() > self.screen_px() / 2.0 {
            self.jump_to(to);
        } else {
            self.step_to(to);
        }
    }

    /// Copies a selection as the file's markdown, on a thread, through
    /// wl-copy, and flashes what it took for a moment.
    fn copy(&mut self, layout: &Arc<select::Layout>, taken: Selection) {
        let pieces = layout.pieces(&taken);
        if keys_logged() {
            eprintln!("flash: {} lines for {} ms", pieces.len(), FLASH.as_millis());
        }
        self.flash = Some((pieces, Instant::now() + FLASH));
        let flashed =
            self.loop_handle
                .insert_source(Timer::from_duration(FLASH), |_, (), app: &mut App| {
                    if app.flash.take().is_some() && keys_logged() {
                        eprintln!("flash: over");
                    }
                    if !app.frame_pending {
                        app.draw();
                    }
                    TimeoutAction::Drop
                });
        if let Err(error) = flashed {
            eprintln!("reader: can't time the copy's flash: {error}");
        }
        let layout = Arc::clone(layout);
        let copying = std::thread::Builder::new()
            .name("copy".into())
            .spawn(move || {
                let text = layout.copied(&taken);
                match put_on_clipboard(&text) {
                    Ok(()) if keys_logged() => eprintln!("copied: {text:?}"),
                    Ok(()) => {}
                    Err(error) => eprintln!("reader: can't copy: {error}"),
                }
            });
        if let Err(error) = copying {
            eprintln!("reader: can't copy: {error}");
        }
    }

    /// The selection, and what a copy took while it flashes, on the
    /// selection's ground with their words in strong; then the cursor, a
    /// block on its character.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "points and pixels as f32, as GL takes them, within a page's few hundred thousand"
    )]
    pub(crate) fn draw_selection(&self, gl: &gl::Gl, screen: f32) {
        let (false, Some([chosen, cursor])) = (baseline(), self.chosen.as_ref()) else {
            return;
        };
        let px = self.px_per_pt();
        let offset = self.offset;
        let left = f64::from(self.page_x());
        let mark = |(swatch, blend): &(gl::Texture, gl::Blend), ground: (f64, f64, f64, f64)| {
            let (top, foot, from, to) = ground;
            let y = top * px - offset;
            let height = (foot - top) * px;
            if y + height < 0.0 || y > f64::from(screen) {
                return;
            }
            gl.mark(
                swatch,
                blend,
                (left + from * px) as f32,
                y as f32,
                ((to - from) * px) as f32,
                height as f32,
            );
        };
        let mut pieces = Vec::new();
        if let (Some(layout), Some(selection)) = (self.layout.as_ref(), self.selection.as_ref()) {
            pieces = layout.pieces(selection);
        }
        if let Some((flashed, until)) = &self.flash
            && Instant::now() < *until
        {
            pieces.extend(flashed.iter().copied());
        }
        for ground in select::grounds(&pieces) {
            mark(chosen, ground);
        }
        if let (Some(layout), Some(selection)) = (self.layout.as_ref(), self.selection.as_ref())
            && let Some(at) = layout.cursor(selection)
        {
            let top = at.baseline_pt - at.size_pt * 0.92;
            mark(cursor, (top, top + at.size_pt * 1.2, at.from_pt, at.to_pt));
        }
    }

    /// Says what the selection holds after a key, for the key log.
    pub(crate) fn log_selection(&self) {
        if let (true, Some(layout), Some(selection)) =
            (keys_logged(), self.layout.as_ref(), self.selection.as_ref())
        {
            eprintln!(
                "selected: {:?} {:?}",
                selection.kind,
                layout.shown(selection)
            );
        }
    }
}

/// Puts text on the clipboard through wl-copy, which keeps offering it once
/// the reader has closed, as a copy from Neovim stays. `READER_COPY` names
/// it; the package sets it to its own.
fn put_on_clipboard(text: &str) -> std::io::Result<()> {
    let program = std::env::var_os("READER_COPY").unwrap_or_else(|| "wl-copy".into());
    let mut child = std::process::Command::new(program)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("wl-copy ended {status}")))
    }
}
