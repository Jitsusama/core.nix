//! Which program a window is, by its app id: the desktop entry the shell
//! named it by through Quickshell's `DesktopEntries.heuristicLookup`
//! (src/core/desktopentry.cpp in Quickshell 0.3.2), and the program a
//! browser's window of its own belongs to when that names none, as the
//! shell's BrowserWindows.js found it.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use programs::notify::Notify;
use programs::{entry, xdg};

/// What a window's row needs of a desktop entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    /// Its `StartupWMClass`, the class its windows are given.
    pub class: Option<String>,
    /// Whether it's listed, as one saying `NoDisplay` isn't: a browser's
    /// window is given only to a listed program, as the shell gave it
    /// only among `DesktopEntries.applications`.
    pub listed: bool,
}

/// An entry from its text, or none where it isn't an application with a
/// name, or says it's `Hidden`, which Quickshell counts as no entry.
pub fn parse(id: &str, text: &str) -> Option<Entry> {
    let groups = entry::groups(text);
    let (_, keys) = groups.iter().find(|(name, _)| name == "Desktop Entry")?;
    let said = |key: &str| {
        keys.iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value.clone())
    };
    if said("Type").as_deref() != Some("Application") || said("Hidden").as_deref() == Some("true") {
        return None;
    }
    let name = said("Name").filter(|name| !name.is_empty())?;
    Some(Entry {
        id: id.to_owned(),
        name,
        class: said("StartupWMClass").filter(|class| !class.is_empty()),
        listed: said("NoDisplay").as_deref() != Some("true"),
    })
}

/// Every desktop entry, by its id.
#[derive(Clone, Debug, Default)]
pub struct Entries {
    by_id: BTreeMap<String, Entry>,
}

impl Entries {
    /// These entries, each its id and its text.
    pub fn from_texts<'a>(texts: impl IntoIterator<Item = (&'a str, &'a str)>) -> Entries {
        Entries {
            by_id: texts
                .into_iter()
                .filter_map(|(id, text)| parse(id, text))
                .map(|entry| (entry.id.clone(), entry))
                .collect(),
        }
    }

    /// The entries along these data folders, the nearest folder's
    /// winning, and every folder looked in, for inotify to watch.
    pub fn read(folders: &[PathBuf]) -> (Entries, Vec<PathBuf>) {
        let (paths, looked) = xdg::entries(folders);
        let texts: Vec<(String, String)> = paths
            .into_iter()
            .filter_map(|(id, path)| Some((id, std::fs::read_to_string(path).ok()?)))
            .collect();
        let entries =
            Entries::from_texts(texts.iter().map(|(id, text)| (id.as_str(), text.as_str())));
        (entries, looked)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// The entry a window's app id names, as `heuristicLookup` finds it:
    /// by its id, then its id in any case, then the class its windows are
    /// given, then that in any case. Where two match alike, Quickshell
    /// takes whichever its hash holds first; here it's the first by id,
    /// so a window is named the same each time.
    pub fn lookup(&self, name: &str) -> Option<&Entry> {
        if let Some(entry) = self.by_id.get(name) {
            return Some(entry);
        }
        let lower = name.to_lowercase();
        let entries = || self.by_id.values();
        entries()
            .find(|entry| entry.id.to_lowercase() == lower)
            .or_else(|| entries().find(|entry| entry.class.as_deref() == Some(name)))
            .or_else(|| {
                entries().find(|entry| {
                    entry
                        .class
                        .as_ref()
                        .is_some_and(|class| class.to_lowercase() == lower)
                })
            })
    }

    /// The program a window counts for: the entry its app id names, else
    /// the listed entry whose own page's window it was opened deeper in,
    /// as a browser's window of its own (`chrome_owner`).
    pub fn owner(&self, app_id: &str) -> Option<&Entry> {
        self.lookup(app_id)
            .or_else(|| chrome_owner(app_id, self.by_id.values().filter(|entry| entry.listed)))
    }
}

/// A Chrome window's host, path and profile, from its class. Chrome names
/// a window of its own after the page it opened at, its host, then its
/// path with each slash an underscore, then its profile, as
/// `chrome-docs.google.com__document_create-Default`. The host holds no
/// underscore and the profile no dash, while a path may hold dashes, as a
/// Meet call's code does, so the path is all that lies between.
pub fn chrome_parts(class: &str) -> Option<(&str, &str, &str)> {
    let rest = class.strip_prefix("chrome-")?;
    let (host, rest) = rest.split_once('_')?;
    let rest = rest.strip_prefix('_')?;
    let (path, profile) = rest.rsplit_once('-')?;
    (!host.is_empty() && !profile.is_empty()).then_some((host, path, profile))
}

