//! The menus and lists: the files, the global and local marks, the jumps and
//! the edits, each asked for, narrowed and closed.

use super::{Landing, desktop_open};
use crate::{App, ex, keys, keys_logged, links, menu, moves, outline, place, prompt};

impl App {
    /// The menu narrowed to what the command line holds now.
    pub(crate) fn narrow_menu(&mut self) {
        let Some(typing) = self.commanding.as_ref() else {
            return;
        };
        let row = self.cursor_file_line();
        let lines = menu::lines(&self.menu_entries, &typing.typed, row, &menu::files_for);
        self.menu = Some(menu::Menu::new(&typing.typed, lines));
        self.ask_menu();
    }

    /// The line of the file the cursor's piece starts on, from 1, which
    /// :{n}'s line writes.
    fn cursor_file_line(&self) -> usize {
        let (_, foot) = self.view_pt();
        self.layout
            .as_ref()
            .and_then(|layout| {
                let unit = layout.piece_at(self.cursor_pt(), foot)?;
                layout.file_lines_of(&layout.pieces_from(unit, 1)?)
            })
            .map_or(1, |(first, _)| first)
    }

    /// A key of the menu's on the command line: Tab, the arrows, Ctrl+N
    /// and Ctrl+P choose a line and write it in, Ctrl+Y takes it as if
    /// typed, and Ctrl+E puts the menu away with what was typed back.
    pub(crate) fn menu_key(&mut self, key: keys::LineKey) {
        let away = self.menu_away;
        let (Some(menu), Some(typing)) = (self.menu.as_mut(), self.commanding.as_mut()) else {
            return;
        };
        match key {
            keys::LineKey::NextRow | keys::LineKey::PreviousRow if !away => {
                let by = if key == keys::LineKey::NextRow { 1 } else { -1 };
                typing.typed = menu.step(by);
            }
            keys::LineKey::TakeName => typing.typed = menu.line(),
            keys::LineKey::MenuAway => {
                typing.typed.clone_from(&menu.typed);
                menu.chosen = None;
            }
            _ => {}
        }
        match key {
            keys::LineKey::TakeName => {
                self.menu_away = false;
                self.narrow_menu();
            }
            keys::LineKey::MenuAway => {
                self.menu_away = true;
                self.ask_menu();
            }
            _ => self.ask_menu(),
        }
        self.ask_prompt();
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Asks for the menu set as it stands, or puts it away when it shows
    /// nothing.
    fn ask_menu(&mut self) {
        let px_per_pt = self.prompt_px_per_pt();
        let showing = self
            .menu
            .as_ref()
            .filter(|menu| !self.menu_away && !menu.lines.is_empty());
        let (Some(menu), Some(prompt), Some(gl)) =
            (showing, self.prompt.as_ref(), self.gl.as_ref())
        else {
            self.forget_over_list();
            return;
        };
        if keys_logged() {
            eprintln!(
                "menu: {} lines, chosen {:?}",
                menu.lines.len(),
                menu.chosen.map(|at| menu.lines[at].called.as_str())
            );
        }
        let all = self
            .menu_entries
            .iter()
            .map(|entry| menu::notation(&entry.command, &entry.short))
            .chain(menu.lines.iter().map(|line| line.called.clone()))
            .collect();
        prompt.ask(prompt::Ask {
            serial: 0,
            keys: false,
            labels: Vec::new(),
            outline: Vec::new(),
            menu: Some(prompt::Menu {
                lines: menu
                    .shown()
                    .map(|(line, chosen)| {
                        (
                            line.called.clone(),
                            line.title.clone(),
                            line.key.clone(),
                            chosen,
                        )
                    })
                    .collect(),
                all,
            }),
            note: String::new(),
            mark: "",
            typed: String::new(),
            cursor: false,
            count: None,
            width_pt: f64::from(gl.width) / px_per_pt,
            px_per_pt,
            page: self.width,
        });
    }

    /// Puts the menu away with the command line.
    pub(crate) fn close_menu(&mut self) {
        self.menu = None;
        self.menu_away = false;
        self.forget_over_list();
    }

    /// Whether the menu shows over the command line.
    pub(crate) fn menu_shows(&self) -> bool {
        self.commanding.is_some()
            && !self.menu_away
            && self
                .menu
                .as_ref()
                .is_some_and(|menu| !menu.lines.is_empty())
    }

    /// Lets go of what stands over the prompt's row, gO's outline or the
    /// menu, so the next to show never shows the last's pixels.
    fn forget_over_list(&mut self) {
        self.stop_reading();
        self.outline_set = None;
        if let (Some(gl), Some(old)) = (self.gl.as_ref(), self.outline_canvas.take()) {
            gl.forget(old);
        }
    }

    /// A range alone, as :12, :$ and :'<,'> go: to its last line, as a
    /// jump, the view moving as Vim's does; the file's end for $.
    pub(crate) fn go_to_range(&mut self, range: ex::Range) {
        let line = match range {
            ex::Range::Here => return,
            ex::Range::Lines(_, last) => last,
            ex::Range::Selected => match self.selected {
                Some((_, last)) => last,
                None => return self.say(ex::NOTHING_SELECTED),
            },
        };
        self.remember(self.heading() / self.px_per_pt());
        if line == usize::MAX {
            self.land(f64::INFINITY);
        } else {
            let blocks = place::blocks(&self.painter.page().frame);
            let pt = moves::source_line(&blocks, line).unwrap_or(0.0);
            // The line it's set on is the first whose foot is past it, since
            // a heading's ink stands above the top of its block.
            let top = self
                .words
                .as_deref()
                .and_then(|lines| lines.iter().find(|l| moves::foot_of(l) > pt + 0.5))
                .map_or(pt, moves::top_of);
            self.bring(top);
        }
    }

    /// Shows a list in gO's place, its rows moved through and gone to as
    /// the outline's are.
    pub(crate) fn list(&mut self, rows: Vec<outline::Row>, chosen: usize) {
        self.outline = outline::Outline::of(rows, chosen);
        self.ask_outline();
    }

    /// :g/pat/: the pieces of the range it matches, each by the line its
    /// first match is on, as / matches; an empty pattern is the last
    /// search's, as Vim's is.
    pub(crate) fn list_global(&mut self, pattern: &str, range: ex::Range) {
        let Some(layout) = self.layout.clone() else {
            return;
        };
        let (from, to) = match range {
            ex::Range::Here => return,
            ex::Range::Lines(from, to) => (from, to),
            ex::Range::Selected => match self.selected {
                Some(lines) => lines,
                None => return self.say(ex::NOTHING_SELECTED),
            },
        };
        let pattern = match (pattern, &self.searched) {
            ("", Some((query, _))) => query.clone(),
            ("", None) => return self.say("E35: No previous regular expression"),
            (pattern, _) => pattern.to_string(),
        };
        let rows: Vec<outline::Row> = layout
            .matching(&pattern, from, to)
            .into_iter()
            .map(|(top, words)| outline::Row {
                level: 1,
                words,
                to: outline::To::Line(top),
            })
            .collect();
        if rows.is_empty() {
            // As Vim's :global says it, with no number, unlike a search's.
            return self.say(&format!("Pattern not found: {pattern}"));
        }
        // The list opens on the first match from the cursor's line on.
        let here = self.cursor_pt() - 0.5;
        let chosen = rows
            .iter()
            .position(|row| matches!(row.to, outline::To::Line(top) if top >= here))
            .unwrap_or(0);
        self.list(rows, chosen);
    }

    /// :marks: ' for the last jump, then each letter's, with the words of
    /// the line it's on, as Vim lists them; only the letters given, when
    /// some are.
    pub(crate) fn list_marks(&mut self, letters: &str) {
        let Some(layout) = self.layout.clone() else {
            return;
        };
        let jumped = self.last_jump.as_ref().map_or(0.0, |spot| spot.aim);
        let mut marks: Vec<(char, f64)> = self.marks.iter().map(|(&c, &pt)| (c, pt)).collect();
        marks.sort_by_key(|&(letter, _)| (letter.is_uppercase(), letter));
        let rows: Vec<outline::Row> = std::iter::once(('\'', jumped))
            .chain(marks)
            .filter(|(letter, _)| letters.is_empty() || letters.contains(*letter))
            .map(|(letter, pt)| outline::Row {
                level: 1,
                words: format!("{letter}  {}", layout.words_at(pt).unwrap_or_default()),
                to: outline::To::Line(pt),
            })
            .collect();
        if rows.is_empty() {
            return self.say(&format!("E283: No marks matching \"{letters}\""));
        }
        self.list(rows, 0);
    }

    /// :jumps: the jump list, oldest first, the place Ctrl+O and Ctrl+I
    /// stand at chosen, each by its line's words, or on another file by
    /// the file's name.
    pub(crate) fn list_jumps(&mut self) {
        let Some(layout) = self.layout.clone() else {
            return;
        };
        let rows: Vec<outline::Row> = self
            .jumps
            .iter()
            .map(|spot| outline::Row {
                level: 1,
                words: if links::same(&spot.file, &self.file) {
                    layout.words_at(spot.aim).unwrap_or_default().to_string()
                } else {
                    spot.file.file_name().map_or_else(
                        || spot.file.display().to_string(),
                        |name| name.to_string_lossy().into_owned(),
                    )
                },
                to: outline::To::Jump(spot.file.clone(), spot.pt, spot.aim),
            })
            .collect();
        if rows.is_empty() {
            return self.say("No jumps yet");
        }
        let chosen = self.jump_at.min(rows.len() - 1);
        self.list(rows, chosen);
    }

    /// :e: another file, named from the folder the reader was started in
    /// or from home with ~, as Vim's takes it. Markdown opens in this
    /// window and anything else as the desktop opens it. The file shown,
    /// or none named, is already as it was last written, read again at
    /// every save.
    pub(crate) fn edit(&mut self, named: Option<&str>) {
        let Some(named) = named else {
            return;
        };
        let file = match named.strip_prefix("~/") {
            Some(under) => std::env::var_os("HOME").map_or_else(
                || std::path::PathBuf::from(named),
                |home| std::path::Path::new(&home).join(under),
            ),
            None => std::path::PathBuf::from(named),
        };
        if file.is_dir() {
            return self.say(&format!("\"{named}\" is a directory"));
        }
        if !file.is_file() {
            return self.say(&format!("E484: Can't open file {named}"));
        }
        if links::same(&file, &self.file) {
            return;
        }
        if links::markdown(&file) {
            if keys_logged() {
                eprintln!("file: {}", file.display());
            }
            self.remember(self.heading() / self.px_per_pt());
            self.open(file, Landing::Kept);
        } else {
            desktop_open(&file.to_string_lossy());
        }
    }
}
