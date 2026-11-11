//! The reader's keys: Neovim's own normal-mode and visual-mode keys, read
//! from the one key language every surface of ours speaks
//! (modules/keys/language.toml), by the names of the acts a page holds
//! there, with the modes each works in, and the reader's own keys for what
//! Vim has nothing for, as zoom. A machine in front of the table keeps what
//! vi keeps between presses: a count, and a key waiting for its second.

mod list;
mod machine;

use std::sync::OnceLock;

use crate::select::Object;

use list::shown_all;
pub use list::{chords, listing};
pub use machine::Machine;

/// A key as the reader is given it, by what it types rather than where it
/// sits, so Colemak's letters are vi's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Escape,
    Return,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    F1,
    /// Tab, and Shift+Tab, which the lines' menu steps through.
    Tab,
    BackTab,
}

/// What a key asks of the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// The cursor down or up by the lines shown, the view following it, or
    /// across by columns, as vi's j, k, l and h; while selecting, the
    /// selection's cursor.
    Down,
    Up,
    Left,
    Right,
    /// The view alone down or up a line, as Ctrl+E and Ctrl+Y: the cursor
    /// stays on its line while that shows.
    ScrollDown,
    ScrollUp,
    HalfDown,
    HalfUp,
    ScreenDown,
    ScreenUp,
    /// The first line, or with a count that line, as gg.
    First,
    /// The last line, or with a count that line, as G.
    Last,
    /// } and {: the view's top to the next paragraph, or the one before,
    /// blocks with no blank line between them in the file being one, as
    /// vi's paragraphs; while selecting, the selection's cursor.
    ParagraphOn,
    ParagraphBack,
    /// ]] and [[: the view's top to the next heading, or the one before.
    HeadingOn,
    HeadingBack,
    /// H, M and L: the cursor to the top, middle or bottom line in view;
    /// what acts on the line here (zt, *, gf) acts on it.
    ViewTop,
    ViewMiddle,
    ViewBottom,
    /// zt, zz and zb: the view placed with the cursor's line at its top,
    /// middle or bottom.
    PlaceTop,
    PlaceMiddle,
    PlaceBottom,
    /// %: to a bracket's other end, from the first on the cursor's line;
    /// with a count, that percent of the way down, as vi's N%.
    Bracket,
    /// Star and hash: a search down or up for the first word on the
    /// cursor's line, whole, as vi's `\<word\>`.
    SameWordOn,
    SameWordBack,
    /// ]b and [b: the next markdown file in the folder, or the one before,
    /// as vi's next buffer.
    FileOn,
    FileBack,
    /// gf: the file the first link from the cursor's line names, or a path
    /// written there; Ctrl+]: the first link from the cursor's line.
    FollowFile,
    FollowLink,
    /// gO: the headings, as a list to go to one from, as Neovim's gO.
    Outline,
    /// `:`: the command line, for :{n}, :q and :noh.
    Command,
    /// Starts a search forward or backward: what follows is typed.
    SearchForward,
    SearchBackward,
    /// The next match in the search's own direction, or the other way.
    Next,
    Previous,
    SetMark(char),
    GoToMark(char),
    /// Back to where the last jump left from, as ''.
    LastJump,
    /// Back and forward through the jumps, as Ctrl+O and Ctrl+I.
    JumpBack,
    JumpForward,
    Close,
    /// Labels every link in view, as flash labels a jump in his Neovim; the
    /// label typed follows it. Neovim's gx opens the link under the cursor,
    /// and the cursor here is a line, which may hold several, so the label
    /// says which.
    Links,
    /// The page larger, smaller, or at its own size again, as a browser's
    /// Ctrl+=, Ctrl+- and Ctrl+0.
    ZoomIn,
    ZoomOut,
    ZoomBack,
    /// Lists every key here, as Space ? in Joel's Neovim.
    ShowKeys,
    /// @: the last line the command line ran, run again.
    AgainCommand,
    /// v and V: a selection, of characters or of whole paragraphs, items
    /// and lines of code, from the first word on the cursor's line, with
    /// the block cursor there; the same key again ends it, and the other
    /// turns it into one of the other kind.
    Select,
    SelectPieces,
    /// While selecting, the cursor on by words as w, b and e, and to a
    /// line's ends as 0 and $.
    WordOn,
    WordBack,
    WordEnd,
    LineStart,
    LineEnd,
    /// o: the cursor to the selection's other end.
    OtherEnd,
    /// A text object round the cursor: iw, aw, is, as, ip, ap and `` i` ``.
    Object(Object),
    /// y, and Ctrl+C as everywhere else: copies the selection as the
    /// file's markdown, and ends it.
    Copy,
    /// y y, y i p and y a p: copies the paragraph, item or line of code the
    /// cursor's line is in, or its paragraph, or that with the blank line
    /// after it.
    CopyPiece,
    CopyParagraph,
    CopyAParagraph,
    /// y }: copies from the cursor's piece to its paragraph's end.
    CopyToParagraphEnd,
    /// A change, which a page refuses as a buffer Vim can't modify does,
    /// saying E21.
    Change(Edit),
    /// Escape at rest: nothing was in progress. In a window it does
    /// nothing, as Vim's does, so a habitual second Escape never closes a
    /// page or puts the search's marks away; while selecting, it ends the
    /// selection.
    Rest,
}

