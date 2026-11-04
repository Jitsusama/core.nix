//! The reader's keys: Neovim's own normal-mode and visual-mode keys, read
//! from the one key language every surface of ours speaks
//! (modules/keys/language.toml), by the names of the acts a page holds
//! there, with the modes each works in, and the reader's own keys for what
//! Vim has nothing for, as zoom. A machine in front of the table keeps what
//! vi keeps between presses: a count, and a key waiting for its second.

use std::sync::OnceLock;

use crate::select::Object;

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
}

/// What a key asks of the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Down or up by lines, or across by columns, as vi's j, k, l and h;
    /// while selecting, the selection's cursor.
    Down,
    Up,
    Left,
    Right,
    /// Down or up a line, keeping where the eye rests, as Ctrl+E and Ctrl+Y.
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
    /// H, M and L: the cursor, a line the page doesn't show at rest, to the
    /// top, middle or bottom line in view; what acts on the line here (zt,
    /// *, gf) acts on it.
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
    /// and a page has no cursor, so the label stands for it.
    Links,
    /// The page larger, smaller, or at its own size again, as a browser's
    /// Ctrl+=, Ctrl+- and Ctrl+0.
    ZoomIn,
    ZoomOut,
    ZoomBack,
    /// Lists every key here, as Space ? in Joel's Neovim.
    ShowKeys,
    /// v and V: a selection, of characters or of whole paragraphs, items
    /// and lines of code, from the first word in view, with the cursor
    /// there; the same key again ends it, and the other turns it into one
    /// of the other kind.
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
    /// y y and y i p: copies the first paragraph, item or line of code
    /// wholly in view, or its paragraph, since a page at rest has no cursor.
    CopyPiece,
    CopyParagraph,
    /// Escape at rest: nothing was in progress. In a window it does
    /// nothing, so a habitual second Escape never closes a page; while
    /// selecting, it ends the selection.
    Rest,
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
const LANGUAGE: &str = include_str!("../../keys/language.toml");

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
    ("copy-selection", Act::Copy),
    ("copy", Act::CopyPiece),
    ("copy-paragraph", Act::CopyParagraph),
    ("close", Act::Close),
    ("show-keys", Act::ShowKeys),
];

/// An act that takes the letter typed after its key, as a mark's name.
type Slot = fn(char) -> Act;

/// The acts whose key takes a letter after it, by their names.
const SLOTS: &[(&str, Slot)] = &[("set-mark", Act::SetMark), ("go-to-mark", Act::GoToMark)];

/// The reader's own keys, for what Vim has nothing for: a browser's zoom,
/// at rest and while selecting.
const OWN: &[(&[Key], Act)] = &[
    (&[Ctrl('=')], Act::ZoomIn),
    (&[Ctrl('+')], Act::ZoomIn),
    (&[Ctrl('-')], Act::ZoomOut),
    (&[Ctrl('0')], Act::ZoomBack),
];

/// A sequence of keys, where it works and what it does.
pub type Sequence = (Vec<Key>, Act, In);

