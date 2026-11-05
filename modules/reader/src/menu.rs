//! The command line's menu, shown the moment : is typed, as his cmp shows
//! one over Neovim's command line and the launcher's does (Commands.js):
//! the page's own commands, then what every surface has, :q, :help and
//! :{n}. Each line is its command in Vim's help notation, the act's name
//! and its key. Typing narrows it by the command's start, then by a word
//! of the name; after a name and a space, :e lists the files it could
//! open. Tab and the arrows choose a line and write it in, coming round
//! through what was typed; Ctrl+Y takes it and Ctrl+E puts the menu away.
//! Kept apart from the window so it can be tested; prompt.rs sets it.

/// The most lines the menu shows at once, his `pumheight`.
pub const SHOWN: usize = 10;

/// A command the page has, as the menu lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its whole name, and the shortest form Vim takes.
    pub command: String,
    pub short: String,
    /// The act's name, in Title Case.
    pub title: String,
    /// Its key, where it has one besides the line.
    pub key: String,
    /// Whether it takes a range, as :y and :g do.
    pub ranged: bool,
    /// Whether a space after it lists files, as :e's does.
    pub files: bool,
    /// Whether it's :{n}'s line, which writes the cursor's line.
    pub row: bool,
}

/// The page's acts that the menu names, in the order it lists them, with
/// their names. One the language gives the page a command that isn't here
/// isn't listed, as :d, which only says E21.
pub const TITLES: &[(&str, &str)] = &[
    ("copy", "Copy the Lines"),
    ("edit", "Open a File"),
    ("next-file", "Next File"),
    ("previous-file", "Previous File"),
    ("global", "List Matching Lines"),
    ("marks", "List the Marks"),
    ("jumps", "List the Jumps"),
    ("nohlsearch", "Hide the Matches"),
    ("close", "Close the Page"),
    ("show-keys", "List the Keys"),
];

/// The page's menu, from the key language's commands for the page in
/// TITLES' order, then :{n}.
pub fn entries() -> Vec<Entry> {
    let commands = crate::keys::commands();
    TITLES
        .iter()
        .filter_map(|&(act, title)| {
            let command = commands.iter().find(|command| command.act == act)?;
            Some(Entry {
                command: command.command.clone(),
                short: command.short.clone(),
                title: title.to_string(),
                key: command.key.clone().unwrap_or_default(),
                ranged: command.ranged,
                files: act == "edit",
                row: false,
            })
        })
        .chain(std::iter::once(Entry {
            command: "{n}".into(),
            short: "{n}".into(),
            title: "Go to a Line".into(),
            key: String::new(),
            ranged: false,
            files: false,
            row: true,
        }))
        .collect()
}

/// A command's name in Vim's help notation: its shortest form outside the
/// brackets and the rest inside, as disc[onnect]; plain where the short
/// form is the whole name.
pub fn notation(command: &str, short: &str) -> String {
    match command.strip_prefix(short) {
        Some(rest) if !short.is_empty() && !rest.is_empty() => format!("{short}[{rest}]"),
        _ => command.to_string(),
    }
}

/// A line of the menu as it shows: what it's called, its name and key,
/// and what choosing it writes in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub called: String,
    pub title: String,
    pub key: String,
    pub writes: String,
}

/// What's typed split as the menu reads it: the range before the name, the
/// name, and whether a space followed it, with what comes after.
fn parts(typed: &str) -> (&str, &str, Option<&str>) {
    let said = typed.trim_start_matches([':', ' ']);
    let ranging = said.len()
        - said
            .trim_start_matches(|c: char| c.is_ascii_digit() || ",%$.'<> ".contains(c))
            .len();
    let (range, rest) = said.split_at(ranging);
    let letters = rest.len()
        - rest
            .trim_start_matches(|c: char| c.is_ascii_alphabetic())
            .len();
    let (name, after) = rest.split_at(letters);
    let argument = after
        .strip_prefix('!')
        .unwrap_or(after)
        .strip_prefix(' ')
        .map(str::trim_start);
    (range, name, argument)
}

/// Whether `word` names `entry` as Vim takes a name: its short form or any
/// longer start of its whole name.
fn names(word: &str, entry: &Entry) -> bool {
    !entry.row && word.len() >= entry.short.len() && entry.command.starts_with(word)
}

