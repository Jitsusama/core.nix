//! What Chrome has open now, read from its own session file: every tab of
//! its browser windows, with its window, whether it's the one in front
//! there, when it was last looked at, its address and its title.
//!
//! Chrome keeps the session in `Sessions/Session_*` under the profile, the
//! newest the one it's writing. The file is "SNSS", a version, then
//! commands, each its size (two bytes, counting its id), its id (one byte)
//! and what it carries. Replaying the commands in order, as Chrome does to
//! restore a session (`components/sessions/core/session_service_commands.cc`),
//! gives the tabs open now. Chrome only appends to it, within seconds of a
//! change, so reading it whole while Chrome runs is safe, and a command cut
//! off at the end is one still being written, left for the next reading.
//! Version 3 is the only one read: 5 is encrypted, and 1 is long gone.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::{LEFT_OUT, host};

/// The session's commands this reads; Chrome's names, ids as it numbers them.
const SET_TAB_WINDOW: u8 = 0;
const SET_TAB_INDEX_IN_WINDOW: u8 = 2;
const UPDATE_TAB_NAVIGATION: u8 = 6;
const SET_SELECTED_NAVIGATION_INDEX: u8 = 7;
const SET_SELECTED_TAB_IN_INDEX: u8 = 8;
const SET_WINDOW_TYPE: u8 = 9;
const TAB_CLOSED: u8 = 16;
const WINDOW_CLOSED: u8 = 17;
const LAST_ACTIVE_TIME: u8 = 21;
const TAB_NAVIGATION_PATH_PRUNED: u8 = 24;

/// The one version read, which marks where its starting state ends.
const VERSION: i32 = 3;

/// A browser window, as against an app's, a popup or developer tools.
const NORMAL: i32 = 0;

/// Microseconds from 1601, when Chrome's clock starts, to 1970.
const TO_UNIX: i64 = 11_644_473_600_000_000;

/// A tab open now, as the launcher is told of it.
#[derive(Debug, PartialEq, Eq)]
pub struct Open {
    pub id: i32,
    pub window: i32,
    pub front: bool,
    /// When it was last looked at, in milliseconds since 1970, or 0 when
    /// Chrome never said.
    pub last: i64,
    /// Its page's address, or nothing when it isn't the web or has a
    /// program of its own (`LEFT_OUT`); it's still told of, since its title
    /// is how a window it's in front of is found.
    pub address: String,
    pub title: String,
}

#[derive(Default)]
struct Tab {
    window: Option<i32>,
    index: i32,
    /// Its pages back and forward, each its index, address and title.
    pages: Vec<(i32, String, String)>,
    current: i32,
    active: i64,
}

#[derive(Default)]
struct Window {
    selected: i32,
    kind: i32,
}

/// What a command carries, read as Chrome writes it: four-byte numbers,
/// and in a pickle, strings each its length then itself padded to four.
struct Carried<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Carried<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let taken = self.bytes.get(self.at..self.at.checked_add(count)?)?;
        self.at += count;
        Some(taken)
    }

    fn int(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn long(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn padded(&mut self, count: usize) -> Option<&'a [u8]> {
        let taken = self.take(count)?;
        self.take(count.next_multiple_of(4) - count)?;
        Some(taken)
    }

    fn string(&mut self) -> Option<String> {
        let length = usize::try_from(self.int()?).ok()?;
        Some(String::from_utf8_lossy(self.padded(length)?).into_owned())
    }

    fn string16(&mut self) -> Option<String> {
        let length = usize::try_from(self.int()?).ok()?;
        let units: Vec<u16> = self
            .padded(length.checked_mul(2)?)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes(*unit))
            .collect();
        Some(String::from_utf16_lossy(&units))
    }
}

#[derive(Default)]
struct Session {
    tabs: HashMap<i32, Tab>,
    windows: HashMap<i32, Window>,
    closed: HashSet<i32>,
}

