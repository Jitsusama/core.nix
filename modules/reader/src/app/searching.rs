//! Searching the page: the prompt's words read, each match shown, and the
//! search typed, run again and logged.

use super::REFERENCE_SCALE;
use crate::{App, keys_logged, prompt, search, text};

impl App {
    /// Device pixels a point of the prompt takes: the page's at zoom 1.
    pub(crate) fn prompt_px_per_pt(&self) -> f64 {
        self.painter.base * self.scale / REFERENCE_SCALE
    }

    /// Whether a match shows whole in the view it's heading for, above
    /// the prompt's row.
    pub(crate) fn in_view(&self, i: usize) -> bool {
        let px = self.px_per_pt();
        let found = &self.found[i];
        let top = search::top_pt(found) * px;
        let foot = search::foot_pt(found) * px;
        let room = self.screen_px() - prompt::ROW_PT * self.prompt_px_per_pt();
        let view = self.heading();
        top >= view && foot <= view + room
    }

    /// Brings a match into view, a third of the way down, unless it shows.
    fn show_match(&mut self, i: usize, leap: bool) {
        if self.in_view(i) {
            return;
        }
        let to = search::top_pt(&self.found[i]) * self.px_per_pt() - self.screen_px() / 3.0;
        if leap {
            self.leap(to);
        } else {
            self.step_to(to);
        }
    }

    /// Reads the page's lines for search on a thread of its own.
    pub(crate) fn read_words(&self) {
        let page = self.painter.page();
        let version = self.painter.version;
        let to = self.words_to.clone();
        let _ = std::thread::Builder::new()
            .name("words".into())
            .spawn(move || {
                let _ = to.send((version, text::lines(text::runs(&page.frame))));
            });
    }

