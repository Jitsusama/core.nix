//! Where each file was left and what comes in: the place and its marks kept as
//! the reader closes, a page set again taken in, and where the view lands on
//! it.

use super::{Incoming, Landing, Reset, STAND_IN};
use crate::painter::{BAND_PX, Band, Key};
use crate::theme::PT_A_PX;
use crate::{
    App, anchor, kept_marks, keys_logged, links, moves, page, place, showing, start, text,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

impl App {
    /// The whole page has been set: draw from it, keeping each band above
    /// where the first screen's page settled and redrawing the rest in place.
    /// A save's page, set: it draws while the page before shows, from where
    /// the view's top words are on it.
    /// A page the reloader set: the file shown, saved, or the file being
    /// opened in its place. One of a file no longer wanted is let go.
    pub(crate) fn saved(&mut self, (file, set): Reset) {
        let file = std::path::PathBuf::from(file);
        let opens = match &self.following {
            Some(following) if following.file == file => true,
            None if file == self.file => false,
            _ => return,
        };
        let (page, ms, lines) = match set {
            Ok(set) => set,
            Err(error) => {
                eprintln!("reader: can't set {}: {error}", file.display());
                if opens {
                    // Left where it was, and saves of it heard again.
                    self.following = None;
                    self.set_on(&self.file);
                }
                return;
            }
        };
        // One set for a width the window has since left is let go, since
        // the reloader sets it again at the new one.
        if !self.fits(&page) {
            return;
        }
        self.let_incoming_go();
        let px_per_pt = self.px_per_pt();
        let screen_pt = self.screen_px() / px_per_pt;
        let new_most_pt = (page.frame.size().y.to_pt() - screen_pt).max(0.0);
        let offset_pt = if opens {
            self.landing(&file, &page, &lines, new_most_pt)
        } else {
            let at_end = self.at_end();
            let top_pt = self.heading() / px_per_pt;
            match &self.words {
                Some(old) => anchor::anchored(old, &lines, top_pt, at_end, new_most_pt),
                None => top_pt.min(new_most_pt),
            }
        };
        let version = self.next_version.max(self.painter.version + 1);
        self.next_version = version + 1;
        self.incoming = Some(Incoming {
            version,
            page: std::sync::Arc::new(page),
            offset: offset_pt * px_per_pt,
            set_at_pt: self.heading() / px_per_pt,
            lines,
            textures: HashMap::new(),
            asked: HashSet::new(),
            loading: HashSet::new(),
            arrived: VecDeque::new(),
            since: Instant::now(),
            set_ms: ms,
            opens: opens.then_some(file),
        });
        self.ask_incoming();
    }

    /// Where the view lands on a file opened in this one's place, in
    /// points: the heading a link names, the place a jump left, or where
    /// he last left the file, and otherwise its top.
    pub(crate) fn landing(
        &self,
        file: &std::path::Path,
        page: &typst_layout::Page,
        lines: &[text::Line],
        most_pt: f64,
    ) -> f64 {
        let Some(following) = self.following.as_ref() else {
            return 0.0;
        };
        let pt = match &following.landing {
            Landing::Heading(anchor) => links::heading_at(lines, anchor).unwrap_or_else(|| {
                eprintln!("reader: no heading in {} is #{anchor}", file.display());
                0.0
            }),
            Landing::At(pt, _) => *pt,
            Landing::Kept => match place::read(file) {
                Some(kept) if kept.at_end => most_pt,
                Some(kept) => place::pt_of(&place::starts(&page.frame), kept.block, kept.into_pt)
                    .unwrap_or(0.0),
                None => 0.0,
            },
        };
        pt.clamp(0.0, most_pt)
    }

    /// The bands the saved page needs to cover the view where it will show,
    /// at the zoom shown.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn ask_incoming(&mut self) {
        let key = self.zoom_key();
        let screen = self.screen_px();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let scale = self.painter.scale(key);
        let Some(incoming) = self.incoming.as_mut() else {
            return;
        };
        let height = incoming.page.frame.size().y.to_pt() * scale;
        let first = (incoming.offset / band).floor().max(0.0) as u32;
        let last =
            (((incoming.offset + screen) / band).ceil() as u32).min((height / band).ceil() as u32);
        for index in first..last {
            let k = Key { zoom: key, index };
            let have = incoming.textures.contains_key(&k)
                || incoming.loading.contains(&k)
                || incoming.arrived.iter().any(|b| b.key == k);
            if !have && incoming.asked.insert(k) {
                self.painter
                    .ask_from(std::sync::Arc::clone(&incoming.page), incoming.version, k);
            }
        }
    }

    /// Whether the saved page's bands cover the view where it will show.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn incoming_covers(&self) -> bool {
        let Some(incoming) = self.incoming.as_ref() else {
            return false;
        };
        let key = self.zoom_key();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let height = incoming.page.frame.size().y.to_pt() * self.painter.scale(key);
        let first = (incoming.offset / band).floor().max(0.0) as u32;
        let last = (((incoming.offset + self.screen_px()) / band).ceil() as u32)
            .min((height / band).ceil() as u32);
        (first..last).all(|index| incoming.textures.contains_key(&Key { zoom: key, index }))
    }

    /// The saved page takes the page's place, whole, with the view on the
    /// same words, and everything of the page before is let go.
    pub(crate) fn take_incoming(&mut self) {
        let Some(incoming) = self.incoming.take() else {
            return;
        };
        // A key that moved the view while this page was set, as G pressed
        // while a first open sets it again at the window's width, still
        // takes the view where it sent it once this page shows.
        let carried = if incoming.opens.is_none() {
            let px_per_pt = self.px_per_pt();
            let screen_pt = self.screen_px() / px_per_pt;
            anchor::carried(
                self.words.as_deref().unwrap_or(&[]),
                &incoming.lines,
                incoming.set_at_pt,
                self.heading() / px_per_pt,
                self.at_end(),
                (incoming.page.frame.size().y.to_pt() - screen_pt).max(0.0),
            )
        } else {
            None
        };
        // Another file's page: where the view was on this one is kept for
        // the next time it's shown, from its own page, before it goes.
        if incoming.opens.is_some() && !self.measuring {
            self.keep_place();
        }
        if let Some(gl) = self.gl.as_ref() {
            for (_, texture) in self.textures.drain() {
                gl.forget(texture);
            }
        }
        self.stale.clear();
        self.fresh.clear();
        self.asked.clear();
        self.arrived.clear();
        self.held_pt = None;
        self.settled_pt = f64::NEG_INFINITY;
        // A saved or followed file's page is set whole.
        self.whole = true;
        self.reloaded = true;
        self.jump = None;
        self.fresh.extend(incoming.textures.keys().copied());
        self.textures = incoming.textures;
        // Set again at another width, the page's blocks have moved, so each
        // mark moves with the block it was in. A save keeps them where they
        // were, as the words below them may have changed.
        let reflowed = incoming.opens.is_none()
            && (incoming.page.frame.size().x.to_pt() - self.painter.width_pt()).abs() >= 0.01;
        let before = reflowed.then(|| place::starts(&self.painter.page().frame));
        self.painter.take(incoming.page, incoming.version);
        if let Some(before) = before {
            let after = place::starts(&self.painter.page().frame);
            // The cursor's line goes with its block too.
            for pt in self.marks.values_mut().chain(self.aim.as_mut()) {
                let (block, into_pt) = place::at(&before, *pt);
                *pt = place::pt_of(&after, block, into_pt).unwrap_or(*pt);
            }
        }
        self.offset = incoming.offset;
        self.clamp();
        if let Some(pt) = carried {
            self.jump_to(pt * self.px_per_pt());
        }
        if keys_logged() && incoming.opens.is_none() {
            eprintln!(
                "saved: set {:.0} ms, shown {:.0} ms after, heading {:.1} pt",
                incoming.set_ms,
                incoming.since.elapsed().as_secs_f64() * 1000.0,
                self.heading() / self.px_per_pt()
            );
        }
        self.words = Some(incoming.lines);
        // A jump back to another file puts the cursor back on its line.
        let landed = self
            .following
            .as_ref()
            .and_then(|following| match following.landing {
                Landing::At(_, aim) => Some(aim),
                _ => None,
            });
        if let Some(file) = incoming.opens {
            self.shown_now(file);
            self.aim = landed;
        }
        self.read_layout();
        self.refind();
    }

    /// Another file is the one shown: its marks come back, '' starts over,
    /// and the launcher's Recent is told.
    fn shown_now(&mut self, file: std::path::PathBuf) {
        self.following = None;
        self.aim = None;
        self.marks.clear();
        self.kept_marks = place::read(&file)
            .map(|kept| kept_marks(&kept))
            .unwrap_or_default();
        self.take_kept_marks();
        self.last_jump = None;
        self.selected = None;
        if !self.measuring
            && let Err(error) = showing::keep(&file)
        {
            eprintln!("reader: can't say {} is shown: {error}", file.display());
        }
        if !self.measuring && !start::is(&file) {
            showing::opened(&file);
        }
        if keys_logged() {
            eprintln!(
                "opened: {}, heading {:.1} pt",
                file.display(),
                self.offset / self.px_per_pt()
            );
        }
        self.file = file;
    }

    /// Takes back the marks kept from the last time whose blocks the page
    /// shown has.
    pub(crate) fn take_kept_marks(&mut self) {
        if self.kept_marks.is_empty() {
            return;
        }
        let starts = place::starts(&self.painter.page().frame);
        let marks = &mut self.marks;
        self.kept_marks.retain(|(letter, block, into_pt)| {
            match place::pt_of(&starts, *block, *into_pt) {
                Some(pt) => {
                    marks.entry(*letter).or_insert(pt);
                    false
                }
                None => true,
            }
        });
    }

    /// Keeps where the view is on the file shown, and its marks, for the
    /// next time it opens, each with the heading it sits under and how far
    /// down the page it is, for the launcher to say.
    pub(crate) fn keep_place(&self) {
        if let Err(error) = place::keep_window(f64::from(self.size.0) * PT_A_PX) {
            eprintln!("reader: can't keep the window's width: {error}");
        }
        let file = self.file.as_path();
        let blocks = place::blocks(&self.painter.page().frame);
        let starts: Vec<(usize, f64)> = blocks
            .iter()
            .map(|block| (block.number, block.top_pt))
            .collect();
        let px_per_pt = self.px_per_pt();
        // Closed while a jump moves, it keeps where the keys sent it.
        let here = self.jump.as_ref().map_or(self.offset, |jump| jump.to) / px_per_pt;
        // How far down is said only of the whole file's page: the first
        // screen's ends where its setting stopped, not where the file does.
        let most = self.whole.then(|| self.most() / px_per_pt);
        let said = |block: usize, pt: f64| place::Said {
            heading: place::heading_of(&blocks, block),
            through: most.map(|most| place::through(pt, most)),
        };
        let (block, into_pt) = place::at(&starts, here);
        // A mark is its line's top, and the line is found by it for its
        // baseline, which says the block it's in; a mark kept where no
        // line is, before the page's lines are read, stands for both.
        // A mark is a point, so its share is of the page's whole height,
        // not of how far the view goes, and known as `most` is.
        let height = self.whole.then(|| self.end_px() / px_per_pt);
        let lines = self.words.as_deref().unwrap_or(&[]);
        let line = |pt: f64| {
            lines
                .iter()
                .find(|line| (moves::top_of(line) - pt).abs() < 0.5)
                .map_or((pt, pt), |line| (pt, line.baseline_pt))
        };
        let mut marks: Vec<place::Mark> = self
            .marks
            .iter()
            .map(|(letter, pt)| place::mark(&blocks, *letter, line(*pt), height))
            .collect();
        marks.sort_by_key(|mark| mark.letter);
        let at_end = self.at_end();
        if let Err(error) = place::write(
            file,
            &place::Place {
                block,
                into_pt,
                at_end,
                marks,
                said: said(block, here),
                left: place::now(),
            },
        ) {
            eprintln!("reader: can't keep where {} was: {error}", file.display());
        }
    }

    /// Whether a page was set at the width the window asks for now.
    pub(crate) fn fits(&self, page: &typst_layout::Page) -> bool {
        (page.frame.size().x.to_pt() - self.width.page_pt()).abs() < 0.01
    }

    pub(crate) fn replace(&mut self, page: typst_layout::Page, ms: f64) {
        // A save set since is newer than the rest of the first set, and
        // a window resized since wants the whole page at its own width.
        if self.reloaded || self.incoming.is_some() || !self.fits(&page) {
            return;
        }
        self.held_pt = Some(self.painter.height_pt());
        self.settled_pt = self.painter.height_pt() - page::bottom_pt() - 1.0;
        self.painter.replace(page);
        self.whole = true;
        let settled = self.settled_pt;
        let below: Vec<Key> = self
            .textures
            .keys()
            .filter(|k| self.painter.foot_pt(**k) > settled)
            .copied()
            .collect();
        self.stale.extend(below);
        self.take_kept_marks();
        let painter = &self.painter;
        self.asked.retain(|k| painter.foot_pt(*k) <= settled);
        self.arrived
            .retain(|band| painter.foot_pt(band.key) <= settled);
        if let Some(measure) = self.measure.as_mut() {
            measure.rest_ms = Some(ms);
        }
        self.read_words();
        self.read_layout();
        if !self.frame_pending {
            self.draw();
        }
    }

    pub(crate) fn arrive(&mut self, band: Band) {
        // A saved page's band waits with the saved page; one of a save set
        // over since, or of a page let go, is let go.
        if band.version > self.painter.version {
            let key = self.zoom_key();
            if let Some(incoming) = self.incoming.as_mut()
                && incoming.version == band.version
            {
                incoming.asked.remove(&band.key);
                if band.key.zoom == key {
                    incoming.arrived.push_back(band);
                }
                if !self.frame_pending {
                    self.draw();
                }
            }
            return;
        }
        // Drawn from the first screen's page below where it settled: wrong
        // now, and asked again already.
        if band.version != self.painter.version && self.painter.foot_pt(band.key) > self.settled_pt
        {
            return;
        }
        self.asked.remove(&band.key);
        // A band for a zoom since passed would only wait to be uploaded and
        // then let go: a glide's stale bands held 500 MB that way. The
        // stand-in's are wanted at any zoom.
        let key = self.zoom_key();
        let wanted = |zoom: u32| zoom == key || zoom == STAND_IN;
        self.arrived.retain(|queued| wanted(queued.key.zoom));
        let had = (self.textures.contains_key(&band.key) && !self.stale.contains(&band.key))
            || self.loading.contains(&band.key)
            || self.arrived.iter().any(|queued| queued.key == band.key);
        if wanted(band.key.zoom) && !had {
            self.arrived.push_back(band);
        }
        if !self.frame_pending {
            self.draw();
        }
    }
}
