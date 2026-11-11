//! The page's bands: which the view covers, asking the workers for the ones it
//! lacks and their blurred stand-ins, taking each in as it is made, and letting
//! the far ones go.

use super::{
    ASKED_AT_ONCE, FIRST_FRAME_AT_MOST, LOADING_AT_ONCE, Loading, RESIZE_UNTIL, STAND_IN,
    STAND_INS_AT_ONCE, covers, memory, phase,
};
use crate::painter::{BAND_PX, Key};
use crate::{App, Feedback, motion, page, prompt};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use std::time::Instant;

impl App {
    /// Whether the view shows no ground: its bands at any zoom, stand-in
    /// included, cover it down to the page's end.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn covered(&self) -> bool {
        let screen = self.screen_px() as f32;
        let mut spans: Vec<(f32, f32)> = self
            .textures
            .iter()
            .map(|(k, texture)| {
                let (top, height) = self.placed(*k, texture.height);
                let foot = if self.stale.contains(k) && self.held_pt.is_none() {
                    let ratio = self.level() * 1000.0 / f64::from(k.zoom);
                    let settled = self.settled_pt * self.painter.scale(k.zoom) * ratio;
                    (settled - self.offset) as f32
                } else {
                    top + height
                };
                (top.max(0.0), foot.min(top + height).min(screen))
            })
            .filter(|(top, foot)| foot > top)
            .collect();
        let page_end = (self.end_px() - self.offset) as f32;
        covers(&mut spans, screen.min(page_end))
    }

    /// Whether the bands for a view whose top is at `top` are all drawn.
    pub(crate) fn drawn_at(&self, top: f64) -> bool {
        self.missing_at(top).is_empty()
    }

    /// The bands a view whose top is at `top` lacks, each as its index and
    /// whether it is asked, waiting to upload, or not asked at all.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn missing_at(&self, top: f64) -> Vec<(u32, &'static str)> {
        let key = self.zoom_key();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let first = (top / band).floor().max(0.0) as u32;
        let last = ((top + self.screen_px()) / band).ceil() as u32;
        (first..last.min(self.painter.bands(key)))
            .filter_map(|index| {
                let k = Key { zoom: key, index };
                if self.textures.contains_key(&k) {
                    None
                } else if self.asked.contains(&k) {
                    Some((index, "asked"))
                } else if self.arrived.iter().any(|band| band.key == k) {
                    Some((index, "waiting"))
                } else {
                    Some((index, "unasked"))
                }
            })
            .collect()
    }

    /// Moves a jump on: held until its bands are drawn or it has waited
    /// long enough, then eased there.
    pub(crate) fn move_jump(&mut self, now: Instant) {
        let Some(jump) = self.jump.as_ref() else {
            return;
        };
        if jump.moving.is_none() {
            let ready = self.drawn_at(jump.from) && self.drawn_at(jump.to);
            let waited = now - jump.asked;
            if !ready && waited < motion::HOLD_AT_MOST {
                return;
            }
            let (from, to) = (jump.from, jump.to);
            let missing = format!(
                "from {} {:?} to {} {:?} of {}",
                from.round(),
                self.missing_at(from),
                to.round(),
                self.missing_at(to),
                self.painter.bands(self.zoom_key())
            );
            if let Some(measure) = self.measure.as_mut() {
                measure.held_ms.push(waited.as_secs_f64() * 1000.0);
                if !ready {
                    measure.missing.push(missing);
                }
            }
            if let Some(jump) = self.jump.as_mut() {
                jump.moving = Some(now);
            }
        }
        let Some(jump) = self.jump.as_ref() else {
            return;
        };
        let (at, landed) = jump.at(now, self.reduced_motion);
        self.offset = at;
        if landed {
            self.jump = None;
        }
    }

    pub(crate) fn ask_for_bands(&mut self) {
        // A jump's landing first, then where it leaves from, then the view.
        let tops: Vec<f64> = match &self.jump {
            Some(jump) if jump.moving.is_none() => vec![jump.to, jump.from, self.offset],
            _ => vec![self.offset],
        };
        for top in tops {
            self.ask_for_bands_at(top);
        }
        // From the start, so a view that moves at once, or a first screen
        // that isn't drawn in time, has its stand-in.
        self.ask_for_stand_ins();
    }

    /// Asks for the stand-in's bands not held yet, nearest the view first,
    /// a few at a time beside the view's own.
    fn ask_for_stand_ins(&mut self) {
        let in_flight = self.asked.iter().filter(|k| k.zoom == STAND_IN).count();
        if in_flight >= STAND_INS_AT_ONCE {
            return;
        }
        let ratio = self.level() * 1000.0 / f64::from(STAND_IN);
        let band = f64::from(BAND_PX) * ratio;
        let middle = (self.offset + self.screen_px() / 2.0) / band;
        let mut wanted: Vec<u32> = (0..self.painter.bands(STAND_IN))
            .filter(|&index| {
                let key = Key {
                    zoom: STAND_IN,
                    index,
                };
                let held = self.textures.contains_key(&key) && !self.stale.contains(&key);
                !held
                    && !self.asked.contains(&key)
                    && !self.loading.contains(&key)
                    && !self.arrived.iter().any(|band| band.key == key)
            })
            .collect();
        wanted.sort_by(|a, b| {
            (f64::from(*a) + 0.5 - middle)
                .abs()
                .total_cmp(&(f64::from(*b) + 0.5 - middle).abs())
        });
        for index in wanted.into_iter().take(STAND_INS_AT_ONCE - in_flight) {
            let key = Key {
                zoom: STAND_IN,
                index,
            };
            self.asked.insert(key);
            self.painter.ask(key);
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn ask_for_bands_at(&mut self, top: f64) {
        let key = self.zoom_key();
        let ratio = self.level() * 1000.0 / f64::from(key);
        let screen = self.screen_px();
        let band = f64::from(BAND_PX) * ratio;
        let first = ((top - screen) / band).floor().max(0.0) as u32;
        let last = (((top + 2.0 * screen) / band).ceil() as u32).min(self.painter.bands(key));
        let middle = (top + screen / 2.0) / band;
        let mut wanted: Vec<u32> = (first..last).collect();
        wanted.sort_by(|a, b| {
            (f64::from(*a) - middle)
                .abs()
                .total_cmp(&(f64::from(*b) - middle).abs())
        });
        for index in wanted {
            // The stand-in's asks have their own few.
            if self.asked.iter().filter(|k| k.zoom != STAND_IN).count() >= ASKED_AT_ONCE {
                break;
            }
            let key = Key { zoom: key, index };
            // A band waiting to be uploaded is neither asked nor held yet;
            // asking again drew it twice and queued both, megabytes each.
            let waiting =
                self.loading.contains(&key) || self.arrived.iter().any(|band| band.key == key);
            let held = self.textures.contains_key(&key) && !self.stale.contains(&key);
            if !waiting && !held && self.asked.insert(key) {
                self.painter.ask(key);
            }
        }
    }

    /// Where a band sits on screen now: its top and height in device pixels.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn placed(&self, key: Key, height: u32) -> (f32, f32) {
        let ratio = self.level() * 1000.0 / f64::from(key.zoom);
        let top = f64::from(key.index) * f64::from(BAND_PX) * ratio - self.offset;
        (top as f32, (f64::from(height) * ratio) as f32)
    }

    /// Takes in the textures the loader has finished: a saved page's to
    /// its incoming page, this zoom's and the stand-in's to the view, and
    /// any other let go. One whose fence hasn't signalled waits.
    fn adopt_made(&mut self) {
        let Some(gl) = self.gl.as_ref() else { return };
        let key = self.zoom_key();
        for made in std::mem::take(&mut self.made) {
            match gl.adopt(made) {
                Ok((loading, texture)) if loading.version > self.painter.version => {
                    match self.incoming.as_mut() {
                        Some(incoming) if incoming.version == loading.version => {
                            incoming.loading.remove(&loading.key);
                            if loading.key.zoom == key {
                                if let Some(old) = incoming.textures.insert(loading.key, texture) {
                                    gl.forget(old);
                                }
                            } else {
                                gl.forget(texture);
                            }
                        }
                        _ => gl.forget(texture),
                    }
                }
                Ok((loading, texture)) => {
                    self.loading.remove(&loading.key);
                    // A zoom since passed, or a band of the first screen's
                    // page below where it settled, is let go, as `arrive`
                    // lets such a band go.
                    let wanted = loading.key.zoom == key || loading.key.zoom == STAND_IN;
                    let outdated = loading.version != self.painter.version
                        && self.painter.foot_pt(loading.key) > self.settled_pt;
                    if !wanted || outdated {
                        gl.forget(texture);
                        continue;
                    }
                    if let Some(measure) = self.measure.as_mut() {
                        measure.drawn_ms.push(loading.drawn_ms);
                    }
                    self.stale.remove(&loading.key);
                    self.fresh.insert(loading.key);
                    if let Some(old) = self.textures.insert(loading.key, texture) {
                        gl.forget(old);
                    }
                }
                Err(made) => self.made.push(made),
            }
        }
    }

    /// Takes the textures the loader has finished, then hands it the most
    /// wanted of the bands that have arrived. Nothing here makes a texture.
    fn upload_arrived(&mut self) {
        self.adopt_made();
        let key = self.zoom_key();
        let Some(loader) = self.loader.as_ref() else {
            return;
        };
        // A saved page's bands go before any other: until they cover the
        // view, the page shown is out of date.
        if let Some(incoming) = self.incoming.as_mut() {
            while let Some(band) = incoming.arrived.pop_front() {
                let loading = Loading {
                    key: band.key,
                    version: band.version,
                    drawn_ms: band.drawn_ms,
                };
                if loader.load(loading, band.width, band.height, band.pixels) {
                    incoming.loading.insert(band.key);
                }
            }
        }
        // This zoom's bands first, nearest the view being waited for: a held
        // jump's landing, or else the view. In arrival order, the landing's
        // last band waited behind bands for the view the jump left.
        // A held jump's whole stretch, from where it starts moving to where
        // it lands, comes before anything else.
        let (wanted, stretch) = match &self.jump {
            Some(jump) if jump.moving.is_none() => (
                jump.to,
                Some((jump.from.min(jump.to), jump.from.max(jump.to))),
            ),
            _ => (self.offset, None),
        };
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let screen = self.screen_px();
        let middle = (wanted + screen / 2.0) / band;
        let outside = |i: usize| {
            stretch.is_some_and(|(top, bottom)| {
                let index = f64::from(self.arrived[i].key.index);
                (index + 1.0) * band <= top || index * band >= bottom + screen
            })
        };
        let mut order: Vec<usize> = (0..self.arrived.len()).collect();
        order.sort_by(|&a, &b| {
            let near = |i: usize| (f64::from(self.arrived[i].key.index) + 0.5 - middle).abs();
            // The stand-in first once the window shows: a few small bands,
            // and what keeps any view from showing ground. Before it shows,
            // the first screen's own bands come first.
            let first = !self.shown && self.opened.elapsed() < FIRST_FRAME_AT_MOST;
            let rank = |i: usize| match self.arrived[i].key.zoom {
                STAND_IN => u8::from(first),
                zoom if zoom == key => u8::from(!first),
                _ => 2,
            };
            rank(a)
                .cmp(&rank(b))
                .then(outside(a).cmp(&outside(b)))
                .then(near(a).total_cmp(&near(b)))
        });
        // Before the window shows, every band that waits goes at once, since
        // only what the loader sends back wakes the first frame.
        let room = if self.shown {
            LOADING_AT_ONCE.saturating_sub(self.loading.len())
        } else {
            usize::MAX
        };
        let taken: Vec<Key> = order
            .into_iter()
            .take(room)
            .map(|i| self.arrived[i].key)
            .collect();
        for key in taken {
            let Some(i) = self.arrived.iter().position(|band| band.key == key) else {
                continue;
            };
            let band = self.arrived.remove(i).expect("an arrived band");
            let loading = Loading {
                key,
                version: band.version,
                drawn_ms: band.drawn_ms,
            };
            if loader.load(loading, band.width, band.height, band.pixels) {
                self.loading.insert(key);
            }
        }
    }

    /// Lets go of bands well out of view, and of other zooms' once this
    /// zoom's cover the view.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn evict(&mut self, covered: bool) {
        let Some(gl) = self.gl.as_ref() else { return };
        let key = self.zoom_key();
        let screen = self.screen_px() as f32;
        // A held jump's start and landing are in play as much as the view,
        // or the bands it waits for would be let go as they arrive.
        let views: Vec<f32> = match &self.jump {
            Some(jump) if jump.moving.is_none() => {
                vec![
                    0.0,
                    (jump.from - self.offset) as f32,
                    (jump.to - self.offset) as f32,
                ]
            }
            _ => vec![0.0],
        };
        let mut gone = Vec::new();
        for (k, texture) in &self.textures {
            // The stand-in stays for as long as the page does.
            if k.zoom == STAND_IN {
                continue;
            }
            let (top, height) = self.placed(*k, texture.height);
            let far = views
                .iter()
                .all(|view| top + height < view - 2.0 * screen || top > view + 3.0 * screen);
            if far || (k.zoom != key && covered) {
                gone.push(*k);
            }
        }
        let many = gone.len() >= 4;
        for k in gone {
            self.stale.remove(&k);
            if let Some(texture) = self.textures.remove(&k) {
                gl.forget(texture);
            }
        }
        // Letting go of a zoom's bands frees tens of megabytes that malloc
        // would otherwise keep: give them back as they go, not on a timer.
        if many {
            page::trim();
        }
    }

    /// Takes the measure's script a frame on, saying how long that took,
    /// or nothing once the script has ended and the reader closes.
    pub(crate) fn measure_step(&mut self, began: Instant) -> Option<f64> {
        let measure = self.measure.as_mut()?;
        {
            let start = *measure.start.get_or_insert(began);
            let dt = measure
                .last
                .map_or(0.0, |last| (began - last).as_secs_f64());
            measure.last = Some(began);
            let t = (began - start).as_secs_f64();
            let ended = measure
                .drawn
                .last()
                .map(|d| d.phase)
                .filter(|p| *p != phase(t));
            if let Some(ended) = ended.or((t >= RESIZE_UNTIL).then_some("resize")) {
                measure.held.push((ended, memory(), self.textures.len()));
            }
            if t >= RESIZE_UNTIL {
                self.exit = true;
                return None;
            }
            let scripting = Instant::now();
            self.script(t, dt);
            Some(scripting.elapsed().as_secs_f64() * 1000.0)
        }
    }

    /// Whether the window may show its first frame: once its screen is
    /// drawn, so it's never bare ground, each band that arrives asking
    /// again. Past `FIRST_FRAME_AT_MOST`, a view the stand-in covers will
    /// do; one nothing covers never shows, since bare ground is never drawn.
    pub(crate) fn first_frame_ready(&mut self) -> bool {
        {
            // Where he was, now the window's scale is known, so the first
            // bands asked are the ones he'll see.
            if let Some(pt) = self.start_pt.take() {
                self.offset = pt * self.px_per_pt();
                self.clamp();
            }
            self.upload_arrived();
            let late = self.opened.elapsed() >= FIRST_FRAME_AT_MOST;
            if !self.drawn_at(self.offset) && (!late || !self.covered()) {
                self.log_first();
                self.ask_for_bands();
                return false;
            }
            self.shown = true;
            // Search's pieces start once the first frame is up, so they
            // take nothing from it.
            self.prompt = Some(prompt::Prompt::new(self.prompt_to.clone()));
            self.read_words();
            self.read_layout();
            if let Some(measure) = self.measure.as_mut() {
                measure.first_frame_ms = Some(self.opened.elapsed().as_secs_f64() * 1000.0);
            }
        }
        true
    }

    /// With `READER_FIRST_LOG`, says why the first frame waits.
    fn log_first(&self) {
        if std::env::var_os("READER_FIRST_LOG").is_some() {
            eprintln!(
                "first: {:.0} ms, missing {:?}, bands {}, end {:.0} px, screen {:.0} px, textures {:?}, loading {}, arrived {}",
                self.opened.elapsed().as_secs_f64() * 1000.0,
                self.missing_at(self.offset),
                self.painter.bands(self.zoom_key()),
                self.end_px(),
                self.screen_px(),
                self.textures
                    .keys()
                    .map(|k| (k.zoom, k.index))
                    .collect::<Vec<_>>(),
                self.loading.len(),
                self.arrived.len()
            );
        }
    }

    /// Asks for the next frame while something moves or waits to be drawn,
    /// saying whether it did, and, measuring, when this one reaches the
    /// screen.
    pub(crate) fn ask_next_frame(&self) -> bool {
        let surface = self.window.wl_surface();
        let asks = self.measure.is_some()
            || self.jump.is_some()
            || self
                .reads_on
                .as_ref()
                .is_some_and(|on| on.reading.at(Instant::now()).resting_until.is_none())
            || !self.arrived.is_empty()
            || !self.loading.is_empty()
            || !self.asked.is_empty();
        if asks {
            surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        }
        if let (Some(presentation), Some(measure)) = (&self.presentation, &self.measure) {
            presentation.feedback(surface, &self.qh, Feedback(measure.drawn.len() as u64));
        }
        asks
    }

    /// Takes in what's come for the frame: textures, and bands for the
    /// loader. The first screen's page is let go once the whole page covers
    /// the view, and a saved page takes its place once it covers it.
    pub(crate) fn take_in(&mut self) {
        self.upload_arrived();
        if self.held_pt.is_some() && self.whole_page_covers() {
            // At the first screen's end, or on its way there, the view goes
            // on to the whole page's, which is where he was or was going.
            let at_end = self.at_end();
            self.held_pt = None;
            if at_end {
                self.keep_to_end();
                self.ask_for_bands();
            }
        }
        if self.incoming.is_some() {
            if self.incoming_covers() {
                self.take_incoming();
            } else {
                self.ask_incoming();
            }
        }
    }
}