    pub(crate) fn words_read(&mut self, version: u32, lines: Vec<text::Line>) {
        if version != self.painter.version {
            return;
        }
        if let Some(needle) = std::env::var_os("READER_WORDS_DUMP") {
            let needle = needle.to_string_lossy().to_lowercase();
            for line in lines
                .iter()
                .filter(|line| line.text.to_lowercase().contains(&needle))
            {
                eprintln!(
                    "words v{version} {:.1},{:.1} {:?}",
                    line.x_pt, line.baseline_pt, line.text
                );
            }
        }
        self.words = Some(lines);
        self.refind();
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Finds what is typed, or the last search, again. While typing, the
    /// view shows the match from where the search began, as incsearch.
    pub(crate) fn refind(&mut self) {
        let (query, forward, from_pt) = match (&self.typing, &self.searched) {
            (Some(typing), _) => (typing.typed.clone(), typing.forward, typing.from_pt),
            (None, Some((query, forward))) => {
                (query.clone(), *forward, self.heading() / self.px_per_pt())
            }
            (None, None) => return,
        };
        let smartcase = self.typing.is_some() || !self.word_search;
        self.found = self
            .words
            .as_ref()
            .map_or_else(Vec::new, |lines| text::find_as(lines, &query, smartcase));
        self.current = search::from(&self.found, from_pt, forward);
        if self.typing.is_some() {
            self.highlighting = true;
            self.step_to(from_pt * self.px_per_pt());
            if let Some(i) = self.current {
                self.show_match(i, false);
            }
        }
        if self.typing.is_some() || self.prompt_shown {
            self.ask_prompt();
        }
    }

    /// n and N: on to the next match the last search's way, or back.
    pub(crate) fn go_on(&mut self, same_way: bool, times: u32) {
        let Some((_, forward)) = self.searched.clone() else {
            return;
        };
        let forward = if same_way { forward } else { !forward };
        self.highlighting = true;
        let count = self.found.len();
        if count > 0 {
            let view_pt = self.heading() / self.px_per_pt();
            let (mut i, left) = match self.current.filter(|&i| self.in_view(i)) {
                Some(i) => (i, times),
                None => (
                    search::from(&self.found, view_pt, forward).unwrap_or(0),
                    times.saturating_sub(1),
                ),
            };
            for _ in 0..(left as usize % count) {
                i = search::step(count, i, forward);
            }
            self.current = Some(i);
            self.show_match(i, true);
            self.aim = Some(search::top_pt(&self.found[i]));
        }
        self.ask_prompt();
    }

    /// Takes a key while a search is typed.
    pub(crate) fn typed(&mut self, said: search::Typed) {
        match said {
            search::Typed::Changed => self.refind(),
            search::Typed::Done => {
                let Some(typing) = self.typing.take() else {
                    return;
                };
                // An empty search is the last one again, as in Neovim.
                let query = if typing.typed.is_empty() {
                    self.searched.as_ref().map(|(query, _)| query.clone())
                } else {
                    Some(typing.typed.clone())
                };
                if let Some(query) = query {
                    self.searched = Some((query, typing.forward));
                    self.word_search = false;
                    self.step_to(typing.from_pt * self.px_per_pt());
                    self.remember(typing.from_pt);
                    self.refind();
                    if let Some(i) = self.current {
                        self.show_match(i, false);
                        self.aim = Some(search::top_pt(&self.found[i]));
                    }
                } else {
                    self.prompt_shown = false;
                }
            }
            search::Typed::Cancelled => {
                if let Some(typing) = self.typing.take() {
                    self.step_to(typing.from_pt * self.px_per_pt());
                }
                // The last search's matches come back, as Neovim's do.
                self.refind();
                self.highlighting = self.searched.is_some();
                self.prompt_shown = false;
            }
            search::Typed::Nothing => {}
        }
        // A search made while selecting takes the cursor to its match, as
        // Neovim's does.
        if matches!(said, search::Typed::Done | search::Typed::Cancelled) {
            self.after_view_moved(said == search::Typed::Done);
        }
        self.log_search(&format!("{said:?}"));
        if !self.frame_pending {
            self.draw();
        }
    }

    pub(crate) fn log_search(&self, what: &str) {
        if keys_logged() {
            let typed = self.typing.as_ref().map(|typing| typing.typed.as_str());
            eprintln!(
                "search {what} {typed:?} {:?}: {} found, on {:?}, heading {:.1} pt, marked {}, prompt {}",
                self.searched,
                self.found.len(),
                self.current,
                self.heading() / self.px_per_pt(),
                self.highlighting,
                self.prompt_shown
            );
        }
    }

    /// Asks for the prompt as it should read now.
    /// A command typed comes first, then a search, then one through gO's
    /// outline, then what a command said, then the last search.
    pub(crate) fn ask_prompt(&mut self) {
        let way = |forward: bool| if forward { "/" } else { "?" };
        let (mark, typed, cursor, counted) = if let Some(typing) = &self.commanding {
            (":", typing.typed.clone(), true, false)
        } else if let Some(typing) = &self.typing {
            (way(typing.forward), typing.typed.clone(), true, true)
        } else if let Some(typing) = &self.outline_typing {
            (way(typing.forward), typing.typed.clone(), true, false)
        } else if let Some(said) = &self.said {
            ("", said.clone(), false, false)
        } else if let Some((query, forward)) = &self.searched {
            (way(*forward), query.clone(), false, true)
        } else {
            return;
        };
        // The command line's foot says how to choose from the menu, then
        // what Return runs once a line is chosen, as the launcher's does.
        let note = match (&self.commanding, &self.menu) {
            (Some(_), Some(menu)) if !self.menu_away && menu.chosen.is_some() => {
                format!("\u{21b5} {}", menu.line().trim())
            }
            (Some(_), Some(menu)) if !self.menu_away && !menu.lines.is_empty() => {
                "Tab choose".to_string()
            }
            _ => String::new(),
        };
        let px_per_pt = self.prompt_px_per_pt();
        let (Some(prompt), Some(gl)) = (self.prompt.as_ref(), self.gl.as_ref()) else {
            return;
        };
        self.prompt_serial += 1;
        prompt.ask(prompt::Ask {
            serial: self.prompt_serial,
            keys: false,
            labels: Vec::new(),
            outline: Vec::new(),
            menu: None,
            note,
            mark,
            typed,
            cursor,
            count: self
                .words
                .as_ref()
                .filter(|_| counted)
                .map(|_| (self.current.map_or(0, |i| i + 1), self.found.len())),
            width_pt: f64::from(gl.width) / px_per_pt,
            px_per_pt,
            page: self.width,
        });
        self.prompt_shown = true;
    }
}
