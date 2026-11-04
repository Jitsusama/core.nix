//! The reader's keys: Neovim's own normal-mode and visual-mode keys, as
//! design.md's Vi's Keys gives them, read from one table so the desktop's
//! key system can replace it whole. A machine in front of the table keeps
//! what vi keeps between presses: a count, and a key waiting for its
//! second.

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
    /// While selecting, the cursor on by words as w, b and e, to a line's
    /// ends as 0 and $, and by paragraphs as { and }.
    WordOn,
    WordBack,
    WordEnd,
    LineStart,
    LineEnd,
    ParagraphBack,
    ParagraphOn,
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
    fn holds(self, selecting: bool) -> bool {
        match self {
            In::Both => true,
            In::Rest => !selecting,
            In::Selecting => selecting,
        }
    }
}

use Key::{Char, Ctrl};

/// Every sequence, where it works and what it does. A sequence whose last
/// key is a letter slot (m, ' and the backtick) takes its letter in
/// `Machine`, not here. An arrow comes before the letter it shares a row
/// with, since it's the one Joel presses.
pub const TABLE: &[(&[Key], Act, In)] = &[
    (&[Key::Down], Act::Down, In::Both),
    (&[Char('j')], Act::Down, In::Both),
    (&[Key::Up], Act::Up, In::Both),
    (&[Char('k')], Act::Up, In::Both),
    (&[Key::Left], Act::Left, In::Both),
    (&[Char('h')], Act::Left, In::Both),
    (&[Key::Right], Act::Right, In::Both),
    (&[Char('l')], Act::Right, In::Both),
    (&[Ctrl('e')], Act::ScrollDown, In::Both),
    (&[Ctrl('y')], Act::ScrollUp, In::Both),
    (&[Ctrl('d')], Act::HalfDown, In::Both),
    (&[Ctrl('u')], Act::HalfUp, In::Both),
    (&[Ctrl('f')], Act::ScreenDown, In::Both),
    (&[Key::PageDown], Act::ScreenDown, In::Both),
    (&[Ctrl('b')], Act::ScreenUp, In::Both),
    (&[Key::PageUp], Act::ScreenUp, In::Both),
    (&[Char('g'), Char('g')], Act::First, In::Both),
    (&[Char('G')], Act::Last, In::Both),
    (&[Char('/')], Act::SearchForward, In::Both),
    (&[Char('?')], Act::SearchBackward, In::Both),
    (&[Char('n')], Act::Next, In::Both),
    (&[Char('N')], Act::Previous, In::Both),
    (&[Char('\''), Char('\'')], Act::LastJump, In::Both),
    (&[Char('`'), Char('`')], Act::LastJump, In::Both),
    (&[Ctrl('o')], Act::JumpBack, In::Both),
    (&[Ctrl('i')], Act::JumpForward, In::Both),
    (&[Char('g'), Char('x')], Act::Links, In::Both),
    (&[Char('v')], Act::Select, In::Both),
    (&[Char('V')], Act::SelectPieces, In::Both),
    (&[Char('w')], Act::WordOn, In::Selecting),
    (&[Char('b')], Act::WordBack, In::Selecting),
    (&[Char('e')], Act::WordEnd, In::Selecting),
    (&[Char('0')], Act::LineStart, In::Selecting),
    (&[Char('$')], Act::LineEnd, In::Selecting),
    (&[Char('{')], Act::ParagraphBack, In::Selecting),
    (&[Char('}')], Act::ParagraphOn, In::Selecting),
    (&[Char('o')], Act::OtherEnd, In::Selecting),
    (
        &[Char('i'), Char('w')],
        Act::Object(Object::InWord),
        In::Selecting,
    ),
    (
        &[Char('a'), Char('w')],
        Act::Object(Object::AWord),
        In::Selecting,
    ),
    (
        &[Char('i'), Char('s')],
        Act::Object(Object::InSentence),
        In::Selecting,
    ),
    (
        &[Char('a'), Char('s')],
        Act::Object(Object::ASentence),
        In::Selecting,
    ),
    (
        &[Char('i'), Char('p')],
        Act::Object(Object::InParagraph),
        In::Selecting,
    ),
    (
        &[Char('a'), Char('p')],
        Act::Object(Object::AParagraph),
        In::Selecting,
    ),
    (
        &[Char('i'), Char('`')],
        Act::Object(Object::InCode),
        In::Selecting,
    ),
    (&[Char('y')], Act::Copy, In::Selecting),
    (&[Ctrl('c')], Act::Copy, In::Selecting),
    (&[Char('y'), Char('y')], Act::CopyPiece, In::Rest),
    (
        &[Char('y'), Char('i'), Char('p')],
        Act::CopyParagraph,
        In::Rest,
    ),
    (&[Char('q')], Act::Close, In::Both),
    (&[Char('Z'), Char('Z')], Act::Close, In::Both),
    (&[Char(' '), Char('w')], Act::Close, In::Both),
    (&[Char(' '), Char('?')], Act::ShowKeys, In::Both),
    (&[Ctrl('=')], Act::ZoomIn, In::Both),
    (&[Ctrl('+')], Act::ZoomIn, In::Both),
    (&[Ctrl('-')], Act::ZoomOut, In::Both),
    (&[Ctrl('0')], Act::ZoomBack, In::Both),
];

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
        Act::First => ("Jumping", "To the top, or a count's line"),
        Act::Last => ("Jumping", "To the end, or a count's line"),
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
        Act::ParagraphBack => ("Selecting", "Back a paragraph"),
        Act::ParagraphOn => ("Selecting", "On a paragraph"),
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
    let slots = [
        (Act::SetMark('a'), "m and a letter"),
        (Act::GoToMark('a'), "' and a letter"),
    ];
    let typed = TABLE.iter().map(|(keys, act, _)| (*act, shown_all(keys)));
    for (act, keys) in typed.chain(slots.iter().map(|(act, keys)| (*act, (*keys).to_string()))) {
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

/// Every chord, as the desktop's key map spells one, with the launcher's
/// group for it and what it does: a sequence's keys apart (g g), and a
/// mark's keys by the key before its letter. The launcher lists a mark's
/// keys with the jumps, since two keys make no group of their own.
pub fn chords() -> Vec<(String, String, &'static str)> {
    let slots = [(Act::SetMark('a'), "m"), (Act::GoToMark('a'), "'")];
    TABLE
        .iter()
        .map(|(keys, act, _)| {
            (
                *act,
                keys.iter()
                    .map(|&key| shown(key))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
        .chain(slots.iter().map(|(act, key)| (*act, (*key).to_string())))
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
            Some((group, chord, does))
        })
        .collect()
}

/// A letter slot: these keys take the next letter typed as a mark's name.
fn takes_a_letter(key: Key) -> Option<fn(char) -> Act> {
    match key {
        Char('m') => Some(Act::SetMark),
        Char('\'' | '`') => Some(Act::GoToMark),
        _ => None,
    }
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
            && let (Some(slot), Char(letter)) = (takes_a_letter(first), key)
            && letter.is_ascii_alphabetic()
        {
            return Some(self.finish(slot(letter)));
        }
        self.pending.push(key);
        let here = || TABLE.iter().filter(|(_, _, held)| held.holds(selecting));
        if let Some(&(_, act, _)) = here().find(|(keys, _, _)| *keys == self.pending.as_slice()) {
            return Some(self.finish(act));
        }
        let waits = here().any(|(keys, _, _)| keys.starts_with(&self.pending))
            || (self.pending.len() == 1 && takes_a_letter(key).is_some());
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
    fn every_listed_act_has_a_group_and_no_two_groups_share_a_name() {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for &(_, act, _) in TABLE {
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
            let here: Vec<&[Key]> = TABLE
                .iter()
                .filter(|(_, _, held)| held.holds(selecting))
                .map(|(keys, _, _)| *keys)
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