impl Act {
    /// Whether a held key does it again, as a held j goes on down the
    /// page: only what moves the view, the cursor or a selection's end,
    /// so a held y copies once and a held v starts one selection.
    pub fn repeats(self) -> bool {
        matches!(
            self,
            Act::Down
                | Act::Up
                | Act::Left
                | Act::Right
                | Act::ScrollDown
                | Act::ScrollUp
                | Act::HalfDown
                | Act::HalfUp
                | Act::ScreenDown
                | Act::ScreenUp
                | Act::ParagraphOn
                | Act::ParagraphBack
                | Act::HeadingOn
                | Act::HeadingBack
                | Act::SameWordOn
                | Act::SameWordBack
                | Act::Next
                | Act::Previous
                | Act::JumpBack
                | Act::JumpForward
                | Act::WordOn
                | Act::WordBack
                | Act::WordEnd
                | Act::ZoomIn
                | Act::ZoomOut
        )
    }
}

/// Vim's changes, which a page has only to refuse: dd, cc and S, d and c
/// with their motion or on a selection, x and p.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    Ending,
    ChangeLine,
    DeleteMotion,
    ChangeMotion,
    DeleteCharacter,
    PutAfter,
}

/// Where a key works: at rest, while selecting, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum In {
    Both,
    Rest,
    Selecting,
}

impl In {
    /// Where an act works, from the modes the language gives it: normal
    /// mode is a page at rest, and visual mode while selecting.
    fn of(modes: &[&str]) -> Option<In> {
        match (modes.contains(&"normal"), modes.contains(&"visual")) {
            (true, true) => Some(In::Both),
            (true, false) => Some(In::Rest),
            (false, true) => Some(In::Selecting),
            (false, false) => None,
        }
    }

    fn holds(self, selecting: bool) -> bool {
        match self {
            In::Both => true,
            In::Rest => !selecting,
            In::Selecting => selecting,
        }
    }
}

use Key::{Char, Ctrl};

/// The key language, as the reader is built with it.
const LANGUAGE: &str = include_str!("../../../keys/language.toml");