/// The entry whose own page's window has the same host and profile as
/// this one's and the longest path this one's starts with, or none. A
/// program's entry says the class of its own page's window, such as
/// `chrome-docs.google.com__document_-Default`, so a window opened deeper
/// in (a new document, a mail opened from the shell, a call) is its too;
/// Docs' `document_` never takes a `spreadsheets_` window, and another
/// profile's window is another person's.
pub fn chrome_owner<'a>(
    app_id: &str,
    entries: impl IntoIterator<Item = &'a Entry>,
) -> Option<&'a Entry> {
    let (host, path, profile) = chrome_parts(app_id)?;
    let mut owner: Option<(&Entry, usize)> = None;
    for entry in entries {
        let Some((page_host, page_path, page_profile)) =
            entry.class.as_deref().and_then(chrome_parts)
        else {
            continue;
        };
        if page_host == host
            && page_profile == profile
            && path.starts_with(page_path)
            && owner.is_none_or(|(_, longest)| page_path.len() > longest)
        {
            owner = Some((entry, page_path.len()));
        }
    }
    owner.map(|(entry, _)| entry)
}

/// A window's title without its program's name trailing it, as Chrome
/// adds " - Google Chrome", since the row already says whose window it
/// is, as the shell's Provider.qml tidied it.
pub fn tidy<'a>(title: &'a str, program: &str) -> &'a str {
    if program.is_empty() {
        return title;
    }
    for joint in [" - ", " \u{2013} ", " \u{2014} ", " | "] {
        if let Some(own) = title
            .strip_suffix(program)
            .and_then(|rest| rest.strip_suffix(joint))
        {
            return own;
        }
    }
    title
}

/// What a watched folder is to whoever follows the entries.
#[derive(Default)]
struct Watched {
    /// Whether it's a folder of entries, whose every change counts.
    entries: bool,
    /// The links in it that a data folder resolves through, by name, as
    /// `current-system` in `/run`: replaced, the entries changed.
    links: Vec<String>,
}

