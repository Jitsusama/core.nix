//! Drawing a frame: the marks over the page, the bands under them, what is
//! drawn above the page, and the line a reading says on.

use super::{Drawn, ReadsOn, STAND_IN, covers, ground, painted, phase};
use crate::painter::Key;
use crate::{App, baseline, gl, keys_logged, place, prompt, reading, run_delay_ns, whole};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use std::time::Instant;

impl App {
    /// Each search match over the page, the one the view is on in the
    /// current match's colours.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "points and pixels as f32, as GL takes them, within a page's few hundred thousand"
    )]
    fn draw_marks(&self, gl: &gl::Gl, screen: f32) {
        let (true, false, Some([others, current])) =
            (self.highlighting, baseline(), self.lit.as_ref())
        else {
            return;
        };
        let px = (self.painter.scale(1000) * self.level()) as f32;
        let offset = self.offset as f32;
        let left = self.page_x();
        for (i, found) in self.found.iter().enumerate() {
            let (swatch, blend) = if Some(i) == self.current {
                current
            } else {
                others
            };
            for piece in &found.pieces {
                let top = (piece.baseline_pt - piece.size_pt * 0.92) as f32 * px - offset;
                let height = (piece.size_pt * 1.2) as f32 * px;
                if top + height < 0.0 || top > screen {
                    continue;
                }
                let width = (piece.to_pt - piece.from_pt) as f32 * px;
                gl.mark(
                    swatch,
                    blend,
                    left + piece.from_pt as f32 * px,
                    top,
                    width,
                    height,
                );
            }
        }
    }

    pub(crate) fn draw(&mut self) {
        let began = Instant::now();
        let waited_before = run_delay_ns();
        if !self.shown && !self.first_frame_ready() {
            return;
        }
        let mut script_ms = 0.0;
        if self.measure.is_some() {
            match self.measure_step(began) {
                Some(ms) => script_ms = ms,
                None => return,
            }
        }
        self.draw_after_measure(began, waited_before, script_ms);
    }

    /// What stands over the page: the prompt at its foot, gO's outline on
    /// it, and the key list in the middle.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a canvas or a buffer is a whole number of pixels within a screen's few thousand"
    )]
    fn draw_over(&mut self, screen: f32) {
        let Some(gl) = self.gl.as_ref() else { return };
        let mut over = None;
        if self.prompt_shown
            && let Some(canvas) = painted(gl, &mut self.prompt_canvas, self.prompt_set.take())
        {
            let (width, height) = (canvas.width as f32, canvas.height as f32);
            gl.draw(canvas, 0.0, screen - height, width, height);
            over = Some(screen - height);
        }
        // gO's outline stands on the prompt's row while that shows, as his
        // location list stands on his command line.
        let below = match self.prompt_canvas.as_ref() {
            Some(canvas) if self.prompt_shown => canvas.height as f32,
            _ => 0.0,
        };
        let mut list_top = None;
        if (self.outline.is_some() || self.menu_shows())
            && let Some(canvas) = painted(gl, &mut self.outline_canvas, self.outline_set.take())
        {
            let (width, height) = (canvas.width as f32, canvas.height as f32);
            gl.draw(canvas, 0.0, screen - below - height, width, height);
            list_top = Some(screen - below - height);
        }
        if let Some(top) = list_top {
            self.draw_reading(top);
        }
        if let Some(top) = list_top.or(over) {
            self.bare_above(top);
        }
        self.draw_keys();
    }

    /// Lays the ground over the page from the top of the first line an
    /// overlay whose top stands at `over` would cut, so no line shows cut by
    /// it and no heading stands alone over it (whole.rs).
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "pixels as f32, as GL takes them, within a screen's few thousand"
    )]
    fn bare_above(&mut self, over: f32) {
        let version = self.painter.version;
        let count = self.words.as_ref().map_or(0, Vec::len);
        if self
            .heading_lines
            .as_ref()
            .is_none_or(|(read, heads)| *read != version || heads.len() != count)
            && let Some(lines) = self.words.as_deref()
        {
            let blocks = place::blocks(&self.painter.page().frame);
            self.heading_lines = Some((version, whole::headings(&blocks, lines)));
        }
        let (Some(gl), Some(ground), Some(lines), Some((_, heads))) = (
            self.gl.as_ref(),
            self.bare.as_ref(),
            self.words.as_deref(),
            self.heading_lines.as_ref(),
        ) else {
            return;
        };
        let shown = whole::shown(lines, heads, self.px_per_pt(), self.offset);
        let from = whole::bare_from(&shown, f64::from(over)).max(0.0).round() as f32;
        if from < over {
            gl.draw(ground, 0.0, from, gl.width as f32, over - from);
        }
    }

    /// Starts the chosen row's words reading on, when they run past their
    /// room, or keeps them reading where it's the row already reading;
    /// a set for a row no longer chosen reads nothing.
    pub(crate) fn read_on(&mut self, strip: Option<prompt::Strip>) {
        let chosen = self
            .outline
            .as_ref()
            .map(|outline| outline.rows[outline.chosen].words.as_str());
        let Some(strip) = strip.filter(|strip| Some(strip.said.as_str()) == chosen) else {
            self.stop_reading();
            return;
        };
        let over = f64::from(strip.words.width) - strip.room_px;
        if let Some(on) = self.reads_on.as_mut()
            && on.strip.said == strip.said
            && (on.over - over).abs() < 0.5
        {
            on.strip = strip;
            on.painted = false;
            return;
        }
        self.stop_reading();
        if keys_logged() {
            eprintln!("outline: reads on {} px past its room", over.round());
        }
        let pace = reading::FIGURES_A_SECOND * strip.figure_px;
        self.reads_on = Some(ReadsOn {
            reading: reading::Reading::new(over, pace, Instant::now()),
            over,
            strip,
            canvas: None,
            painted: false,
            wake: None,
            moved: false,
            ended: false,
        });
    }

    /// Lets the chosen row's words go back to their start.
    pub(crate) fn stop_reading(&mut self) {
        if let Some(on) = self.reads_on.take()
            && let (Some(gl), Some(canvas)) = (self.gl.as_ref(), on.canvas)
        {
            gl.forget(canvas);
        }
    }

    /// The chosen row's words where they've slid to, over the list whose
    /// top stands at `top`, shown only inside their room; at their start
    /// the list's own row shows, ending in its ellipsis. Resting, a timer
    /// wakes the frame that moves them on.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "a row is a few thousand pixels at most"
    )]
    fn draw_reading(&mut self, top: f32) {
        let (Some(gl), Some(on)) = (self.gl.as_ref(), self.reads_on.as_mut()) else {
            return;
        };
        let at = on.reading.at(Instant::now());
        if let Some(until) = at.resting_until
            && on.wake != Some(until)
        {
            on.wake = Some(until);
            let woken = self.loop_handle.insert_source(
                Timer::from_deadline(until),
                |_, (), app: &mut App| {
                    if !app.frame_pending {
                        app.draw();
                    }
                    TimeoutAction::Drop
                },
            );
            if let Err(error) = woken {
                eprintln!("reader: can't time the reading on: {error}");
            }
        }
        if at.slid <= 0.0 {
            return;
        }
        if !on.moved && keys_logged() {
            eprintln!("outline: reading on");
        }
        on.moved = true;
        if !on.ended && at.resting_until.is_some() && keys_logged() {
            eprintln!("outline: read on to its end");
        }
        on.ended |= at.resting_until.is_some();
        let words = &on.strip.words;
        let fits = on
            .canvas
            .as_ref()
            .is_some_and(|it| it.width == words.width && it.height == words.height);
        if !fits {
            if let Some(old) = on.canvas.take() {
                gl.forget(old);
            }
            on.canvas = gl.canvas(words.width, words.height).ok();
            on.painted = false;
        }
        let Some(canvas) = on.canvas.as_ref() else {
            return;
        };
        if !on.painted {
            gl.paint(canvas, &words.pixels);
            on.painted = true;
        }
        let (left, y) = (on.strip.left_px as f32, top + on.strip.top_px as f32);
        let (width, height) = (canvas.width as f32, canvas.height as f32);
        gl.draw_within(
            canvas,
            (left - at.slid as f32, y, width, height),
            [left, y, on.strip.room_px as f32, height],
        );
    }

    /// The frame past the first frame's wait and the measure's script: the
    /// view moved, bands taken in and asked for, drawn, marked, and swapped.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn draw_after_measure(&mut self, began: Instant, waited_before: u64, script_ms: f64) {
        self.move_jump(began);
        self.clamp();
        let uploading = Instant::now();
        self.take_in();
        let upload_ms = uploading.elapsed().as_secs_f64() * 1000.0;
        let asking = Instant::now();
        self.ask_for_bands();
        let ask_ms = asking.elapsed().as_secs_f64() * 1000.0;
        let Some(gl) = self.gl.as_ref() else { return };
        let key = self.zoom_key();
        let screen = self.screen_px() as f32;
        let left = self.page_x();
        let drawing = Instant::now();
        // The first GL call after a swap, where Mesa takes the next back
        // buffer and waits if the compositor holds them all.
        gl.clear(ground());
        let clear_ms = drawing.elapsed().as_secs_f64() * 1000.0;
        // The stand-in under everything, other zooms' bands over it, and
        // this zoom's on top, so a view shows the sharpest it has.
        let mut keys: Vec<&Key> = if baseline() {
            Vec::new()
        } else {
            self.textures.keys().collect()
        };
        keys.sort_by_key(|k| (k.zoom != STAND_IN, k.zoom == key, k.index));
        let mut shown = Vec::new();
        let mut sharp = Vec::new();
        let mut drawn_now = Vec::new();
        for k in keys {
            let texture = &self.textures[k];
            let (top, height) = self.placed(*k, texture.height);
            if top + height < 0.0 || top > screen {
                continue;
            }
            let ratio = (self.level() * 1000.0 / f64::from(k.zoom)) as f32;
            gl.draw(texture, left, top, texture.width as f32 * ratio, height);
            drawn_now.push(*k);
            // A band of the first screen's page holds the page only down to
            // where it settled; below that it shows ground, so it counts so.
            // While that page is held it is the page shown, so it counts whole.
            let foot = if self.stale.contains(k) && self.held_pt.is_none() {
                let settled = self.settled_pt * self.painter.scale(k.zoom) * f64::from(ratio);
                (settled - self.offset) as f32
            } else {
                top + height
            };
            let span = (top.max(0.0), foot.min(top + height).min(screen));
            shown.push(span);
            if k.zoom == key {
                sharp.push(span);
            }
        }
        // Blank: some of the view shows only the ground. Stood in: some of
        // it shows a stand-in rather than this zoom's own bands.
        let page_end = (self.end_px() - self.offset) as f32;
        let draw_ms = drawing.elapsed().as_secs_f64() * 1000.0;
        let fresh = drawn_now.iter().filter(|k| self.fresh.remove(k)).count();
        let blank = !covers(&mut shown, screen.min(page_end));
        let stood_in = !blank && !covers(&mut sharp, screen.min(page_end));
        // Other zooms' bands go only once this zoom's alone cover the view;
        // the stand-in never does.
        // counting the others in let the first new band send them all away.
        let covered = self.drawn_at(self.offset);
        let evicting = Instant::now();
        self.evict(covered);
        let evict_ms = evicting.elapsed().as_secs_f64() * 1000.0;
        let Some(gl) = self.gl.as_ref() else { return };
        // The matches over the page, a selection over them and its cursor
        // over that, then the prompt over its foot.
        self.draw_marks(gl, screen);
        self.draw_selection(gl, screen);
        self.draw_cursor_line(gl, screen);
        self.draw_labels();
        self.draw_over(screen);
        let Some(gl) = self.gl.as_ref() else { return };
        if self.ask_next_frame() {
            self.frame_pending = true;
        }
        let swapping = Instant::now();
        if let Err(error) = gl.swap() {
            eprintln!("can't show the frame: {error}");
        }
        let swap_ms = swapping.elapsed().as_secs_f64() * 1000.0;
        if let Some(measure) = self.measure.as_mut() {
            let t = (began - measure.start.unwrap_or(began)).as_secs_f64();
            measure.drawn.push(Drawn {
                phase: phase(t),
                t,
                work_ms: began.elapsed().as_secs_f64() * 1000.0,
                parts_ms: [
                    upload_ms, evict_ms, swap_ms, ask_ms, script_ms, draw_ms, clear_ms,
                ],
                waited_ms: run_delay_ns().saturating_sub(waited_before) as f64 / 1e6,
                blank,
                stood_in,
                fresh,
                presented_ns: None,
            });
        }
    }
}