/// The acts the reader takes from the language, by their names there, in
/// the order its key list shows them.
const SPOKEN: &[(&str, Act)] = &[
    ("down", Act::Down),
    ("up", Act::Up),
    ("left", Act::Left),
    ("right", Act::Right),
    ("scroll-down", Act::ScrollDown),
    ("scroll-up", Act::ScrollUp),
    ("half-down", Act::HalfDown),
    ("half-up", Act::HalfUp),
    ("screen-down", Act::ScreenDown),
    ("screen-up", Act::ScreenUp),
    ("first", Act::First),
    ("last", Act::Last),
    ("paragraph-on", Act::ParagraphOn),
    ("paragraph-back", Act::ParagraphBack),
    ("heading-on", Act::HeadingOn),
    ("heading-back", Act::HeadingBack),
    ("view-top", Act::ViewTop),
    ("view-middle", Act::ViewMiddle),
    ("view-bottom", Act::ViewBottom),
    ("place-top", Act::PlaceTop),
    ("place-middle", Act::PlaceMiddle),
    ("place-bottom", Act::PlaceBottom),
    ("bracket", Act::Bracket),
    ("search-word-forward", Act::SameWordOn),
    ("search-word-backward", Act::SameWordBack),
    ("next-file", Act::FileOn),
    ("previous-file", Act::FileBack),
    ("follow-file", Act::FollowFile),
    ("follow-link", Act::FollowLink),
    ("outline", Act::Outline),
    ("command-line", Act::Command),
    ("search-forward", Act::SearchForward),
    ("search-backward", Act::SearchBackward),
    ("next", Act::Next),
    ("previous", Act::Previous),
    ("last-jump", Act::LastJump),
    ("jump-back", Act::JumpBack),
    ("jump-forward", Act::JumpForward),
    ("links", Act::Links),
    ("select", Act::Select),
    ("select-lines", Act::SelectPieces),
    ("word-on", Act::WordOn),
    ("word-back", Act::WordBack),
    ("word-end", Act::WordEnd),
    ("line-start", Act::LineStart),
    ("line-end", Act::LineEnd),
    ("other-end", Act::OtherEnd),
    ("in-word", Act::Object(Object::InWord)),
    ("a-word", Act::Object(Object::AWord)),
    ("in-sentence", Act::Object(Object::InSentence)),
    ("a-sentence", Act::Object(Object::ASentence)),
    ("in-paragraph", Act::Object(Object::InParagraph)),
    ("a-paragraph", Act::Object(Object::AParagraph)),
    ("in-code", Act::Object(Object::InCode)),
    ("a-code", Act::Object(Object::ACode)),
    ("copy-selection", Act::Copy),
    ("copy", Act::CopyPiece),
    ("copy-paragraph", Act::CopyParagraph),
    ("copy-a-paragraph", Act::CopyAParagraph),
    ("copy-to-paragraph-end", Act::CopyToParagraphEnd),
    ("ending", Act::Change(Edit::Ending)),
    ("change-line", Act::Change(Edit::ChangeLine)),
    ("delete-character", Act::Change(Edit::DeleteCharacter)),
    ("put-after", Act::Change(Edit::PutAfter)),
    ("close", Act::Close),
    ("show-keys", Act::ShowKeys),
    ("again-command", Act::AgainCommand),
];

/// An act that takes the letter typed after its key, as a mark's name.
type Slot = fn(char) -> Act;

/// The acts whose key takes a letter after it, by their names.
const SLOTS: &[(&str, Slot)] = &[("set-mark", Act::SetMark), ("go-to-mark", Act::GoToMark)];

/// The acts whose key takes a motion after it, as Vim's operators do, or
/// acts on a selection at once, by their names.
const OPERATORS: &[(&str, Act)] = &[
    ("delete-motion", Act::Change(Edit::DeleteMotion)),
    ("change-motion", Act::Change(Edit::ChangeMotion)),
];

/// What a key does on a line being typed: the find line after / and ?,
/// and the command line after :, as Vim's command line takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKey {
    /// Escape and Ctrl+C: the line dropped.
    Back,
    Leave,
    /// Backspace, or back to normal from an empty line.
    DeleteLetter,
    DeleteWord,
    DeleteToStart,
    /// Return: the command run, or the search made.
    Run,
    Open,
    /// The command line's menu: the next line or the one before written
    /// in, the one chosen taken, or the menu put away.
    NextRow,
    PreviousRow,
    TakeName,
    MenuAway,
}