impl Session {
    /// Plays one command; one it can't read changes nothing.
    fn play(&mut self, id: u8, carried: &[u8]) -> Option<()> {
        let mut read = Carried {
            bytes: carried,
            at: 0,
        };
        match id {
            SET_TAB_WINDOW => {
                let (window, tab) = (read.int()?, read.int()?);
                self.tabs.entry(tab).or_default().window = Some(window);
            }
            SET_TAB_INDEX_IN_WINDOW => {
                let (tab, index) = (read.int()?, read.int()?);
                self.tabs.entry(tab).or_default().index = index;
            }
            UPDATE_TAB_NAVIGATION => {
                // A pickle: its size, then the tab and the page.
                read.int()?;
                let (tab, index) = (read.int()?, read.int()?);
                let (address, title) = (read.string()?, read.string16()?);
                let pages = &mut self.tabs.entry(tab).or_default().pages;
                match pages.binary_search_by_key(&index, |page| page.0) {
                    Ok(at) => pages[at] = (index, address, title),
                    Err(at) => pages.insert(at, (index, address, title)),
                }
            }
            SET_SELECTED_NAVIGATION_INDEX => {
                let (tab, index) = (read.int()?, read.int()?);
                self.tabs.entry(tab).or_default().current = index;
            }
            SET_SELECTED_TAB_IN_INDEX => {
                let (window, index) = (read.int()?, read.int()?);
                self.windows.entry(window).or_default().selected = index;
            }
            SET_WINDOW_TYPE => {
                let (window, kind) = (read.int()?, read.int()?);
                self.windows.entry(window).or_default().kind = kind;
            }
            TAB_CLOSED => {
                self.tabs.remove(&read.int()?);
            }
            WINDOW_CLOSED => {
                let window = read.int()?;
                self.windows.remove(&window);
                self.closed.insert(window);
            }
            LAST_ACTIVE_TIME => {
                // Its tab, then the time, eight bytes aligned to eight.
                let tab = read.int()?;
                read.int()?;
                self.tabs.entry(tab).or_default().active = read.long()?;
            }
            TAB_NAVIGATION_PATH_PRUNED => {
                let (tab, index, count) = (read.int()?, read.int()?, read.int()?);
                let tab = self.tabs.entry(tab).or_default();
                let end = index.checked_add(count)?;
                if (index..end).contains(&tab.current) {
                    tab.current = index - 1;
                } else if tab.current >= end {
                    tab.current -= count;
                }
                tab.pages.retain(|page| !(index..end).contains(&page.0));
                for page in &mut tab.pages {
                    if page.0 >= end {
                        page.0 -= count;
                    }
                }
            }
            _ => {}
        }
        Some(())
    }

    /// The tabs of its browser windows still open, newest looked at first.
    fn open(self) -> Vec<Open> {
        let mut open: Vec<Open> = self
            .tabs
            .into_iter()
            .filter_map(|(id, tab)| {
                let window = tab.window?;
                let shown = self.windows.get(&window);
                if self.closed.contains(&window) || shown.is_some_and(|one| one.kind != NORMAL) {
                    return None;
                }
                let (_, address, title) = tab.pages.iter().find(|page| page.0 == tab.current)?;
                let web = host(address)
                    .map(str::to_ascii_lowercase)
                    .is_some_and(|host| !LEFT_OUT.contains(&host.as_str()));
                Some(Open {
                    id,
                    window,
                    front: shown.is_some_and(|one| one.selected == tab.index),
                    // A tab never looked at says so with nothing.
                    last: if tab.active == 0 {
                        0
                    } else {
                        (tab.active - TO_UNIX) / 1000
                    },
                    address: if web { address.clone() } else { String::new() },
                    // A line a tab, so nothing in a title may break one.
                    title: title.split_whitespace().collect::<Vec<_>>().join(" "),
                })
            })
            .collect();
        open.sort_by(|a, b| b.last.cmp(&a.last).then(a.id.cmp(&b.id)));
        open
    }
}

/// The tabs a session file holds open.
pub fn replay(file: &[u8]) -> Result<Vec<Open>, String> {
    if file.get(..4) != Some(b"SNSS") {
        return Err("not a session file".into());
    }
    let version = Carried { bytes: file, at: 4 }
        .int()
        .ok_or("its version is cut off")?;
    if version != VERSION {
        return Err(format!(
            "its version is {version}, and only {VERSION} is read"
        ));
    }
    let mut session = Session::default();
    let mut at = 8;
    while let Some(size) = file.get(at..at + 2) {
        let size = usize::from(u16::from_le_bytes([size[0], size[1]]));
        let Some(command) = file.get(at + 2..at + 2 + size).filter(|_| size > 0) else {
            break;
        };
        session.play(command[0], &command[1..]);
        at += 2 + size;
    }
    Ok(session.open())
}

