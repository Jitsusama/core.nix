//! Where the view stands: the buffer's cut and scale, the zoom, the page's
//! edges, the cursor and the jumps between places.

use super::{FLICK, JUMP_UNTIL, REFERENCE_SCALE, SCROLL_UNTIL, Spot, ZOOM_UNTIL, covers, phase};
use crate::{App, SCROLL_OFF_LINES, line_pt, motion, moves, search};
use smithay_client_toolkit::shell::WaylandSurface;
use std::time::Instant;

impl App {
    /// Shows the buffer's top left at the window's size.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn cut(&self) {
        if let Some(viewport) = &self.viewport {
            let (w, h) = self.size;
            let (bw, bh) = self.buffer_size();
            // The source is in the surface's own pixels: the buffer's over
            // its buffer scale, which is 1 under a fractional scale.
            let over = f64::from(self.buffer_scale());
            viewport.set_source(0.0, 0.0, f64::from(bw) / over, f64::from(bh) / over);
            viewport.set_destination(w as i32, h as i32);
        }
    }

    /// The buffer for the window at its scale, in device pixels.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn buffer_size(&self) -> (i32, i32) {
        let (w, h) = self.size;
        (
            (f64::from(w) * self.scale).round() as i32,
            (f64::from(h) * self.scale).round() as i32,
        )
    }

    /// The surface's buffer scale: 1 under a fractional scale, which the
    /// viewport carries instead, and the whole scale otherwise.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn buffer_scale(&self) -> i32 {
        if self.fractional.is_some() {
            1
        } else {
            self.scale.round() as i32
        }
    }

    /// Draws at a new scale from here on: the bands drawn at the old one
    /// are let go and the view keeps its place.
    pub(crate) fn rescale(&mut self, scale: f64) {
        if (scale - self.scale).abs() < 1e-6 {
            return;
        }
        self.offset *= scale / self.scale;
        self.scale = scale;
        self.window
            .wl_surface()
            .set_buffer_scale(self.buffer_scale());
        // The bands drawn at the old scale are of another level now, and
        // show stretched until this level's replace them, as a zoom's do.
        let (w, h) = self.buffer_size();
        if let Some(gl) = self.gl.as_mut() {
            gl.resize(w, h);
        }
        self.cut();
        if !self.frame_pending {
            self.draw();
        }
    }

    /// How large the page is drawn: the zoom over the screen's scale, against
    /// the reference scale bands are keyed by.
    pub(crate) fn level(&self) -> f64 {
        self.zoom * self.scale / REFERENCE_SCALE
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn zoom_key(&self) -> u32 {
        // Drawn in twentieths of a level, so a glide asks for a band set a
        // step rather than every frame, and a new scale is one more step.
        ((self.level() * 20.0).round() * 50.0) as u32
    }

    pub(crate) fn screen_px(&self) -> f64 {
        self.gl.as_ref().map_or(0.0, |gl| f64::from(gl.height))
    }

    /// Where the page's left edge sits in the window, in device pixels: the
    /// page centred, so a wide window leaves even room either side of its
    /// measure, on a whole pixel so its type is sharp, and at the window's
    /// edge once the page is as wide as the window.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a window's pixels as f32, as GL takes them"
    )]
    pub(crate) fn page_x(&self) -> f32 {
        let window = self.gl.as_ref().map_or(0.0, |gl| f64::from(gl.width));
        ((window - self.painter.width_pt() * self.px_per_pt()) / 2.0)
            .max(0.0)
            .round() as f32
    }

    pub(crate) fn clamp(&mut self) {
        self.offset = self.offset.clamp(0.0, self.most());
    }

    pub(crate) fn set_zoom(&mut self, zoom: f64) {
        self.offset *= zoom / self.zoom;
        self.zoom = zoom;
    }

    /// Moves the view as the measure's script says at `t` seconds, `dt` since
    /// the last frame.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn script(&mut self, t: f64, dt: f64) {
        let flick = FLICK * self.scale * dt;
        let screen = self.screen_px();
        match phase(t) {
            "scroll" if t < SCROLL_UNTIL / 2.0 => self.offset += flick,
            "scroll" => self.offset -= flick,
            "jump" => {
                // A screen down every 0.4 s, then to the end and the top, as
                // Ctrl+F, G and gg would.
                let step = ((t - SCROLL_UNTIL) / 0.4).floor() as u32;
                let measure = self.measure.as_mut().expect("measuring");
                if step > measure.jumps {
                    measure.jumps = step;
                    let to = match step {
                        1..=7 => self.offset + screen,
                        8 => f64::MAX,
                        _ => 0.0,
                    };
                    self.jump_to(to);
                }
            }
            "zoom" => {
                self.jump = None;
                let s = t - JUMP_UNTIL;
                let zoom = if s < 2.0 {
                    [1.25, 1.5, 1.25, 1.0][((s / 0.5) as usize).min(3)]
                } else {
                    1.0 + 0.5 * (std::f64::consts::PI * (s - 2.0) / 2.0).sin()
                };
                self.set_zoom(zoom);
            }
            _ => {
                // A floating window in a headless sway is resized by
                // nobody, so the reader resizes its own buffer every quarter
                // second, which costs it what a resize from niri would.
                self.set_zoom(1.0);
                self.offset += 300.0 * self.scale * dt;
                let wide = ((t - ZOOM_UNTIL) / 0.25) as u32 % 2 == 1;
                let width = if wide { 1200 } else { 960 };
                if width != self.size.0 {
                    self.size.0 = width;
                    let (w, h) = self.buffer_size();
                    if let Some(gl) = self.gl.as_mut() {
                        gl.resize(w, h);
                    }
                    self.cut();
                    if let Some(measure) = self.measure.as_mut() {
                        measure.configures += 1;
                    }
                }
            }
        }
    }

    /// Asks for the bands in view and a screen either side, nearest first.
    /// The furthest down the view's top may go, in device pixels.
    pub(crate) fn most(&self) -> f64 {
        (self.end_px() - self.screen_px()).max(0.0)
    }

    /// Whether the view is at the page's end, which it keeps to as the page
    /// or the window changes under it, as while writing there, or is on
    /// its way there.
    pub(crate) fn at_end(&self) -> bool {
        motion::at_end(self.offset, self.jump.as_ref(), self.most())
    }

    /// Keeps a view at the end to the page's end where it is now: a jump on
    /// its way to the old end goes on to the new one, as G meant.
    pub(crate) fn keep_to_end(&mut self) {
        if self.jump.is_some() {
            self.jump_to(self.most());
        } else {
            self.offset = self.most();
        }
    }

    /// Where the page shown ends, in device pixels from its top at the
    /// zoom shown: the first screen's page's end while it is held.
    pub(crate) fn end_px(&self) -> f64 {
        let pt = self.held_pt.unwrap_or_else(|| self.painter.height_pt());
        pt * self.px_per_pt()
    }

    /// Whether the whole page's own bands, at any zoom or the stand-in's,
    /// cover the view down to the whole page's end, with nothing of the
    /// first screen's page needed.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn whole_page_covers(&self) -> bool {
        let screen = self.screen_px() as f32;
        let mut spans: Vec<(f32, f32)> = self
            .textures
            .iter()
            .filter(|(k, _)| !self.stale.contains(k))
            .map(|(k, texture)| {
                let (top, height) = self.placed(*k, texture.height);
                (top.max(0.0), (top + height).min(screen))
            })
            .filter(|(top, foot)| foot > top)
            .collect();
        let end = (self.painter.height_pt() * self.px_per_pt() - self.offset) as f32;
        covers(&mut spans, screen.min(end))
    }

    /// Starts a jump: it holds the view until the bands it lands on are
    /// drawn, then moves there (motion.rs).
    pub(crate) fn jump_to(&mut self, to: f64) {
        let to = to.clamp(0.0, self.most());
        let from = self
            .jump
            .as_ref()
            .map_or(self.offset, |jump| jump.at(Instant::now(), false).0);
        self.offset = from;
        self.jump = Some(motion::Jump::new(
            from,
            to,
            self.screen_px(),
            Instant::now(),
        ));
    }

    /// Device pixels a point of the page takes at the zoom shown.
    pub(crate) fn px_per_pt(&self) -> f64 {
        self.painter.scale(1000) * self.level()
    }

    /// Where the view is going: a jump's landing while one runs.
    pub(crate) fn heading(&self) -> f64 {
        self.jump.as_ref().map_or(self.offset, |jump| jump.to)
    }

    /// Moves at once, as a line does: a jump in progress is cut short.
    pub(crate) fn step_to(&mut self, to: f64) {
        self.jump = None;
        self.offset = to;
        self.clamp();
        self.ask_for_bands();
    }

    /// A jump vi remembers: where it left from goes on the jump list and
    /// becomes the place '' goes back to.
    pub(crate) fn leap(&mut self, to: f64) {
        self.remember(self.heading() / self.px_per_pt());
        self.jump_to(to);
    }

    /// Puts a place, in points, on the jump list as the last jump's start,
    /// with the cursor's line where it is now.
    pub(crate) fn remember(&mut self, here: f64) {
        self.remember_at(here, self.cursor_pt());
    }

    /// Puts a place and the cursor's line there on the jump list as the
    /// last jump's start.
    pub(crate) fn remember_at(&mut self, here: f64, aim: f64) {
        let spot = Spot {
            file: self.file.clone(),
            pt: here,
            aim,
        };
        self.jumps.truncate(self.jump_at);
        self.jumps.push(spot.clone());
        self.jump_at = self.jumps.len();
        self.last_jump = Some(spot);
        self.leapt = true;
    }

    /// The cursor's line, among the page's lines, once they're read.
    pub(crate) fn cursor(&self) -> Option<usize> {
        let (top, foot) = self.view_pt();
        moves::cursor(self.words.as_deref()?, top, foot, self.aim)
    }

    /// Where the cursor's line's top is, in points, or the view's top
    /// before the page's lines are read.
    pub(crate) fn cursor_pt(&self) -> f64 {
        self.cursor()
            .zip(self.words.as_deref())
            .map_or_else(|| self.view_pt().0, |(i, lines)| moves::top_of(&lines[i]))
    }

    /// The first line whose top is at or below `pt`: where a block's words
    /// start, which a move to it lands on.
    pub(crate) fn line_from(&self, pt: f64) -> Option<usize> {
        self.words
            .as_deref()?
            .iter()
            .position(|line| moves::top_of(line) >= pt - 0.5)
    }

    /// Puts the view's top on a place, in points, and the cursor on its
    /// line, as a heading's link lands; past the page's end is its end, the
    /// cursor on the last line.
    pub(crate) fn land(&mut self, pt: f64) {
        let to = self
            .line_from(pt)
            .zip(self.words.as_deref())
            .map_or(pt, |(i, lines)| moves::top_of(&lines[i]));
        self.aim = Some(to);
        self.jump_to(to * self.px_per_pt());
    }

    /// Puts the cursor on the line at or below `pt`, as Vim moves its
    /// cursor to a paragraph, a heading, a mark or a line's number: the
    /// view stays while the line shows four lines inside its edges, moves
    /// just enough to keep them when the line is less than half a screen
    /// past one, and else sets the line in its middle. Without the page's
    /// lines, or past its last, it lands as a heading's link does.
    pub(crate) fn bring(&mut self, pt: f64) {
        let Some((top, foot)) = self
            .line_from(pt)
            .zip(self.words.as_deref())
            .map(|(i, lines)| (moves::top_of(&lines[i]), moves::foot_of(&lines[i])))
        else {
            self.land(pt);
            return;
        };
        self.aim = Some(top);
        let (view_top, view_foot) = self.view_pt();
        let half = (view_foot - view_top) / 2.0;
        if top < view_top - half || foot > view_foot + half {
            self.jump_to((f64::midpoint(top, foot) - half) * self.px_per_pt());
        } else {
            self.follow_line(top, foot, None);
        }
    }

    /// j and k: the cursor `by` lines down, or up, by the lines shown, and
    /// the view after it, keeping four lines between it and the edge it
    /// goes toward, as his Neovim's scrolloff. Before the page's lines are
    /// read there's no line to move to, so the view moves as they would.
    pub(crate) fn step_cursor(&mut self, by: isize) {
        let (top, _) = self.view_pt();
        let Some((from, lines)) = self.words.as_deref().and_then(|lines| {
            let shown = self
                .cursor()
                .or_else(|| lines.iter().position(|line| moves::top_of(line) >= top))?;
            Some((shown, lines))
        }) else {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a count of lines, a few thousand at most"
            )]
            let by_px = by as f64 * line_pt() * self.px_per_pt();
            self.step_to(self.heading() + by_px);
            return;
        };
        let to = &lines[moves::line_by(lines, from, by)];
        let (top, foot) = (moves::top_of(to), moves::foot_of(to));
        self.aim = Some(top);
        self.follow_line(top, foot, Some(by > 0));
    }

    /// After the view moved without the cursor, as Ctrl+E, Ctrl+D and
    /// Ctrl+F move it: the cursor, from its line `from_pt`, keeps to the
    /// view, four lines inside each edge the page goes on past.
    pub(crate) fn keep_cursor(&mut self, from_pt: f64) {
        let Some(lines) = self.words.as_deref() else {
            return;
        };
        let (top, foot) = self.view_pt();
        let room = SCROLL_OFF_LINES * line_pt();
        let top = if self.heading() > 0.5 {
            top + room
        } else {
            top
        };
        let foot = if self.heading() < self.most() - 1.0 {
            foot - room
        } else {
            foot
        };
        let kept = moves::kept_in(lines, top, foot, from_pt)
            .or_else(|| moves::kept_in(lines, self.view_pt().0, self.view_pt().1, from_pt));
        if let Some(i) = kept {
            self.aim = Some(moves::top_of(&lines[i]));
        }
    }

    /// Puts the cursor on a line: the view stays where the line shows
    /// whole, as vi's cursor moves within the screen, and else the view
    /// brings it in, as Vim's does.
    pub(crate) fn reach(&mut self, line: usize) {
        let Some(lines) = self.words.as_deref() else {
            return;
        };
        let (top, foot) = (moves::top_of(&lines[line]), moves::foot_of(&lines[line]));
        let (view_top, view_foot) = self.view_pt();
        if top >= view_top - 0.5 && foot <= view_foot {
            self.aim = Some(top);
        } else {
            self.bring(top);
        }
    }

    /// Star and hash: a search down or up for the cursor's line's first
    /// word, whole and whatever its case, from that word on, as vi's.
    pub(crate) fn same_word(&mut self, forward: bool, times: u32) {
        let Some((from, word)) = self
            .cursor()
            .zip(self.words.as_deref())
            .and_then(|(i, lines)| {
                moves::first_word(&lines[i]).map(|word| (moves::top_of(&lines[i]), word))
            })
        else {
            return;
        };
        self.searched = Some((format!("\\<{word}\\>"), forward));
        self.word_search = true;
        self.refind();
        // From the word itself, as vi's * starts on the word under the
        // cursor, so it's passed over once.
        self.current = search::from(&self.found, from - 0.5, true);
        self.go_on(true, times);
    }
}