/// The menu's lines for what's typed. Nothing typed lists every line;
/// after a range, only those that take one, and :{n}'s for a number
/// alone; a name lists the commands it starts, then those with a word of
/// their name starting so, case ignored. After a name and a space, :e's
/// lines are the files `files` lists for the word being typed, each
/// written in its place, and a command taking nothing after it has none.
pub fn lines(
    entries: &[Entry],
    typed: &str,
    cursor_line: usize,
    files: &dyn Fn(&str) -> Vec<String>,
) -> Vec<Line> {
    let (range, name, argument) = parts(typed);
    let line = |entry: &Entry| Line {
        called: notation(&entry.command, &entry.short),
        title: entry.title.clone(),
        key: entry.key.clone(),
        writes: if entry.row {
            cursor_line.to_string()
        } else {
            format!("{range}{}", entry.command)
        },
    };
    if let Some(argument) = argument {
        if !entries
            .iter()
            .any(|entry| entry.files && names(name, entry))
        {
            return Vec::new();
        }
        let before = &typed[..typed.len() - argument.len()];
        return files(argument)
            .into_iter()
            .map(|file| Line {
                called: file.clone(),
                title: String::new(),
                key: String::new(),
                writes: format!("{before}{file}"),
            })
            .collect();
    }
    let unranged = range.trim().is_empty();
    let fits = |entry: &&Entry| unranged || entry.ranged || entry.row;
    if name.is_empty() {
        let numbered = !range.trim().is_empty() && range.trim().chars().all(|c| c.is_ascii_digit());
        return entries
            .iter()
            .filter(fits)
            .filter(|entry| !numbered || entry.row)
            .map(line)
            .collect();
    }
    let word = name.to_lowercase();
    let starting = entries
        .iter()
        .filter(fits)
        .filter(|entry| !entry.row && entry.command.starts_with(&word));
    let worded = entries.iter().filter(fits).filter(|entry| {
        !entry.row
            && !entry.command.starts_with(&word)
            && entry
                .title
                .to_lowercase()
                .split(' ')
                .any(|part| part.starts_with(&word))
    });
    starting.chain(worded).map(line).collect()
}

/// The menu while the line is typed: what was typed, its lines, the line
/// chosen, if any, and the first shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    pub typed: String,
    pub lines: Vec<Line>,
    pub chosen: Option<usize>,
    pub first: usize,
}

impl Menu {
    /// The menu for what's been typed, nothing chosen.
    pub fn new(typed: &str, lines: Vec<Line>) -> Self {
        Menu {
            typed: typed.to_string(),
            lines,
            chosen: None,
            first: 0,
        }
    }

    /// Tab, Shift+Tab, the arrows, Ctrl+N and Ctrl+P: the next line or the
    /// one before, coming round through what was typed, as his cmp and
    /// Vim's wildmenu do. What the line then holds.
    pub fn step(&mut self, by: isize) -> String {
        if self.lines.is_empty() {
            return self.typed.clone();
        }
        let places = isize::try_from(self.lines.len() + 1).unwrap_or(isize::MAX);
        let at = self
            .chosen
            .map_or(0, |at| isize::try_from(at + 1).unwrap_or(0));
        let next = usize::try_from((at + by).rem_euclid(places)).unwrap_or(0);
        self.chosen = next.checked_sub(1);
        if let Some(at) = self.chosen {
            if at < self.first {
                self.first = at;
            } else if at >= self.first + SHOWN {
                self.first = at + 1 - SHOWN;
            }
        }
        self.line()
    }

    /// What the line holds: the line chosen's words, or what was typed.
    pub fn line(&self) -> String {
        self.chosen
            .map_or_else(|| self.typed.clone(), |at| self.lines[at].writes.clone())
    }

    /// The lines shown, each with whether it's chosen.
    pub fn shown(&self) -> impl Iterator<Item = (&Line, bool)> {
        self.lines
            .iter()
            .enumerate()
            .skip(self.first)
            .take(SHOWN)
            .map(|(at, line)| (line, Some(at) == self.chosen))
    }
}