impl LineKey {
    /// Whether it's the menu's, rather than the line's own.
    pub fn menus(self) -> bool {
        matches!(
            self,
            LineKey::NextRow | LineKey::PreviousRow | LineKey::TakeName | LineKey::MenuAway
        )
    }
}

/// The acts of the find line and the command line, by their names, and
/// what each does as the key list says it.
const LINES: &[(&str, LineKey, &str)] = &[
    (
        "back",
        LineKey::Back,
        "Cancel the line, a selection or a count",
    ),
    ("leave", LineKey::Leave, "Cancel the line"),
    (
        "delete-letter",
        LineKey::DeleteLetter,
        "Delete a letter, or leave an empty line",
    ),
    ("delete-word", LineKey::DeleteWord, "Delete the word before"),
    (
        "delete-to-start",
        LineKey::DeleteToStart,
        "Delete everything typed",
    ),
    ("run", LineKey::Run, "Run the command typed"),
    ("open", LineKey::Open, "Search for what's typed"),
    (
        "next-row",
        LineKey::NextRow,
        "Choose the next command in the menu",
    ),
    (
        "previous-row",
        LineKey::PreviousRow,
        "Choose the command before in the menu",
    ),
    ("take-name", LineKey::TakeName, "Take the command chosen"),
    ("menu-away", LineKey::MenuAway, "Put the menu away"),
];

/// A key of a line, what it does, and whether it's the command line's,
/// the find line's, or both.
type LineSlot = (Key, LineKey, bool, bool);

/// The reader's own keys: a browser's zoom, at rest and while selecting,
/// for what Vim has nothing for.
const OWN: &[(&[Key], Act, In)] = &[
    (&[Ctrl('=')], Act::ZoomIn, In::Both),
    (&[Ctrl('+')], Act::ZoomIn, In::Both),
    (&[Ctrl('-')], Act::ZoomOut, In::Both),
    (&[Ctrl('0')], Act::ZoomBack, In::Both),
];

/// A sequence of keys, where it works and what it does.
pub type Sequence = (Vec<Key>, Act, In);

/// Every key the reader takes: each sequence, and each key that takes a
/// letter after it, with where each works.
pub struct Table {
    pub sequences: Vec<Sequence>,
    slots: Vec<(Key, Slot, In)>,
    operators: Vec<(Key, Act, In)>,
    lines: Vec<LineSlot>,
}

/// Each arrow and the letter it still stands for here.
const ARROWS: [(&str, &str); 4] = [
    ("<Down>", "j"),
    ("<Up>", "k"),
    ("<Left>", "h"),
    ("<Right>", "l"),
];

/// The keys, read from the language once.
pub fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| read(LANGUAGE))
}