/// Calls `changed` on a thread of its own each time the entries in the
/// data folders `folders` change, for as long as the program runs: each
/// folder of entries `looked`, as [`Entries::read`] answers it, and each
/// link a data folder is reached through. Every daemon naming programs
/// by their entries follows them this way, so each hears a switch alike.
/// Answers why it can't, where inotify isn't there.
pub fn follow(
    folders: &[PathBuf],
    looked: Vec<PathBuf>,
    mut changed: impl FnMut() + Send + 'static,
) -> std::io::Result<()> {
    let notify = Notify::new()?;
    let mut watched: HashMap<i32, Watched> = HashMap::new();
    for folder in looked {
        if let Some(watch) = notify.watch(&folder) {
            watched.entry(watch).or_default().entries = true;
        }
    }
    // On NixOS each data folder is a store path reached through a
    // profile's links, so a switch says nothing in the folder itself: the
    // link it replaces is heard instead.
    for (folder, name) in folders.iter().flat_map(|folder| xdg::links_toward(folder)) {
        if let Some(watch) = notify.watch(&folder) {
            let links = &mut watched.entry(watch).or_default().links;
            if !links.contains(&name) {
                links.push(name);
            }
        }
    }
    std::thread::spawn(move || {
        while let Ok(changes) = notify.changes() {
            let entries = changes.iter().any(|(watch, name)| {
                watched
                    .get(watch)
                    .is_some_and(|one| one.entries || one.links.contains(name))
            });
            if entries {
                changed();
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Entries as Joel's are written: kitty and foot by their ids, Slack's
    /// class unlike its id, Chrome's web apps by their own pages' windows,
    /// one per profile, and one he hid.
    fn joels() -> Entries {
        Entries::from_texts([
            ("kitty", "[Desktop Entry]\nType=Application\nName=kitty\n"),
            ("foot", "[Desktop Entry]\nType=Application\nName=Foot\n"),
            (
                "com.slack.Slack",
                "[Desktop Entry]\nType=Application\nName=Slack\nStartupWMClass=Slack\n",
            ),
            (
                "google-chrome",
                "[Desktop Entry]\nType=Application\nName=Google Chrome\n",
            ),
            (
                "chrome-docs",
                concat!(
                    "[Desktop Entry]\nType=Application\nName=Docs\n",
                    "StartupWMClass=chrome-docs.google.com__document_-Default\n"
                ),
            ),
            (
                "chrome-sheets",
                concat!(
                    "[Desktop Entry]\nType=Application\nName=Sheets\n",
                    "StartupWMClass=chrome-docs.google.com__spreadsheets_-Default\n"
                ),
            ),
            (
                "chrome-meet",
                concat!(
                    "[Desktop Entry]\nType=Application\nName=Meet\n",
                    "StartupWMClass=chrome-meet.google.com__-Default\n"
                ),
            ),
            (
                "chrome-meet-work",
                concat!(
                    "[Desktop Entry]\nType=Application\nName=Meet at Work\nNoDisplay=true\n",
                    "StartupWMClass=chrome-meet.google.com__-Profile_1\n"
                ),
            ),
            (
                "gone",
                "[Desktop Entry]\nType=Application\nName=Gone\nHidden=true\n",
            ),
            ("link", "[Desktop Entry]\nType=Link\nName=A Link\n"),
        ])
    }

    fn named(entries: &Entries, app_id: &str) -> Option<String> {
        entries.lookup(app_id).map(|entry| entry.name.clone())
    }

    #[test]
    fn an_app_id_names_its_entry_by_id_then_case_then_class() {
        let entries = joels();
        assert_eq!(named(&entries, "kitty").as_deref(), Some("kitty"));
        assert_eq!(named(&entries, "Foot").as_deref(), Some("Foot"));
        assert_eq!(named(&entries, "Slack").as_deref(), Some("Slack"));
        assert_eq!(named(&entries, "slack").as_deref(), Some("Slack"));
        assert_eq!(named(&entries, "firefox"), None);
    }

    #[test]
    fn a_hidden_entry_or_one_not_an_application_names_nothing() {
        let entries = joels();
        assert_eq!(named(&entries, "gone"), None);
        assert_eq!(named(&entries, "link"), None);
        assert_eq!(entries.len(), 8);
    }

    #[test]
    fn a_browser_window_opened_deeper_is_its_pages_program() {
        let entries = joels();
        let owner = |app: &str| entries.owner(app).map(|entry| entry.id.clone());
        // A program's own page is its own.
        assert_eq!(
            owner("chrome-docs.google.com__document_-Default").as_deref(),
            Some("chrome-docs")
        );
        assert_eq!(
            owner("chrome-docs.google.com__document_create-Default").as_deref(),
            Some("chrome-docs")
        );
        assert_eq!(
            owner("chrome-docs.google.com__document_d_1aBc_edit-Default").as_deref(),
            Some("chrome-docs")
        );
        // A spreadsheet never goes to Docs, though they share a host.
        assert_eq!(
            owner("chrome-docs.google.com__spreadsheets_d_1-Default").as_deref(),
            Some("chrome-sheets")
        );
        // A page no program keeps is nobody's.
        assert_eq!(
            owner("chrome-calendar.google.com__calendar_r_eventedit-Default"),
            None
        );
        // A Meet call's code holds dashes, and is still Meet's.
        assert_eq!(
            owner("chrome-meet.google.com__abc-defg-hij-Default").as_deref(),
            Some("chrome-meet")
        );
        // Another profile's window is another person's, and an unlisted
        // entry takes none.
        assert_eq!(owner("chrome-meet.google.com__abc-Profile_1"), None);
        assert_eq!(owner("chrome-docs.google.com__document_-Profile_2"), None);
        // A window with no app id is nobody's, and with no entries none is.
        assert_eq!(owner(""), None);
        assert_eq!(
            Entries::from_texts(std::iter::empty())
                .owner("chrome-docs.google.com__document_create-Default")
                .map(|entry| entry.id.clone()),
            None
        );
        // The lookup alone, as Windows' rows name a program, finds none.
        assert_eq!(
            named(&entries, "chrome-docs.google.com__document_create-Default"),
            None
        );
    }

    #[test]
    fn a_chrome_class_parts_at_its_double_underscore_and_last_dash() {
        assert_eq!(
            chrome_parts("chrome-meet.google.com__abc-defg-hij-Default"),
            Some(("meet.google.com", "abc-defg-hij", "Default"))
        );
        assert_eq!(chrome_parts("chrome-x_y-Default"), None);
        assert_eq!(chrome_parts("kitty"), None);
        assert_eq!(chrome_parts("chrome-a__b-"), None);
    }

    #[test]
    fn a_title_loses_its_programs_name_after_a_joint() {
        assert_eq!(tidy("Inbox - Google Chrome", "Google Chrome"), "Inbox");
        assert_eq!(tidy("Notes \u{2014} Obsidian", "Obsidian"), "Notes");
        assert_eq!(tidy("general | Slack", "Slack"), "general");
        assert_eq!(tidy("Slack", "Slack"), "Slack");
        assert_eq!(tidy("about kitty", "kitty"), "about kitty");
        assert_eq!(tidy("anything", ""), "anything");
    }
}