/// What a profile has open now, from its newest session file; none when
/// Chrome has never kept one.
pub fn read(profile: &Path) -> Result<Vec<Open>, String> {
    let sessions = profile.join("Sessions");
    let Ok(entries) = fs::read_dir(&sessions) else {
        return Ok(Vec::new());
    };
    let newest = entries
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            let made: u64 = name.strip_prefix("Session_")?.parse().ok()?;
            Some((made, name))
        })
        .max();
    let Some((_, name)) = newest else {
        return Ok(Vec::new());
    };
    let file = sessions.join(name);
    let bytes = fs::read(&file).map_err(|error| format!("{}: {error}", file.display()))?;
    replay(&bytes).map_err(|error| format!("{}: {error}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A session file holding these commands, as Chrome writes them.
    fn session(commands: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut file = b"SNSS".to_vec();
        file.extend(VERSION.to_le_bytes());
        for (id, carried) in commands {
            let size = u16::try_from(carried.len() + 1).unwrap();
            file.extend(size.to_le_bytes());
            file.push(*id);
            file.extend(carried);
        }
        file
    }

    fn ints(values: &[i32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn padded(mut bytes: Vec<u8>) -> Vec<u8> {
        bytes.resize(bytes.len().next_multiple_of(4), 0);
        bytes
    }

    fn page(tab: i32, index: i32, address: &str, title: &str) -> (u8, Vec<u8>) {
        let mut pickle = ints(&[tab, index, i32::try_from(address.len()).unwrap()]);
        pickle.extend(padded(address.as_bytes().to_vec()));
        let units: Vec<u16> = title.encode_utf16().collect();
        pickle.extend(ints(&[i32::try_from(units.len()).unwrap()]));
        pickle.extend(padded(
            units.iter().flat_map(|unit| unit.to_le_bytes()).collect(),
        ));
        // What Chrome writes after the title, which nothing here reads.
        pickle.extend(ints(&[0, 0]));
        let mut carried = ints(&[i32::try_from(pickle.len()).unwrap()]);
        carried.extend(pickle);
        (UPDATE_TAB_NAVIGATION, carried)
    }

    /// A tab in a window at a place, on one page, looked at a while after
    /// 1970.
    fn tab(window: i32, tab: i32, index: i32, address: &str, looked: i64) -> Vec<(u8, Vec<u8>)> {
        let mut active = ints(&[tab, 0]);
        active.extend((TO_UNIX + looked * 1000).to_le_bytes());
        vec![
            (SET_TAB_WINDOW, ints(&[window, tab])),
            (SET_TAB_INDEX_IN_WINDOW, ints(&[tab, index])),
            page(tab, 0, address, &format!("Title {tab}")),
            (SET_SELECTED_NAVIGATION_INDEX, ints(&[tab, 0])),
            (LAST_ACTIVE_TIME, active),
        ]
    }

    fn lines(open: &[Open]) -> Vec<String> {
        open.iter()
            .map(|tab| {
                format!(
                    "{} {} {} {} {} {}",
                    tab.id, tab.window, tab.front, tab.last, tab.address, tab.title
                )
            })
            .collect()
    }

    #[test]
    fn every_tab_open_with_its_window_and_whether_its_in_front_newest_first() {
        let mut commands = Vec::new();
        commands.extend(tab(1, 10, 0, "https://a.example.org/", 300));
        commands.extend(tab(1, 11, 1, "https://b.example.org/x", 100));
        commands.extend(tab(2, 20, 0, "https://c.example.org/", 200));
        commands.push((SET_SELECTED_TAB_IN_INDEX, ints(&[1, 1])));
        commands.push((SET_SELECTED_TAB_IN_INDEX, ints(&[2, 0])));
        assert_eq!(
            lines(&replay(&session(&commands)).unwrap()),
            [
                "10 1 false 300 https://a.example.org/ Title 10",
                "20 2 true 200 https://c.example.org/ Title 20",
                "11 1 true 100 https://b.example.org/x Title 11",
            ]
        );
    }

    #[test]
    fn a_tab_or_window_closed_is_gone_as_is_an_apps_window() {
        let mut commands = Vec::new();
        commands.extend(tab(1, 10, 0, "https://kept.example.org/", 1));
        commands.extend(tab(1, 11, 1, "https://shut.example.org/", 2));
        commands.extend(tab(2, 20, 0, "https://gone.example.org/", 3));
        commands.extend(tab(3, 30, 0, "https://app.example.org/", 4));
        commands.push((TAB_CLOSED, ints(&[11, 0, 0])));
        commands.push((WINDOW_CLOSED, ints(&[2, 0, 0])));
        commands.push((SET_WINDOW_TYPE, ints(&[3, 2])));
        let open = replay(&session(&commands)).unwrap();
        assert_eq!(
            open.iter().map(|tab| tab.id).collect::<Vec<_>>(),
            [10],
            "{open:?}"
        );
    }

    #[test]
    fn a_tab_shows_the_page_it_went_back_to_after_its_path_was_pruned() {
        let mut commands = tab(1, 10, 0, "https://first.example.org/", 1);
        commands.push(page(10, 1, "https://second.example.org/", "Second"));
        commands.push(page(10, 2, "https://third.example.org/", "Third"));
        commands.push((SET_SELECTED_NAVIGATION_INDEX, ints(&[10, 2])));
        // Its first two pages pruned, so the third is now its first.
        commands.push((TAB_NAVIGATION_PATH_PRUNED, ints(&[10, 0, 2])));
        commands.push(page(10, 1, "https://fourth.example.org/", "Fourth"));
        commands.push((SET_SELECTED_NAVIGATION_INDEX, ints(&[10, 0])));
        let open = replay(&session(&commands)).unwrap();
        assert_eq!(open[0].address, "https://third.example.org/");
        assert_eq!(open[0].title, "Third");
    }

    #[test]
    fn a_page_with_a_program_of_its_own_or_off_the_web_keeps_its_title_alone() {
        let mut commands = tab(1, 10, 0, "https://mail.google.com/mail/u/0/", 2);
        commands.extend(tab(1, 11, 1, "chrome://newtab/", 1));
        let open = replay(&session(&commands)).unwrap();
        assert_eq!(
            lines(&open),
            ["10 1 false 2  Title 10", "11 1 false 1  Title 11"]
        );
    }

    #[test]
    fn a_title_is_one_line() {
        let mut commands = tab(1, 10, 0, "https://a.example.org/", 1);
        commands.push(page(10, 0, "https://a.example.org/", "Two\tparts\nhere"));
        assert_eq!(
            replay(&session(&commands)).unwrap()[0].title,
            "Two parts here"
        );
    }

    #[test]
    fn a_command_cut_off_at_the_end_waits_for_the_next_reading() {
        let whole = session(&tab(1, 10, 0, "https://a.example.org/", 1));
        // The next tab's first command, half of it written.
        let mut cut = whole.clone();
        cut.extend(&session(&tab(1, 11, 1, "https://b.example.org/", 2))[8..14]);
        assert_eq!(
            lines(&replay(&cut).unwrap()),
            lines(&replay(&whole).unwrap())
        );
    }

    #[test]
    fn a_file_of_another_version_or_kind_is_refused_saying_which() {
        let mut encrypted = session(&[]);
        encrypted[4] = 5;
        assert_eq!(
            replay(&encrypted),
            Err("its version is 5, and only 3 is read".into())
        );
        assert_eq!(replay(b"SQLite format 3"), Err("not a session file".into()));
    }

    #[test]
    fn the_newest_session_file_is_read_and_a_profile_without_one_has_none() {
        let profile = std::env::temp_dir().join(format!(
            "chrome-recent-sites-test-{}-sessions",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&profile);
        assert_eq!(read(&profile).unwrap(), []);
        let sessions = profile.join("Sessions");
        fs::create_dir_all(&sessions).unwrap();
        assert_eq!(read(&profile).unwrap(), []);
        let older = session(&tab(1, 10, 0, "https://older.example.org/", 1));
        let newer = session(&tab(1, 10, 0, "https://newer.example.org/", 1));
        fs::write(sessions.join("Session_13437839700511944"), older).unwrap();
        fs::write(sessions.join("Session_13437850409745633"), newer).unwrap();
        fs::write(sessions.join("Tabs_13437850409745999"), b"not read").unwrap();
        assert_eq!(
            read(&profile).unwrap()[0].address,
            "https://newer.example.org/"
        );
    }
}
