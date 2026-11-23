//! Where each file was left and what comes in: the place and its marks kept as
//! the reader closes, a page set again taken in, and where the view lands on
//! it.

use super::{Incoming, Landing, Reset, STAND_IN};
use crate::numbers::u32_of;
use crate::painter::{BAND_PX, Band, Key};
use crate::theme::PT_A_PX;
use crate::{
    App, anchor, kept_marks, keys_logged, links, moves, page, place, showing, start, text,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::time::Instant;

/// The bands of a page `height` pixels tall, each `band` pixels, that a
/// view `offset` pixels down it and `screen` pixels tall shows.
fn bands_over(offset: f64, screen: f64, band: f64, height: f64) -> Range<u32> {
    let first = u32_of((offset / band).floor().max(0.0));
    let last = u32_of(((offset + screen) / band).ceil()).min(u32_of((height / band).ceil()));
    first..last
}

impl App {
    /// The whole page has been set: draw from it, keeping each band above
    /// where the first screen's page settled and redrawing the rest in place.
    /// A save's page, set: it draws while the page before shows, from where
    /// the view's top words are on it.
    /// A page the reloader set: the file shown, saved, or the file being
    /// opened in its place. One of a file no longer wanted is let go.
    pub(crate) fn saved(&mut self, (file, edited, set): Reset) {
        let file = std::path::PathBuf::from(file);
        let opens = match &self.following {
            Some(following) if following.file == file => true,
            None if file == self.file => false,
            _ => return,
        };
        let (page, ms, lines, title) = match set {
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
            let (pt, missing) = self.landing(&file, &page, &lines, new_most_pt);
            if let Some(anchor) = missing {
                self.say(&format!("E426: Tag not found: #{anchor}"));
            }
            pt
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
            title,
            edited,
        });
        self.ask_incoming();
    }

    /// Where the view lands on a file opened in this one's place, in
    /// points: the heading a link names, the place a jump left, or where
    /// he last left the file, and otherwise its top. A heading the file
    /// doesn't have is given back to be said, and the file opens where he
    /// left it, as it would with none named.
    pub(crate) fn landing(
        &self,
        file: &std::path::Path,
        page: &typst_layout::Page,
        lines: &[text::Line],
        most_pt: f64,
    ) -> (f64, Option<String>) {
        let Some(following) = self.following.as_ref() else {
            return (0.0, None);
        };
        let kept = || match place::read(file) {
            Some(kept) if kept.at_end => most_pt,
            Some(kept) => {
                let starts = place::starts(&page.frame);
                place::pt_of(&starts, kept.block, kept.into_pt, kept.block_pt).unwrap_or(0.0)
            }
            None => 0.0,
        };
        let (pt, missing) = match &following.landing {
            Landing::Heading(anchor) => match links::heading_at(lines, anchor) {
                Some(pt) => (pt, None),
                None => (kept(), Some(anchor.clone())),
            },
            Landing::At(pt, _) => (*pt, None),
            Landing::Kept => (kept(), None),
        };
        (pt.clamp(0.0, most_pt), missing)
    }

    /// The bands the saved page needs to cover the view where it will show,
    /// at the zoom shown.
    pub(crate) fn ask_incoming(&mut self) {
        let key = self.zoom_key();
        let screen = self.screen_px();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let scale = self.painter.scale(key);
        let Some(incoming) = self.incoming.as_mut() else {
            return;
        };
        let height = incoming.page.frame.size().y.to_pt() * scale;
        for index in bands_over(incoming.offset, screen, band, height) {
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
    pub(crate) fn incoming_covers(&self) -> bool {
        let Some(incoming) = self.incoming.as_ref() else {
            return false;
        };
        let key = self.zoom_key();
        let band = f64::from(BAND_PX) * self.level() * 1000.0 / f64::from(key);
        let height = incoming.page.frame.size().y.to_pt() * self.painter.scale(key);
        bands_over(incoming.offset, self.screen_px(), band, height)
            .all(|index| incoming.textures.contains_key(&Key { zoom: key, index }))
    }

    /// The saved page takes the page's place, whole, with the view on the
    /// same words, and everything of the page before is let go.
    pub(crate) fn take_incoming(&mut self) {
        let Some(incoming) = self.incoming.take() else {
            return;
        };
        // Its title, which a save may have changed and a file opened in
        // this one's place always does; niri tells nobody of one unchanged.
        self.window.set_title(incoming.title.clone());
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
        // mark, and the cursor's line, goes as far through its block from
        // the line it's on. Set again at the same width, saved or with its
        // pictures come, they stay where they were, as the words below them
        // may have changed. A kept mark where it landed lands again from
        // what was kept either way, unless the file was edited.
        let reflowed = incoming.opens.is_none()
            && (incoming.page.frame.size().x.to_pt() - self.painter.width_pt()).abs() >= 0.01;
        let before = reflowed.then(|| place::starts(&self.painter.page().frame));
        self.painter.take(incoming.page, incoming.version);
        if incoming.opens.is_none() && (reflowed || !self.landed.is_empty()) {
            let after = place::starts(&self.painter.page().frame);
            let was = self.words.as_deref().unwrap_or(&[]);
            let now: Vec<(f64, f64)> = incoming.lines.iter().map(moves::ends_of).collect();
            let moved = |pt: f64| match &before {
                Some(before) => {
                    let line = moves::line_topped(was, pt);
                    place::moved(before, &after, line, &now).unwrap_or(pt)
                }
                None => pt,
            };
            place::set_again(
                &mut self.landed,
                &mut self.marks,
                &after,
                incoming.edited,
                moved,
            );
            if let Some(pt) = self.aim.as_mut() {
                *pt = moved(*pt);
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
        } else {
            // The whole page, set again at another width or saved, has the
            // blocks the first screen's marks waited for.
            self.take_kept_marks();
        }
        self.marks_on_lines();
        self.read_layout();
        self.refind();
    }

    /// Another file is the one shown: its marks come back, '' starts over,
    /// and the launcher's Recent is told.
    fn shown_now(&mut self, file: std::path::PathBuf) {
        self.following = None;
        self.aim = None;
        self.marks.clear();
        self.landed.clear();
        self.kept_marks = place::read(&file)
            .map(|kept| kept_marks(&kept))
            .unwrap_or_default();
        self.take_kept_marks();
        self.last_jump = None;
        self.selected = None;
        // The cursor's character, the selection and the lines read were
        // the other file's, and opening a file ends visual mode, as :e
        // does. Until this one's are read, keys act as before the first
        // read, or v would start on the other file's lines.
        self.character = None;
        self.drop_selection();
        self.layout = None;
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
    /// shown has. The first screen's page often ends on a mark's block,
    /// whose height is known only once the next is set, so a mark kept
    /// with its block's height waits for the whole page there.
    pub(crate) fn take_kept_marks(&mut self) {
        self.take_kept_marks_as(self.whole);
    }

    /// The rest of the page couldn't be set, so no block past the first
    /// screen's comes: marks waiting on one land by their kept distance,
    /// as one kept without its height does, rather than wait all session.
    pub(crate) fn rest_failed(&mut self) {
        self.take_kept_marks_as(true);
    }

    /// Takes back the kept marks whose blocks the page has, `whole` when
    /// no block past the page's comes.
    fn take_kept_marks_as(&mut self, whole: bool) {
        if self.kept_marks.is_empty() {
            return;
        }
        let starts = place::starts(&self.painter.page().frame);
        place::take_kept(
            &mut self.kept_marks,
            &mut self.marks,
            &mut self.landed,
            &starts,
            whole,
        );
        self.marks_on_lines();
    }

    /// Puts each kept mark landed as far through its block on the line of
    /// that block nearest it, once the page's lines are read.
    pub(crate) fn marks_on_lines(&mut self) {
        // Most often none waits, and the page's blocks needn't be walked.
        if self.landed.iter().all(|one| one.on_line) {
            return;
        }
        let Some(lines) = self.words.as_deref() else {
            return;
        };
        let lines: Vec<(f64, f64)> = lines.iter().map(moves::ends_of).collect();
        let starts = place::starts(&self.painter.page().frame);
        for (letter, top) in place::on_lines(&mut self.landed, &self.marks, &starts, &lines) {
            self.marks.insert(letter, top);
        }
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
        // baseline, which says the block it's in; a mark set where no line
        // is stands for both. A kept mark where it landed goes back as it
        // was read, its block's height and all. A mark is a point, so its
        // share is of the page's whole height, not of how far the view
        // goes, and known as `most` is.
        let height = self.whole.then(|| self.end_px() / px_per_pt);
        let lines = self.words.as_deref().unwrap_or(&[]);
        let mut marks: Vec<place::Mark> = self
            .marks
            .iter()
            .map(
                |(letter, pt)| match place::as_read(&self.landed, *letter, *pt) {
                    Some(kept) => kept.clone(),
                    None => place::mark(&blocks, *letter, moves::line_topped(lines, *pt), height),
                },
            )
            .collect();
        marks.extend(place::still_waiting(
            &self.kept_marks,
            self.whole,
            |letter| self.marks.contains_key(&letter),
        ));
        marks.sort_by_key(|mark| mark.letter);
        let at_end = self.at_end();
        if let Err(error) = place::write(
            file,
            &place::Place {
                block,
                into_pt,
                block_pt: place::height_of(&starts, block),
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
