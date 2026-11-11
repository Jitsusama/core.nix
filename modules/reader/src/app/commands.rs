//! The command line's side: the file followed, what it says, the commands it
//! runs and their history.

use super::{Landing, desktop_open, folder_of};
use crate::{App, ex, keys_logged, links, moves, outline, search};

impl App {
    /// gf: opens the file the first link from the cursor's line down names,
    /// or the first path written from there that is a file, as vi's gf
    /// opens the one named under the cursor: markdown in this window, and
    /// anything else as the desktop opens it.
    pub(crate) fn follow_file(&mut self) {
        let (_, foot) = self.view_pt();
        let from = self.cursor_pt();
        let folder = folder_of(&self.file).to_path_buf();
        let mut named: Vec<(f64, f64, std::path::PathBuf)> =
            links::links(&self.painter.page().frame)
                .into_iter()
                .filter(|link| link.baseline_pt > from && link.top_pt < foot)
                .filter_map(|link| {
                    let file = match links::target(&link.href, &self.file) {
                        links::Target::Page(file, _) => file,
                        links::Target::Elsewhere(to) if !to.contains("://") => to.into(),
                        _ => return None,
                    };
                    file.is_file()
                        .then_some((link.baseline_pt.round(), link.left_pt, file))
                })
                .collect();
        // A path written on a line comes after the links on it.
        for line in self.words.as_deref().unwrap_or_default() {
            if line.baseline_pt <= from || moves::foot_of(line) > foot {
                continue;
            }
            for path in moves::written_paths(line) {
                let file = folder.join(&path);
                if file.is_file() {
                    named.push((line.baseline_pt.round(), f64::INFINITY, file));
                }
            }
        }
        named.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let Some((_, _, file)) = named.into_iter().next() else {
            self.say("E446: No file name under cursor");
            return;
        };
        if keys_logged() {
            eprintln!("file: {}", file.display());
        }
        if links::markdown(&file) {
            self.remember(self.heading() / self.px_per_pt());
            self.open(file, Landing::Kept);
        } else {
            desktop_open(&file.to_string_lossy());
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