/// The files and folders `typed` starts naming, from the folder it names
/// or the one the reader started in, as Vim completes :e's: folders with
/// their slash, then files, by name, dotted ones only once a dot is typed.
pub fn files_for(typed: &str) -> Vec<String> {
    let (folder, start) = typed.rsplit_once('/').unwrap_or(("", typed));
    let read = if folder.is_empty() && typed.starts_with('/') {
        "/".to_string()
    } else if folder.is_empty() {
        ".".to_string()
    } else if let Some(under) = folder.strip_prefix('~') {
        format!("{}{under}", std::env::var("HOME").unwrap_or_default())
    } else {
        folder.to_string()
    };
    let Ok(entries) = std::fs::read_dir(&read) else {
        return Vec::new();
    };
    let lead = if typed.contains('/') {
        format!("{folder}/")
    } else {
        String::new()
    };
    let mut found: Vec<(bool, String)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let folder = entry.path().is_dir();
            (name.starts_with(start) && (start.starts_with('.') || !name.starts_with('.'))).then(
                || {
                    (
                        !folder,
                        format!("{lead}{name}{}", if folder { "/" } else { "" }),
                    )
                },
            )
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, name)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(command: &str, short: &str, title: &str, key: &str) -> Entry {
        Entry {
            command: command.into(),
            short: short.into(),
            title: title.into(),
            key: key.into(),
            ranged: command == "yank" || command == "global",
            files: command == "edit",
            row: command == "{n}",
        }
    }

    fn entries() -> Vec<Entry> {
        vec![
            entry("yank", "y", "Copy the Lines", "y y"),
            entry("edit", "e", "Open a File", ""),
            entry("bnext", "bn", "Next File", "]b"),
            entry("bprevious", "bp", "Previous File", "[b"),
            entry("marks", "marks", "List the Marks", ""),
            entry("quit", "q", "Close the Page", "ZZ"),
            entry("help", "h", "List the Keys", "F1"),
            entry("{n}", "{n}", "Go to a Line", ""),
        ]
    }

    fn called(typed: &str) -> Vec<String> {
        lines(&entries(), typed, 7, &|_| vec!["a.md".into(), "b/".into()])
            .into_iter()
            .map(|line| line.called)
            .collect()
    }

    #[test]
    fn the_page_lists_every_command_the_language_gives_it_but_delete() {
        let listed: Vec<String> = super::entries()
            .iter()
            .map(|entry| format!("{} {}", notation(&entry.command, &entry.short), entry.key))
            .collect();
        assert_eq!(
            listed,
            [
                "y[ank] yy",
                "e[dit] ",
                "bn[ext] ]b",
                "bp[revious] [b",
                "g[lobal] ",
                "marks ",
                "ju[mps] ",
                "noh[lsearch] ",
                "q[uit] ZZ",
                "h[elp] Space ?",
                "{n} ",
            ]
        );
        let named: Vec<&str> = TITLES.iter().map(|(act, _)| *act).collect();
        for command in crate::keys::commands() {
            assert!(
                named.contains(&command.act.as_str()) || command.act == "ending",
                "{} has no line in the menu",
                command.act
            );
        }
    }

    #[test]
    fn a_command_is_named_in_vims_help_notation() {
        assert_eq!(notation("disconnect", "disc"), "disc[onnect]");
        assert_eq!(notation("marks", "marks"), "marks");
        assert_eq!(notation("{n}", "{n}"), "{n}");
    }

    #[test]
    fn nothing_typed_lists_everything_and_a_name_narrows_it() {
        assert_eq!(called("").len(), 8);
        assert_eq!(called("b"), ["bn[ext]", "bp[revious]"]);
        assert_eq!(called("q"), ["q[uit]"]);
        assert_eq!(called("list"), ["marks", "h[elp]"], "by a word of the name");
        assert_eq!(called("file"), ["e[dit]", "bn[ext]", "bp[revious]"]);
        assert!(called("zzz").is_empty());
    }

    #[test]
    fn a_range_keeps_what_takes_one_and_a_number_its_line() {
        assert_eq!(called("%"), ["y[ank]", "{n}"]);
        assert_eq!(called("'<,'>y"), ["y[ank]"]);
        assert_eq!(called("12"), ["{n}"]);
        let written = lines(&entries(), "%y", 7, &|_| Vec::new());
        assert_eq!(written[0].writes, "%yank");
        let row = lines(&entries(), "1", 7, &|_| Vec::new());
        assert_eq!(row[0].writes, "7", "the cursor's line");
    }

    #[test]
    fn a_space_after_edit_lists_files_and_after_others_nothing() {
        assert_eq!(called("e "), ["a.md", "b/"]);
        let files = lines(&entries(), "edit b/", 7, &|typed| {
            vec![format!("{typed}c.md")]
        });
        assert_eq!(files[0].writes, "edit b/c.md");
        assert!(called("marks ").is_empty());
        assert!(called("x ").is_empty());
    }

    #[test]
    fn tab_comes_round_through_what_was_typed_and_scrolls() {
        let mut menu = Menu::new("b", lines(&entries(), "b", 7, &|_| Vec::new()));
        assert_eq!(menu.step(1), "bnext");
        assert_eq!(menu.step(1), "bprevious");
        assert_eq!(menu.step(1), "b", "round to what was typed");
        assert_eq!(menu.step(-1), "bprevious");
        let many: Vec<Line> = (0..15)
            .map(|n| Line {
                called: n.to_string(),
                title: String::new(),
                key: String::new(),
                writes: n.to_string(),
            })
            .collect();
        let mut menu = Menu::new("", many);
        for _ in 0..12 {
            menu.step(1);
        }
        assert_eq!((menu.chosen, menu.first), (Some(11), 2));
        assert_eq!(menu.shown().count(), SHOWN);
        assert!(menu.shown().last().is_some_and(|(_, chosen)| chosen));
    }
}