/// The reader's table from a language. An act the reader takes that the
/// language doesn't name, a key of it the reader can't be given, or an act
/// in neither normal nor visual mode, is a fault of the build, which the
/// tests find before it ships.
fn read(language: &str) -> Table {
    let language: toml::Table = language.parse().expect("the key language is TOML");
    // The reader still takes an arrow as its letter in normal and visual
    // mode, so an act with the letter takes the arrow too, listed before
    // it. The key language no longer says so, since no arrow does anything
    // in normal (Joel, 2026-11-05); the reader keeps them until its own
    // piece takes them out, and is on the language check's STILL_ARROWED
    // until then.
    let arrows: Vec<(Vec<Key>, Vec<Key>)> = ARROWS
        .iter()
        .map(|(arrow, letter)| {
            (
                parse(letter).expect("an arrow's letter is a key"),
                parse(arrow).expect("an arrow is a key"),
            )
        })
        .collect();
    let keys = |name: &str| -> (Vec<Vec<Key>>, In) {
        let act = &language["acts"][name];
        let keys = act["keys"]
            .as_array()
            .unwrap_or_else(|| panic!("the key language has no act {name}"))
            .iter()
            .flat_map(|notation| {
                let notation = notation.as_str().expect("a key is a string");
                let keys = parse(notation)
                    .unwrap_or_else(|| panic!("the reader can't be given {notation}, for {name}"));
                let mut taken: Vec<Vec<Key>> = arrows
                    .iter()
                    .filter(|(letter, _)| *letter == keys)
                    .map(|(_, arrow)| arrow.clone())
                    .collect();
                taken.push(keys);
                taken
            })
            .collect();
        // An act is pressed in normal mode unless the language says.
        let modes: Vec<&str> = act
            .get("modes")
            .and_then(|modes| modes.as_array())
            .map_or_else(
                || vec!["normal"],
                |modes| modes.iter().filter_map(|mode| mode.as_str()).collect(),
            );
        let held =
            In::of(&modes).unwrap_or_else(|| panic!("{name} is in neither normal nor visual mode"));
        (keys, held)
    };
    let mut sequences: Vec<Sequence> = SPOKEN
        .iter()
        .flat_map(|&(name, act)| {
            let (keys, held) = keys(name);
            keys.into_iter().map(move |keys| (keys, act, held))
        })
        .collect();
    sequences.extend(
        OWN.iter()
            .map(|(keys, act, held)| (keys.to_vec(), *act, *held)),
    );
    let slots = SLOTS
        .iter()
        .flat_map(|&(name, slot)| {
            let (keys, held) = keys(name);
            keys.into_iter().map(move |keys| match keys[..] {
                [key] => (key, slot, held),
                _ => panic!("{name} takes its letter after one key"),
            })
        })
        .collect();
    let operators = OPERATORS
        .iter()
        .flat_map(|&(name, act)| {
            let (keys, held) = keys(name);
            keys.into_iter().map(move |keys| match keys[..] {
                [key] => (key, act, held),
                _ => panic!("{name} takes its motion after one key"),
            })
        })
        .collect();
    Table {
        sequences,
        slots,
        operators,
        lines: read_lines(&language),
    }
}

/// The find line's and the command line's keys, from their acts' keys in
/// find and command mode; each is one key.
fn read_lines(language: &toml::Table) -> Vec<LineSlot> {
    LINES
        .iter()
        .flat_map(|&(name, line, _)| {
            let act = &language["acts"][name];
            let modes: Vec<&str> = act["modes"]
                .as_array()
                .unwrap_or_else(|| panic!("{name} names its modes"))
                .iter()
                .filter_map(|mode| mode.as_str())
                .collect();
            let (command, find) = (modes.contains(&"command"), modes.contains(&"find"));
            assert!(command || find, "{name} is on neither line");
            act["keys"]
                .as_array()
                .unwrap_or_else(|| panic!("the key language has no act {name}"))
                .iter()
                .map(move |notation| {
                    let notation = notation.as_str().expect("a key is a string");
                    match parse(notation).as_deref() {
                        Some(&[key]) => (key, line, command, find),
                        _ => panic!("the reader can't be given {notation}, for {name}"),
                    }
                })
        })
        .collect()
}

/// A command the language gives the page: the act's name, its whole
/// command and short form, whether it takes a range, and the act's first
/// key as the key list shows it, if the reader has one for it.
pub struct PageCommand {
    pub act: String,
    pub name: String,
    pub short: String,
    pub ranged: bool,
    pub key: Option<String>,
}