/// Every key the reader takes: each sequence, and each key that takes a
/// letter after it, with where each works.
pub struct Table {
    pub sequences: Vec<Sequence>,
    slots: Vec<(Key, Slot, In)>,
}

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
    let keys = |name: &str| -> (Vec<Vec<Key>>, In) {
        let act = &language["acts"][name];
        let keys = act["keys"]
            .as_array()
            .unwrap_or_else(|| panic!("the key language has no act {name}"))
            .iter()
            .map(|notation| {
                let notation = notation.as_str().expect("a key is a string");
                parse(notation)
                    .unwrap_or_else(|| panic!("the reader can't be given {notation}, for {name}"))
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
            .map(|(keys, act)| (keys.to_vec(), *act, In::Both)),
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
    Table { sequences, slots }
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
            .find(|&&(_, spoken)| spoken == act)
            .map_or("", |&(name, _)| name),
    }
}

/// The letter slots as the key list and the key map show them: the act it
/// starts, and the key.
fn slots() -> impl Iterator<Item = (Act, Key)> {
    table().slots.iter().map(|&(key, slot, _)| (slot('a'), key))
}

/// The key list's groups, in the order it shows them.
pub const GROUPS: &[&str] = &[
    "Moving",
    "Jumping",
    "Searching",
    "Marks",
    "Selecting",
    "Text Objects",
    "Copying",
    "The Page",
];

/// What the key list says an act does, and its group, in the words his
/// Neovim's key map uses: a sentence, as keys.nix's `does`. Acts that do
/// the same on a page share their words, so the list gives them one row;
/// Escape at rest isn't listed. h and l move only a selection's cursor,
/// since the page fits the window, so they're listed with selecting.
pub fn named(act: Act) -> Option<(&'static str, &'static str)> {
    Some(match act {
        Act::Down | Act::ScrollDown => ("Moving", "Down a line"),
        Act::Up | Act::ScrollUp => ("Moving", "Up a line"),
        Act::HalfDown => ("Moving", "Down half a screen"),
        Act::HalfUp => ("Moving", "Up half a screen"),
        Act::ScreenDown => ("Moving", "Down a screen"),
        Act::ScreenUp => ("Moving", "Up a screen"),
        Act::ParagraphOn => ("Moving", "On a paragraph"),
        Act::ParagraphBack => ("Moving", "Back a paragraph"),
        Act::ViewTop => ("Moving", "To the top line in view"),
        Act::ViewMiddle => ("Moving", "To the middle line in view"),
        Act::ViewBottom => ("Moving", "To the bottom line in view"),
        Act::PlaceTop => ("Moving", "This line to the top"),
        Act::PlaceMiddle => ("Moving", "This line to the middle"),
        Act::PlaceBottom => ("Moving", "This line to the bottom"),
        Act::First => ("Jumping", "To the top, or a count's line"),
        Act::Last => ("Jumping", "To the end, or a count's line"),
        Act::HeadingOn => ("Jumping", "On to the next heading"),
        Act::HeadingBack => ("Jumping", "Back to the heading before"),
        Act::Outline => ("Jumping", "Go to a heading from a list of them"),
        Act::Bracket => ("Jumping", "To a bracket's other end, or a count's percent"),
        Act::FollowLink => ("Jumping", "Follow the link here"),
        Act::FollowFile => ("Jumping", "Open the file a link here names"),
        Act::FileOn => ("Jumping", "On to the next markdown file here"),
        Act::FileBack => ("Jumping", "Back to the markdown file before"),
        Act::SameWordOn => ("Searching", "Search down for the word here"),
        Act::SameWordBack => ("Searching", "Search up for the word here"),
        Act::Command => ("The Page", "A command: a line's number, q or noh"),
        Act::LastJump => ("Jumping", "Back to before the last jump"),
        Act::JumpBack => ("Jumping", "Back through the jumps"),
        Act::JumpForward => ("Jumping", "Forward through the jumps"),
        Act::Links => ("Jumping", "Follow a link on screen, by its label"),
        Act::SearchForward => ("Searching", "Search down the page"),
        Act::SearchBackward => ("Searching", "Search up the page"),
        Act::Next => ("Searching", "On to the next match"),
        Act::Previous => ("Searching", "Back to the match before"),
        Act::SetMark(_) => ("Marks", "Mark this place with a letter"),
        Act::GoToMark(_) => ("Marks", "Go to a letter's mark"),
        Act::Select => ("Selecting", "Select from the first word in view"),
        Act::SelectPieces => ("Selecting", "Select whole paragraphs, items and lines"),
        Act::Left => ("Selecting", "A character left"),
        Act::Right => ("Selecting", "A character right"),
        Act::WordOn => ("Selecting", "On to the next word"),
        Act::WordBack => ("Selecting", "Back to a word's start"),
        Act::WordEnd => ("Selecting", "On to a word's end"),
        Act::LineStart => ("Selecting", "To the line's start"),
        Act::LineEnd => ("Selecting", "To the line's end"),
        Act::OtherEnd => ("Selecting", "To the selection's other end"),
        Act::Object(Object::InWord) => ("Text Objects", "A word"),
        Act::Object(Object::AWord) => ("Text Objects", "A word and the space after it"),
        Act::Object(Object::InSentence) => ("Text Objects", "A sentence"),
        Act::Object(Object::ASentence) => ("Text Objects", "A sentence and the space after it"),
        Act::Object(Object::InParagraph) => ("Text Objects", "A paragraph"),
        Act::Object(Object::AParagraph) => {
            ("Text Objects", "A paragraph and the blank line after it")
        }
        Act::Object(Object::InCode) => ("Text Objects", "The code between backticks"),
        Act::Copy => ("Copying", "Copy the selection as markdown"),
        Act::CopyPiece => ("Copying", "Copy the first block in view"),
        Act::CopyParagraph => ("Copying", "Copy the paragraph in view"),
        Act::ZoomIn => ("The Page", "Larger"),
        Act::ZoomOut => ("The Page", "Smaller"),
        Act::ZoomBack => ("The Page", "Back to its own size"),
        Act::ShowKeys => ("The Page", "Show every key here"),
        Act::Close => ("The Page", "Close the page"),
        Act::Rest => return None,
    })
}

/// A key as the list shows it, in the key map's words: Ctrl+D, Page Down.
fn shown(key: Key) -> String {
    match key {
        Char(' ') => "Space".into(),
        Char(c) => c.to_string(),
        // The keys a chord joins with +, by name, as the key map spells them.
        Ctrl('=') => "Ctrl+Equal".into(),
        Ctrl('+') => "Ctrl+Plus".into(),
        Ctrl('-') => "Ctrl+Minus".into(),
        Ctrl(c) => format!("Ctrl+{}", c.to_ascii_uppercase()),
        Key::Escape => "Escape".into(),
        Key::Return => "Return".into(),
        Key::Backspace => "Backspace".into(),
        Key::Up => "Up".into(),
        Key::Down => "Down".into(),
        Key::Left => "Left".into(),
        Key::Right => "Right".into(),
        Key::PageUp => "Page Up".into(),
        Key::PageDown => "Page Down".into(),
        Key::F1 => "F1".into(),
    }
}

/// A key as the desktop's key map spells it, which names a key of two
/// words in one, as `Page_Down`, since a space there parts the keys of a
/// sequence.
fn spelled(key: Key) -> String {
    match key {
        Key::PageUp => "Page_Up".into(),
        Key::PageDown => "Page_Down".into(),
        key => shown(key),
    }
}

/// A sequence as the list shows it: letters typed one after another run
/// together, as gg, and a named key stands apart, as Space w.
fn shown_all(keys: &[Key]) -> String {
    let mut out = String::new();
    for (i, &key) in keys.iter().enumerate() {
        let word = shown(key);
        let joins = matches!(key, Char(c) if c != ' ')
            && i > 0
            && matches!(keys[i - 1], Char(c) if c != ' ');
        if i > 0 && !joins {
            out.push(' ');
        }
        out.push_str(&word);
    }
    out
}

/// A row of the key list: the ways to type it side by side, and what it
/// does.
pub type Row = (Vec<String>, &'static str);

/// Every row of the key list, by group in the list's order.
pub fn listing() -> Vec<(&'static str, Vec<Row>)> {
    let mut rows: Vec<(&'static str, Vec<String>, &'static str)> = Vec::new();
    let typed = table()
        .sequences
        .iter()
        .map(|(keys, act, _)| (*act, shown_all(keys)));
    let slots = slots().map(|(act, key)| (act, format!("{} and a letter", shown(key))));
    for (act, keys) in typed.chain(slots) {
        let Some((group, does)) = named(act) else {
            continue;
        };
        match rows.iter_mut().find(|(g, _, d)| *g == group && *d == does) {
            Some((_, ways, _)) => ways.push(keys),
            None => rows.push((group, vec![keys], does)),
        }
    }
    GROUPS
        .iter()
        .map(|&group| {
            let mine = rows
                .iter()
                .filter(|(g, _, _)| *g == group)
                .map(|(_, ways, does)| (ways.clone(), *does));
            (group, mine.collect())
        })
        .collect()
}

/// A chord as the desktop's key map lists it.
pub struct Listed {
    /// The launcher's group for it.
    pub group: String,
    /// The keys, as the key map spells them.
    pub chord: String,
    pub does: &'static str,
    /// The act's name in the key language, or nothing for the reader's own.
    pub act: &'static str,
}

/// Every chord, as the desktop's key map spells one, with the launcher's
/// group for it, what it does and its act: a sequence's keys apart (g g),
/// and a mark's keys by the key before its letter. The launcher lists a
/// mark's keys with the jumps, since two keys make no group of their own.
pub fn chords() -> Vec<Listed> {
    table()
        .sequences
        .iter()
        .map(|(keys, act, _)| {
            (
                *act,
                keys.iter()
                    .map(|&key| spelled(key))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
        .chain(slots().map(|(act, key)| (act, spelled(key))))
        .filter_map(|(act, chord)| {
            let (group, does) = named(act)?;
            // Moving is the picture viewer's group in the launcher, so the
            // reader's is Scrolling there.
            let group = match group {
                "Marks" => "jumping".to_string(),
                "Moving" => "scrolling".to_string(),
                "Text Objects" => "objects".to_string(),
                "The Page" => "page".to_string(),
                other => other.to_lowercase(),
            };
            Some(Listed {
                group,
                chord,
                does,
                act: spoken(act),
            })
        })
        .collect()
}

/// A letter slot: these keys take the next letter typed as a mark's name,
/// where they work.
fn takes_a_letter(key: Key, selecting: bool) -> Option<Slot> {
    table()
        .slots
        .iter()
        .find(|&&(slot, _, held)| slot == key && held.holds(selecting))
        .map(|&(_, act, _)| act)
}

/// What vi keeps between presses.
#[derive(Default)]
pub struct Machine {
    count: Option<u32>,
    pending: Vec<Key>,
}

impl Machine {
    /// Takes one key, at rest or while selecting. Gives an act and its
    /// count once a sequence is whole; nothing while one waits for more,
    /// or when a key means nothing here, which drops whatever was in
    /// progress, as vi does.
    pub fn press(&mut self, key: Key, selecting: bool) -> Option<(Act, Option<u32>)> {
        if key == Key::Escape {
            let busy = self.count.is_some() || !self.pending.is_empty();
            self.reset();
            return if busy { None } else { Some((Act::Rest, None)) };
        }
        if let Char(digit @ '0'..='9') = key {
            // A 0 starts no count, as vi's 0 is its own motion: to the
            // line's start while selecting, and nothing at rest, where
            // there's no cursor to move.
            if self.pending.is_empty() && (digit != '0' || self.count.is_some()) {
                let value = digit.to_digit(10).unwrap_or(0);
                let count = self
                    .count
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(value);
                self.count = Some(count.min(999_999));
                return None;
            }
        }
        if let [first] = self.pending[..]
            && let (Some(slot), Char(letter)) = (takes_a_letter(first, selecting), key)
            && letter.is_ascii_alphabetic()
        {
            return Some(self.finish(slot(letter)));
        }
        self.pending.push(key);
        let here = || {
            table()
                .sequences
                .iter()
                .filter(|(_, _, held)| held.holds(selecting))
        };
        if let Some(&(_, act, _)) = here().find(|(keys, _, _)| *keys == self.pending.as_slice()) {
            return Some(self.finish(act));
        }
        let waits = here().any(|(keys, _, _)| keys.starts_with(&self.pending))
            || (self.pending.len() == 1 && takes_a_letter(key, selecting).is_some());
        if !waits {
            self.reset();
        }
        None
    }

    /// Whether a key is waiting for its second, or a count is being typed.
    #[cfg(test)]
    pub fn busy(&self) -> bool {
        self.count.is_some() || !self.pending.is_empty()
    }

    fn finish(&mut self, act: Act) -> (Act, Option<u32>) {
        let count = self.count;
        self.reset();
        (act, count)
    }

    fn reset(&mut self) {
        self.count = None;
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_of(group: &str) -> Vec<Row> {
        listing()
            .into_iter()
            .find_map(|(g, rows)| (g == group).then_some(rows))
            .unwrap_or_else(|| panic!("{group} isn't listed"))
    }

    #[test]
    fn the_list_gives_each_thing_one_row_with_every_way_to_type_it() {
        let moving = rows_of("Moving");
        let down = moving
            .iter()
            .find(|(_, does)| *does == "Down a line")
            .expect("a line down");
        assert_eq!(
            down.0,
            ["Down", "j", "Ctrl+E"],
            "the arrow before its letter"
        );
        assert!(rows_of("Jumping").iter().any(|(ways, _)| ways == &["gg"]));
        let page = rows_of("The Page");
        assert!(page.iter().any(|(ways, _)| ways == &["q", "ZZ", "Space w"]));
        assert!(
            page.iter()
                .any(|(ways, _)| ways == &["Ctrl+Equal", "Ctrl+Plus"])
        );
        assert!(listing().iter().all(|(_, rows)| !rows.is_empty()));
    }

    #[test]
    fn the_list_says_selecting_text_objects_and_copying_with_arrows_first() {
        let selecting = rows_of("Selecting");
        assert!(selecting.iter().any(|(ways, _)| ways == &["Left", "h"]));
        assert!(selecting.iter().any(|(ways, _)| ways == &["Right", "l"]));
        assert!(selecting.iter().any(|(ways, _)| ways == &["v"]));
        assert!(selecting.iter().any(|(ways, _)| ways == &["V"]));
        let objects = rows_of("Text Objects");
        assert_eq!(objects.len(), 7);
        assert!(objects.iter().any(|(ways, _)| ways == &["i`"]));
        let copying = rows_of("Copying");
        assert!(copying.iter().any(|(ways, _)| ways == &["y", "Ctrl+C"]));
        assert!(copying.iter().any(|(ways, _)| ways == &["yy"]));
        assert!(copying.iter().any(|(ways, _)| ways == &["yip"]));
    }

    #[test]
    fn every_act_the_reader_speaks_is_the_languages_and_every_key_of_it_is_taken() {
        // Reading the language panics on an act it doesn't name or a key the
        // reader can't be given.
        let table = read(LANGUAGE);
        for &(name, act) in SPOKEN {
            assert!(
                table.sequences.iter().any(|&(_, taken, _)| taken == act),
                "{name} has no keys"
            );
        }
        assert_eq!(table.slots.len(), 3, "m, ' and the backtick take a letter");
    }

    #[test]
    fn keys_are_read_in_neovims_notation() {
        assert_eq!(parse("gg"), Some(vec![Char('g'), Char('g')]));
        assert_eq!(parse("<C-d>"), Some(vec![Ctrl('d')]));
        assert_eq!(parse("<Space>?"), Some(vec![Char(' '), Char('?')]));
        assert_eq!(parse("<PageDown>"), Some(vec![Key::PageDown]));
        assert_eq!(parse("<lt>"), Some(vec![Char('<')]));
        assert_eq!(parse("<"), Some(vec![Char('<')]));
        assert_eq!(parse("<S-Tab>"), None);
    }

    #[test]
    fn the_key_map_spells_a_key_of_two_words_as_one() {
        let chords = chords();
        let screen = |chord: &str| {
            chords
                .iter()
                .any(|c| c.chord == chord && c.act == "screen-down")
        };
        assert!(screen("Page_Down") && screen("Ctrl+F"));
        assert!(chords.iter().any(|c| c.chord == "g g" && c.act == "first"));
        assert!(
            chords
                .iter()
                .any(|c| c.chord == "`" && c.act == "go-to-mark")
        );
        assert!(
            chords
                .iter()
                .any(|c| c.chord == "Ctrl+Equal" && c.act.is_empty())
        );
    }

    #[test]
    fn every_listed_act_has_a_group_and_no_two_groups_share_a_name() {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for &(_, act, _) in &table().sequences {
            if let Some((group, name)) = named(act) {
                assert!(
                    GROUPS.contains(&group),
                    "{act:?} is in {group}, which isn't shown"
                );
                if let Some(other) = seen.iter().find(|(g, n)| *n == name && *g != group) {
                    panic!("{name} is in both {} and {group}", other.0);
                }
                seen.push((group, name));
            }
        }
    }

    fn typed(machine: &mut Machine, text: &str) -> Vec<(Act, Option<u32>)> {
        typed_in(machine, text, false)
    }

    fn typed_in(machine: &mut Machine, text: &str, selecting: bool) -> Vec<(Act, Option<u32>)> {
        text.chars()
            .filter_map(|c| machine.press(Char(c), selecting))
            .collect()
    }

    #[test]
    fn a_letter_moves_at_once() {
        let mut machine = Machine::default();
        assert_eq!(
            typed(&mut machine, "jk"),
            [(Act::Down, None), (Act::Up, None)]
        );
    }

    #[test]
    fn the_arrows_move_as_the_letters_do() {
        let mut machine = Machine::default();
        assert_eq!(machine.press(Key::Down, false), Some((Act::Down, None)));
        assert_eq!(
            machine.press(Key::PageUp, false),
            Some((Act::ScreenUp, None))
        );
        assert_eq!(machine.press(Key::Left, true), Some((Act::Left, None)));
        assert_eq!(machine.press(Key::Right, true), Some((Act::Right, None)));
        assert_eq!(machine.press(Key::Up, true), Some((Act::Up, None)));
    }

    #[test]
    fn a_count_goes_with_the_motion_after_it() {
        let mut machine = Machine::default();
        assert_eq!(typed(&mut machine, "12j"), [(Act::Down, Some(12))]);
        assert_eq!(typed(&mut machine, "3G"), [(Act::Last, Some(3))]);
        assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
        assert_eq!(typed_in(&mut machine, "3w", true), [(Act::WordOn, Some(3))]);
    }

    #[test]
    fn a_zero_alone_starts_no_count_and_goes_to_the_lines_start_while_selecting() {
        let mut machine = Machine::default();
        assert_eq!(typed(&mut machine, "0j"), [(Act::Down, None)]);
        assert_eq!(typed(&mut machine, "10j"), [(Act::Down, Some(10))]);
        assert_eq!(typed_in(&mut machine, "0", true), [(Act::LineStart, None)]);
        assert_eq!(
            typed_in(&mut machine, "20l", true),
            [(Act::Right, Some(20))]
        );
    }

    #[test]
    fn two_keys_make_one_act() {
        let mut machine = Machine::default();
        assert_eq!(typed(&mut machine, "g"), []);
        assert!(machine.busy());
        assert_eq!(typed(&mut machine, "g"), [(Act::First, None)]);
        assert_eq!(typed(&mut machine, "ZZ"), [(Act::Close, None)]);
        assert_eq!(typed(&mut machine, " w"), [(Act::Close, None)]);
        assert_eq!(typed(&mut machine, " ?"), [(Act::ShowKeys, None)]);
    }

    #[test]
    fn y_copies_at_once_while_selecting_and_waits_for_its_object_at_rest() {
        let mut machine = Machine::default();
        assert_eq!(typed_in(&mut machine, "y", true), [(Act::Copy, None)]);
        assert_eq!(machine.press(Ctrl('c'), true), Some((Act::Copy, None)));
        assert_eq!(
            machine.press(Ctrl('c'), false),
            None,
            "nothing to copy at rest"
        );
        assert_eq!(typed(&mut machine, "y"), []);
        assert_eq!(typed(&mut machine, "y"), [(Act::CopyPiece, None)]);
        assert_eq!(typed(&mut machine, "3yy"), [(Act::CopyPiece, Some(3))]);
        assert_eq!(typed(&mut machine, "yip"), [(Act::CopyParagraph, None)]);
    }

    #[test]
    fn text_objects_and_motions_work_only_while_selecting() {
        let mut machine = Machine::default();
        assert_eq!(
            typed_in(&mut machine, "iwapi`o$e{}", true),
            [
                (Act::Object(Object::InWord), None),
                (Act::Object(Object::AParagraph), None),
                (Act::Object(Object::InCode), None),
                (Act::OtherEnd, None),
                (Act::LineEnd, None),
                (Act::WordEnd, None),
                (Act::ParagraphBack, None),
                (Act::ParagraphOn, None),
            ]
        );
        assert_eq!(typed(&mut machine, "iwob"), [], "at rest they mean nothing");
        assert_eq!(typed(&mut machine, "v"), [(Act::Select, None)]);
        assert_eq!(
            typed_in(&mut machine, "V", true),
            [(Act::SelectPieces, None)]
        );
    }

    #[test]
    fn vis_paragraph_heading_and_view_keys_take_counts() {
        let mut machine = Machine::default();
        assert_eq!(
            typed(&mut machine, "}{2]][["),
            [
                (Act::ParagraphOn, None),
                (Act::ParagraphBack, None),
                (Act::HeadingOn, Some(2)),
                (Act::HeadingBack, None),
            ]
        );
        assert_eq!(
            typed(&mut machine, "H3MLztzzzb"),
            [
                (Act::ViewTop, None),
                (Act::ViewMiddle, Some(3)),
                (Act::ViewBottom, None),
                (Act::PlaceTop, None),
                (Act::PlaceMiddle, None),
                (Act::PlaceBottom, None),
            ]
        );
        assert_eq!(
            typed(&mut machine, "%50%*#"),
            [
                (Act::Bracket, None),
                (Act::Bracket, Some(50)),
                (Act::SameWordOn, None),
                (Act::SameWordBack, None),
            ]
        );
        // Only { and } of these move a selection's cursor; the rest are a
        // page's at rest.
        assert_eq!(
            typed_in(&mut machine, "{}H", true),
            [(Act::ParagraphBack, None), (Act::ParagraphOn, None)]
        );
    }

    #[test]
    fn files_links_the_outline_and_the_command_line_have_their_keys() {
        let mut machine = Machine::default();
        assert_eq!(
            typed(&mut machine, "]b2[bgfgO:"),
            [
                (Act::FileOn, None),
                (Act::FileBack, Some(2)),
                (Act::FollowFile, None),
                (Act::Outline, None),
                (Act::Command, None),
            ]
        );
        assert_eq!(
            machine.press(Ctrl(']'), false),
            Some((Act::FollowLink, None))
        );
        // ] waits for its second key, as g does, and a wrong one drops it.
        assert_eq!(typed(&mut machine, "]x"), []);
        assert!(!machine.busy());
    }

    #[test]
    fn a_wrong_second_key_drops_the_sequence_as_vi_does() {
        let mut machine = Machine::default();
        assert_eq!(typed(&mut machine, "gq"), []);
        assert!(!machine.busy());
        assert_eq!(typed(&mut machine, "gx"), [(Act::Links, None)]);
        assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
    }

    #[test]
    fn marks_take_the_letter_after_them() {
        let mut machine = Machine::default();
        assert_eq!(typed(&mut machine, "ma"), [(Act::SetMark('a'), None)]);
        assert_eq!(typed(&mut machine, "'a"), [(Act::GoToMark('a'), None)]);
        assert_eq!(typed(&mut machine, "`b"), [(Act::GoToMark('b'), None)]);
        assert_eq!(typed(&mut machine, "''"), [(Act::LastJump, None)]);
    }

    #[test]
    fn a_mark_is_made_while_selecting_as_vim_makes_it() {
        let mut machine = Machine::default();
        assert_eq!(
            typed_in(&mut machine, "ma", true),
            [(Act::SetMark('a'), None)]
        );
    }

    #[test]
    fn the_jumps_go_back_and_forward() {
        let mut machine = Machine::default();
        assert_eq!(machine.press(Ctrl('o'), false), Some((Act::JumpBack, None)));
        assert_eq!(
            machine.press(Ctrl('i'), false),
            Some((Act::JumpForward, None))
        );
    }

    #[test]
    fn escape_cancels_what_is_in_progress() {
        let mut machine = Machine::default();
        typed(&mut machine, "5g");
        assert_eq!(machine.press(Key::Escape, false), None);
        assert!(!machine.busy());
        assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
    }

    #[test]
    fn escape_at_rest_says_so_and_closes_nothing() {
        let mut machine = Machine::default();
        assert_eq!(machine.press(Key::Escape, false), Some((Act::Rest, None)));
        assert_eq!(machine.press(Key::Escape, true), Some((Act::Rest, None)));
    }

    #[test]
    fn q_closes_and_search_starts_both_ways() {
        let mut machine = Machine::default();
        assert_eq!(
            typed(&mut machine, "q/?nN"),
            [
                (Act::Close, None),
                (Act::SearchForward, None),
                (Act::SearchBackward, None),
                (Act::Next, None),
                (Act::Previous, None),
            ]
        );
    }

    #[test]
    fn every_sequence_in_the_table_is_reachable_and_none_twice() {
        for selecting in [false, true] {
            let here: Vec<&[Key]> = table()
                .sequences
                .iter()
                .filter(|(_, _, held)| held.holds(selecting))
                .map(|(keys, _, _)| keys.as_slice())
                .collect();
            for (i, keys) in here.iter().enumerate() {
                assert!(!here[..i].contains(keys), "{keys:?} is in the table twice");
                // No whole sequence is the start of a longer one, or the
                // longer could never be typed.
                assert!(
                    !here
                        .iter()
                        .any(|other| other.len() > keys.len() && other.starts_with(keys)),
                    "{keys:?} hides a longer sequence"
                );
            }
        }
    }
}
