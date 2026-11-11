//! Visual mode in the window: v and V start a selection with the theme's
//! block cursor on the first word of the cursor's line, the keys move it
//! and its text objects take what's round it (select/), and y or Ctrl+C
//! copies it as the file's markdown through wl-copy, flashing what it took
//! as his Neovim's yank does. At rest, y y, y i p, y a p and y } copy the
//! paragraph, item or line of code the cursor's line is in, its paragraph,
//! that with the blank line after it, or to its paragraph's end. The
//! selection's ground is drawn over the page's bands, as search's marks
//! are, recolouring the page under it in one pass, and the cursor's line
//! is marked by a rule in the margin, at rest and while selecting.

use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};

use crate::keys::Act;
use crate::select::{Kind, Motion, Object, Selection};
use crate::{
    App, FLASH, SCROLL_OFF_LINES, baseline, ex, gl, keys_logged, line_pt, moves, prompt, select,
    text,
};

/// How wide the cursor's rule is, in logical pixels: thin, a whole number
/// of device pixels at every scale.
const RULE_PX: f64 = 2.0;

/// How far the rule's right edge stands from the words' edge, in points of
/// the page: the items rung past the page's outer column, which a list's
/// numbers hang within and a spread table or code ends on, so nothing that
/// hangs out of the words comes near it.
fn rule_from_words_pt() -> f64 {
    let space = crate::theme::get().space;
    (space.outer + space.rung.items) * crate::theme::PT_A_PX
}

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

    /// Ends the selection, leaving the cursor on the line its cursor was
    /// on, as Neovim's stays where visual mode left it.
    fn end_selection(&mut self) {
        let Some(selection) = self.selection.take() else {
            return;
        };
        if let Some(layout) = self.layout.as_ref() {
            self.aim = Some(layout.line_extent(selection.cursor.line).0);
            self.selected = layout.file_lines_of(&selection);
        }
        if keys_logged() {
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
                Act::Select
                    | Act::SelectPieces
                    | Act::CopyPiece
                    | Act::CopyParagraph
                    | Act::CopyAParagraph
                    | Act::CopyToParagraphEnd
            );
        };
        let (_, foot) = self.view_pt();
        let here = self.cursor_pt();
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
                        let first = layout.first_in_view(here - 0.5, foot);
                        // On the line a search left the cursor on, v starts at
                        // the match, as Neovim's starts at the cursor's
                        // character, which the search put there.
                        let matched = self
                            .current
                            .and_then(|i| self.found.get(i))
                            .and_then(|found| found.pieces.first())
                            .and_then(|piece| layout.place_of(piece.baseline_pt, piece.from_pt))
                            .filter(|at| first.is_some_and(|first| first.line == at.line));
                        self.selection = matched.or(first).map(|at| Selection::new(kind, at));
                    }
                }
                return true;
            }
            Act::CopyPiece | Act::CopyParagraph | Act::CopyAParagraph | Act::CopyToParagraphEnd => {
                self.copy_at_rest(&layout, act, times);
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
            // y leaves the cursor at the selection's start, as Vim's does.
            Act::Copy => {
                let taken = selection.clone();
                self.end_selection();
                let (top, foot) = layout.line_extent(taken.ends().0.line);
                self.aim = Some(top);
                self.follow_line(top, foot, None);
                self.copy(&layout, taken);
                return true;
            }
            Act::Rest => {
                self.end_selection();
                return true;
            }
            // A mark made while selecting is the cursor's line, as his
            // Neovim's is, and the selection holds.
            Act::SetMark(letter) => {
                let (top, _) = layout.line_extent(selection.cursor.line);
                self.marks.insert(letter, top);
                return true;
            }
            _ => return false,
        };
        layout.moved(selection, motion, times);
        self.follow_cursor();
        true
    }

    /// y y, y i p, y a p or y } at rest, from the cursor's line. As Vim's
    /// yank, the cursor goes to the start of what it took when that's above
    /// it, as y i p's is from inside.
    fn copy_at_rest(&mut self, layout: &Arc<select::Layout>, act: Act, times: usize) {
        let (_, foot) = self.view_pt();
        let here = self.cursor_pt();
        let taken = layout.piece_at(here, foot).and_then(|unit| match act {
            Act::CopyPiece => layout.pieces_from(unit, times),
            Act::CopyParagraph => layout.paragraph_of(unit, Object::InParagraph),
            Act::CopyAParagraph => layout.paragraph_of(unit, Object::AParagraph),
            _ => layout.to_paragraph_end(unit, times),
        });
        if let Some(taken) = taken {
            let (top, foot) = layout.line_extent(taken.ends().0.line);
            if top < here - 0.5 {
                self.aim = Some(top);
                self.follow_line(top, foot, None);
            }
            self.copy(layout, taken);
        }
    }

    /// :y, a range's lines of the file as they're written, flashing what
    /// the page shows of them, and saying how many past two, as Neovim
    /// does; with none, the cursor's piece, as y y takes it.
    pub(crate) fn copy_range(&mut self, range: ex::Range) {
        let Some(layout) = self.layout.clone() else {
            return;
        };
        let (first, last) = match range {
            ex::Range::Here => return self.copy_at_rest(&layout, Act::CopyPiece, 1),
            ex::Range::Lines(first, last) => (first, last),
            ex::Range::Selected => match self.selected {
                Some(lines) => lines,
                None => return self.say(ex::NOTHING_SELECTED),
            },
        };
        let Some((taken, shown)) = layout.file_range(first, last) else {
            return self.say(ex::PAST_THE_END);
        };
        let lines = layout
            .file_lines_of(&taken)
            .map_or(0, |(first, last)| last + 1 - first);
        if shown {
            self.flash(layout.pieces(&taken));
        }
        clip(&layout, taken);
        if let Some(said) = ex::copied(lines) {
            self.say(&said);
        }
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

    /// Moves the view to keep the selection's cursor in it.
    fn follow_cursor(&mut self) {
        let (Some(layout), Some(selection)) = (self.layout.as_ref(), self.selection.as_ref())
        else {
            return;
        };
        let (top, foot) = layout.line_extent(selection.cursor.line);
        self.follow_line(top, foot, None);
    }

    /// Moves the view to keep a line from `top` to `foot` in it, four lines
    /// from its edge as his Neovim's scrolloff: from the edge it went
    /// toward, down for `Some(true)` and up for `Some(false)`, or either.
    /// A heading's link leaves its line at the view's top, so a j after it
    /// keeps the view where it is. At once for a step, as a jump for
    /// further than half a screen.
    pub(crate) fn follow_line(&mut self, top: f64, foot: f64, toward: Option<bool>) {
        let (view_top, view_foot) = self.view_pt();
        let room = SCROLL_OFF_LINES * line_pt();
        let to_pt = if toward != Some(true) && top - room < view_top {
            top - room
        } else if toward != Some(false) && foot + room > view_foot {
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
        self.flash(layout.pieces(&taken));
        clip(layout, taken);
    }

    /// Flashes pieces of the page on the selection's ground for a moment,
    /// as his Neovim's yank does.
    fn flash(&mut self, pieces: Vec<text::Piece>) {
        if keys_logged() {
            eprintln!("flash: {} lines for {} ms", pieces.len(), FLASH.as_millis());
        }
        self.flash = Some((pieces, Instant::now() + FLASH));
        let flashed =
            self.loop_handle
                .insert_source(Timer::from_duration(FLASH), |_, (), app: &mut App| {
                    // A flash after it ends its own time; only the last
                    // one's ends what shows.
                    let over = app
                        .flash
                        .as_ref()
                        .is_some_and(|(_, until)| Instant::now() >= *until);
                    if over {
                        app.flash = None;
                        if keys_logged() {
                            eprintln!("flash: over");
                        }
                    }
                    if !app.frame_pending {
                        app.draw();
                    }
                    TimeoutAction::Drop
                });
        if let Err(error) = flashed {
            eprintln!("reader: can't time the flash: {error}");
        }
    }

    /// The cursor's line, its cell's top and foot in points: the
    /// selection's cursor's while selecting, and else the page's.
    pub(crate) fn cursor_line(&self) -> Option<(f64, f64)> {
        if let (Some(layout), Some(selection)) = (self.layout.as_ref(), self.selection.as_ref()) {
            return layout.lines.get(selection.cursor.line).map(moves::rule_of);
        }
        let (i, lines) = self.cursor().zip(self.words.as_deref())?;
        Some(moves::rule_of(&lines[i]))
    }

    /// The cursor's line, marked as his `cursorlineopt=number` marks only
    /// the number: a thin rule in the accent in the page's left margin,
    /// as tall as the line's cell, clear of the marks a list hangs there.
    /// A window too narrow for that sets it half way into the room left of
    /// the words, and one with no room there draws none.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "points and pixels as f32, as GL takes them, within a page's few hundred thousand"
    )]
    pub(crate) fn draw_cursor_line(&self, gl: &gl::Gl, screen: f32) {
        let (false, Some(rule), Some((top, foot))) =
            (baseline(), self.rule.as_ref(), self.cursor_line())
        else {
            return;
        };
        let px = self.px_per_pt();
        let y = (top * px - self.offset).round();
        let height = ((foot - top) * px).round();
        if y + height < 0.0 || y > f64::from(screen) {
            return;
        }
        let words = f64::from(self.page_x()) + self.width.spread_pt * px;
        let width = (RULE_PX * self.scale).round().max(1.0);
        let right = words - (rule_from_words_pt() * px).min(words / 2.0);
        if right < width {
            return;
        }
        let x = (right - width).round();
        gl.draw(rule, x as f32, y as f32, width as f32, height as f32);
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
        let mark = |chosen: &gl::Mark, ground: (f64, f64, f64, f64)| {
            let (top, foot, from, to) = ground;
            let y = top * px - offset;
            let height = (foot - top) * px;
            if y + height < 0.0 || y > f64::from(screen) {
                return;
            }
            gl.lay(
                chosen,
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
            // The block stands its line's cell, as the ground under it does,
            // so it fills the selection's height rather than its letters'.
            let (top, foot) = select::line_cell(at.baseline_pt, at.cap_pt, at.size_pt);
            mark(cursor, (top, foot, at.from_pt, at.to_pt));
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

/// Copies what a selection took as the file's markdown, on a thread, so
/// reading it and wl-copy's start never hold a frame.
fn clip(layout: &Arc<select::Layout>, taken: Selection) {
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