/// Every command the language gives the page, by its act's name.
pub fn commands() -> Vec<PageCommand> {
    let language: toml::Table = LANGUAGE.parse().expect("the key language is TOML");
    let acts = language["acts"]
        .as_table()
        .expect("the language's acts are a table");
    acts.iter()
        .filter(|(_, act)| {
            act.get("kinds")
                .and_then(|kinds| kinds.as_array())
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("page")))
        })
        .filter_map(|(name, act)| {
            let command = act.get("command")?.as_str()?.to_string();
            let short = act
                .get("short")
                .and_then(|short| short.as_str())
                .map_or_else(|| command.clone(), str::to_string);
            let key = SPOKEN
                .iter()
                .find(|(spoken, _)| spoken == name)
                .and_then(|&(_, done)| table().sequences.iter().find(|(_, act, _)| *act == done))
                .map(|(keys, _, _)| shown_all(keys));
            Some(PageCommand {
                act: name.clone(),
                name: command,
                short,
                ranged: act.get("range").and_then(toml::Value::as_bool) == Some(true),
                key,
            })
        })
        .collect()
}

/// What a key does on the command line, or on the find line when not.
pub fn line_key(key: Key, command: bool) -> Option<LineKey> {
    table()
        .lines
        .iter()
        .find(|&&(k, _, on_command, on_find)| {
            k == key && if command { on_command } else { on_find }
        })
        .map(|&(_, line, _, _)| line)
}

/// The lines' keys as the key list and the key map give them: each key,
/// what it does, and its act's name.
fn line_keys() -> impl Iterator<Item = (Key, &'static str, &'static str)> {
    table().lines.iter().map(|&(key, line, _, _)| {
        let &(name, _, does) = LINES
            .iter()
            .find(|(_, l, _)| *l == line)
            .expect("every line key is named");
        (key, does, name)
    })
}

/// Keys written in Neovim's notation, as j, gg, <C-d> and <Space>?, or
/// nothing for one the reader is never given.
fn parse(notation: &str) -> Option<Vec<Key>> {
    let mut keys = Vec::new();
    let mut rest = notation;
    while let Some(c) = rest.chars().next() {
        let named = rest
            .strip_prefix('<')
            .and_then(|after| after.split_once('>'));
        if let Some((name, after)) = named.filter(|(name, _)| !name.is_empty()) {
            keys.push(match name {
                "Space" => Char(' '),
                "lt" => Char('<'),
                "CR" => Key::Return,
                "Esc" => Key::Escape,
                "BS" => Key::Backspace,
                "Up" => Key::Up,
                "Down" => Key::Down,
                "Left" => Key::Left,
                "Right" => Key::Right,
                "PageUp" => Key::PageUp,
                "PageDown" => Key::PageDown,
                "F1" => Key::F1,
                "Tab" => Key::Tab,
                "S-Tab" => Key::BackTab,
                held => {
                    let mut letters = held.strip_prefix("C-")?.chars();
                    match (letters.next(), letters.next()) {
                        (Some(letter), None) => Ctrl(letter.to_ascii_lowercase()),
                        _ => return None,
                    }
                }
            });
            rest = after;
        } else {
            keys.push(Char(c));
            rest = &rest[c.len_utf8()..];
        }
    }
    Some(keys)
}

/// An act's name in the language, or nothing for one of the reader's own.
fn spoken(act: Act) -> &'static str {
    match act {
        Act::SetMark(_) => "set-mark",
        Act::GoToMark(_) => "go-to-mark",
        act => SPOKEN
            .iter()
            .chain(OPERATORS)
            .find(|&&(_, spoken)| spoken == act)
            .map_or("", |&(name, _)| name),
    }
}

/// The operators as the key list and the key map show them: the act, and
/// the key that waits for its motion.
fn operators() -> impl Iterator<Item = (Act, Key)> {
    table().operators.iter().map(|&(key, act, _)| (act, key))
}

/// The letter slots as the key list and the key map show them: the act it
/// starts, and the key.
fn slots() -> impl Iterator<Item = (Act, Key)> {
    table().slots.iter().map(|&(key, slot, _)| (slot('a'), key))
}

#[cfg(test)]
mod tests;
