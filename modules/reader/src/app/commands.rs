//! The command line's side: the file followed, what it says, the commands it
//! runs and their history.

use super::{Landing, desktop_open};
use crate::{App, ex, keys_logged, links, outline, search};

impl App {
    /// gf: with the cursor on a character, goes by the name under it or
    /// after it on its line, by a link or written, and none behind it, or
    /// says it can't find that one, as vi's gf does with the name under the
    /// cursor; with the cursor on a whole line, by the first link from it
    /// down to another file, or the first path written from there that is
    /// a file. A link goes on to the heading it names, here or in its file,
    /// as Ctrl+] goes; a markdown file opens in this window, at its heading
    /// or where it was left, and anything else as the desktop opens it.
    pub(crate) fn follow_file(&mut self) {
        if self.character_waits() {
            return self.say(super::STILL_READING);
        }
        let page = self.painter.page();
        let is_file = std::path::Path::is_file;
        let going = if let Some((line, x_pt)) = self.character_on() {
            let pieces = links::pieces(&page.frame);
            match links::file_here(&pieces, line, x_pt, &self.file, is_file) {
                Ok(going) => Some(going),
                Err(None) => None,
                Err(Some(said)) => {
                    return self.say(&format!("E447: Can't find file \"{said}\" in path"));
                }
            }
        } else {
            let lines = self.words.as_deref().unwrap_or_default();
            let (from, (_, foot)) = (self.cursor_pt(), self.view_pt());
            links::going_down(
                links::links(&page.frame),
                lines,
                (from, foot),
                &self.file,
                is_file,
            )
        };
        match going {
            None => self.say("E446: No file name under cursor"),
            Some(links::Going::Here) => {}
            Some(links::Going::Heading(anchor)) => self.follow(&format!("#{anchor}")),
            Some(links::Going::File(file, heading)) => {
                if keys_logged() {
                    eprintln!("file: {}", file.display());
                }
                if links::markdown(&file) {
                    self.remember(self.heading() / self.px_per_pt());
                    self.open(file, heading.map_or(Landing::Kept, Landing::Heading));
                } else {
                    desktop_open(&file.to_string_lossy());
                }
            }
        }
    }

    /// Says something in the prompt's row until the next key, as Neovim
    /// says an error on its command line.
    pub(crate) fn say(&mut self, said: &str) {
        if keys_logged() {
            eprintln!("said: {said}");
        }
        self.said = Some(said.to_string());
        self.ask_prompt();
    }

    /// Takes a key while a command is typed after :, doing it on Return.
    pub(crate) fn commanded(&mut self, said: search::Typed) {
        match said {
            search::Typed::Changed => {
                self.menu_away = false;
                self.narrow_menu();
                self.ask_prompt();
            }
            search::Typed::Done => {
                let Some(typing) = self.commanding.take() else {
                    return;
                };
                self.close_menu();
                self.prompt_shown = false;
                self.remember_line(&typing.typed);
                self.run_line(&typing.typed);
            }
            search::Typed::Cancelled => {
                self.commanding = None;
                self.close_menu();
                self.prompt_shown = false;
            }
            search::Typed::Nothing => {}
        }
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Keeps a line run for :his and @:, as Vim's history keeps them: once
    /// each, the newest last, fifty at most, as the launcher's are.
    pub(crate) fn remember_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.earlier.retain(|earlier| earlier != line);
        self.earlier.push(line.to_string());
        let over = self.earlier.len().saturating_sub(EARLIER_KEPT);
        self.earlier.drain(..over);
    }

    /// @: the last line run, run again, or Vim's E30 with none yet.
    pub(crate) fn again_command(&mut self) {
        match self.earlier.last().cloned() {
            Some(line) => self.run_line(&line),
            None => self.say("E30: No previous command line"),
        }
    }

    /// :his: the lines run, oldest first and the newest chosen, each by its
    /// number as Vim's :his numbers them; Return runs the one chosen.
    fn list_history(&mut self) {
        let rows: Vec<outline::Row> = self
            .earlier
            .iter()
            .enumerate()
            .map(|(at, line)| outline::Row {
                level: 1,
                words: format!("{}  {line}", at + 1),
                to: outline::To::Command(line.clone()),
            })
            .collect();
        if rows.is_empty() {
            return self.say("E30: No previous command line");
        }
        let newest = rows.len() - 1;
        self.list(rows, newest);
    }

    /// Runs a command line's words, as Return on it does.
    pub(crate) fn run_line(&mut self, typed: &str) {
        let command = ex::parse(typed);
        match &command {
            ex::Command::Go(range) => self.go_to_range(*range),
            ex::Command::Quit => self.exit = true,
            ex::Command::NoHighlight => self.highlighting = false,
            ex::Command::Change => self.say(ex::CANNOT_CHANGE),
            ex::Command::Copy(range) => self.copy_range(*range),
            ex::Command::Edit(file) => self.edit(file.as_deref()),
            ex::Command::File { on, times } => self.file_beside(*on, *times),
            ex::Command::Help => self.show_keys(true),
            ex::Command::Global { pattern, range } => self.list_global(pattern, *range),
            ex::Command::Marks(letters) => self.list_marks(letters),
            ex::Command::Jumps => self.list_jumps(),
            ex::Command::History => self.list_history(),
            ex::Command::Nothing => {}
            ex::Command::Refused(said) => self.say(said),
        }
        if keys_logged() {
            eprintln!(
                "command: {typed:?} is {command:?}: heading {:.1} pt",
                self.heading() / self.px_per_pt()
            );
        }
    }
}

/// How many lines the command line's history keeps, as the launcher's.
const EARLIER_KEPT: usize = 50;
