//! The command line's side: the file followed, what it says, the commands it
//! runs and their history.

use super::{Landing, desktop_open};
use crate::{App, ex, keys_logged, links, moves, outline, search};

impl App {
    /// gf: with the cursor on a character, opens the file named under it or
    /// after it on its line, by a link or written, and none behind it, or
    /// says it can't find that one, as vi's gf does with the name under the
    /// cursor; with the cursor on a whole line, the file the first link
    /// from it down names, or the first path written from there that is a
    /// file: markdown in this window, and anything else as the desktop
    /// opens it.
    pub(crate) fn follow_file(&mut self) {
        if self.character_waits() {
            return self.say(super::STILL_READING);
        }
        let page = self.painter.page();
        // The file a name stands for, when it's one on this machine.
        let file_of =
            |name: &links::Name| links::file_of(name, &self.file).filter(|file| file.is_file());
        let found = if let Some((line, x_pt)) = self.character_on() {
            let pieces = links::pieces(&page.frame);
            match links::file_here(&pieces, line, x_pt, &self.file, std::path::Path::is_file) {
                Ok(file) => Some(file),
                Err(None) => return self.say("E446: No file name under cursor"),
                Err(Some(said)) => {
                    return self.say(&format!("E447: Can't find file \"{said}\" in path"));
                }
            }
        } else {
            let (from, (_, foot)) = (self.cursor_pt(), self.view_pt());
            let mut named: Vec<(f64, f64, std::path::PathBuf)> = links::links(&page.frame)
                .into_iter()
                .filter(|link| link.baseline_pt > from && link.top_pt < foot)
                // A heading here is Ctrl+]'s to go to; gf from a line wants
                // another file.
                .filter(|link| {
                    !matches!(
                        links::target(&link.href, &self.file),
                        links::Target::Here(_)
                    )
                })
                .filter_map(|link| {
                    let file = file_of(&links::Name::Link(link.href))?;
                    Some((link.baseline_pt.round(), link.left_pt, file))
                })
                .collect();
            // A path written on a line comes after the links on it.
            for line in self.words.as_deref().unwrap_or_default() {
                if line.baseline_pt <= from || moves::foot_of(line) > foot {
                    continue;
                }
                for (path, ..) in moves::written_paths(line) {
                    if let Some(file) = file_of(&links::Name::Written(path)) {
                        named.push((line.baseline_pt.round(), f64::INFINITY, file));
                    }
                }
            }
            named.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
            named.into_iter().next().map(|(.., file)| file)
        };
        let Some(file) = found else {
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
