//! The key list and the chords: every key the reader binds, grouped and
//! named as the key language names its acts, shown as they are typed.

use crate::select::Object;

use super::Key::{Char, Ctrl};
use super::{Act, Key, line_keys, operators, slots, spoken, table};

/// The key list's groups, in the order it shows them. The lines in view
/// and the links and files are groups of their own, so that no group lists
/// more rows than the launcher's thirteen, Neovim's longest.
pub const GROUPS: &[&str] = &[
    "Moving",
    "In View",
    "Jumping",
    "Links and Files",
    "Searching",
    "Marks",
    "Selecting",
    "Text Objects",
    "Copying",
    "The Page",
    "Command Line",
];

/// What the key list says an act does, and its group, in the words his
/// Neovim's key map uses: a sentence, as keys.nix's `does`. Acts that do
/// the same on a page share their words, so the list gives them one row;
/// Escape at rest isn't listed. h and l move only a selection's cursor,
/// since the page fits the window, so they're listed with selecting.
pub fn named(act: Act) -> Option<(&'static str, &'static str)> {
    Some(match act {
        Act::Down => ("Moving", "Down a line"),
        Act::Up => ("Moving", "Up a line"),
        Act::ScrollDown => ("Moving", "The view down a line"),
        Act::ScrollUp => ("Moving", "The view up a line"),
        Act::HalfDown => ("Moving", "Down half a screen"),
        Act::HalfUp => ("Moving", "Up half a screen"),
        Act::ScreenDown => ("Moving", "Down a screen"),
        Act::ScreenUp => ("Moving", "Up a screen"),
        Act::ParagraphOn => ("Moving", "On a paragraph"),
        Act::ParagraphBack => ("Moving", "Back a paragraph"),
        Act::ViewTop => ("In View", "To the top line in view"),
        Act::ViewMiddle => ("In View", "To the middle line in view"),
        Act::ViewBottom => ("In View", "To the bottom line in view"),
        Act::PlaceTop => ("In View", "This line to the top"),
        Act::PlaceMiddle => ("In View", "This line to the middle"),
        Act::PlaceBottom => ("In View", "This line to the bottom"),
        Act::First => ("Jumping", "To the top, or a count's line"),
        Act::Last => ("Jumping", "To the end, or a count's line"),
        Act::HeadingOn => ("Jumping", "On to the next heading"),
        Act::HeadingBack => ("Jumping", "Back to the heading before"),
        Act::Outline => ("Jumping", "Go to a heading from a list of them"),
        Act::Bracket => ("Jumping", "To a bracket's other end, or a count's percent"),
        Act::FollowLink => ("Links and Files", "Follow the link here"),
        Act::FollowFile => ("Links and Files", "Open the file a link here names"),
        Act::FileOn => ("Links and Files", "On to the next markdown file here"),
        Act::FileBack => ("Links and Files", "Back to the markdown file before"),
        Act::SameWordOn => ("Searching", "Search down for the word here"),
        Act::SameWordBack => ("Searching", "Search up for the word here"),
        Act::Command => ("The Page", "A command: a line's number, q or noh"),
        Act::LastJump => ("Jumping", "Back to before the last jump"),
        Act::JumpBack => ("Jumping", "Back through the jumps"),
        Act::JumpForward => ("Jumping", "Forward through the jumps"),
        Act::Links => ("Links and Files", "Follow a link on screen, by its label"),
        Act::SearchForward => ("Searching", "Search down the page"),
        Act::SearchBackward => ("Searching", "Search up the page"),
        Act::Next => ("Searching", "On to the next match"),
        Act::Previous => ("Searching", "Back to the match before"),
        Act::SetMark(_) => ("Marks", "Mark this place with a letter"),
        Act::GoToMark(_) => ("Marks", "Go to a letter's mark"),
        Act::Select => ("Selecting", "Select from the cursor's line"),
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
        Act::Object(Object::ACode) => ("Text Objects", "The code, its backticks and a space"),
        Act::Copy => ("Copying", "Copy the selection as markdown"),
        Act::CopyPiece => ("Copying", "Copy the block here"),
        Act::CopyParagraph => ("Copying", "Copy the paragraph here"),
        Act::CopyAParagraph => ("Copying", "Copy the paragraph and the blank line after it"),
        Act::CopyToParagraphEnd => ("Copying", "Copy to the paragraph's end"),
        Act::ZoomIn => ("The Page", "Larger"),
        Act::ZoomOut => ("The Page", "Smaller"),
        Act::ZoomBack => ("The Page", "Back to its own size"),
        Act::ShowKeys => ("The Page", "Show every key here"),
        Act::AgainCommand => ("Command Line", "Run the last command again"),
        Act::Close => ("The Page", "Close the page"),
        Act::Change(_) => ("The Page", "Nothing: a page can't be changed"),
        Act::Rest => return None,
    })
}

/// A key as the list shows it, in the key map's words: Ctrl+D, Page Down.
pub(super) fn shown(key: Key) -> String {
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
        Key::Tab => "Tab".into(),
        Key::BackTab => "Shift+Tab".into(),
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
pub(super) fn shown_all(keys: &[Key]) -> String {
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
        .filter(|(keys, _, _)| !an_arrow(keys))
        .map(|(keys, act, _)| (*act, shown_all(keys)));
    let slots = slots().map(|(act, key)| (act, format!("{} and a letter", shown(key))));
    let operators = operators().map(|(act, key)| (act, shown(key)));
    for (act, keys) in typed.chain(slots).chain(operators) {
        let Some((group, does)) = named(act) else {
            continue;
        };
        match rows.iter_mut().find(|(g, _, d)| *g == group && *d == does) {
            Some((_, ways, _)) => ways.push(keys),
            None => rows.push((group, vec![keys], does)),
        }
    }
    for (key, does, _) in line_keys().filter(|(key, _, _)| !an_arrow(&[*key])) {
        match rows
            .iter_mut()
            .find(|(g, _, d)| *g == "Command Line" && *d == does)
        {
            Some((_, ways, _)) => ways.push(shown(key)),
            None => rows.push(("Command Line", vec![shown(key)], does)),
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

/// Whether keys are an arrow alone. An arrow moves as its letter does, and
/// on a line as a line's arrows do, but no list names one, as Joel asked:
/// the letters are what's shown.
fn an_arrow(keys: &[Key]) -> bool {
    matches!(keys, [Key::Up | Key::Down | Key::Left | Key::Right])
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
        .filter(|(keys, _, _)| !an_arrow(keys))
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
        .chain(operators().map(|(act, key)| (act, spelled(key))))
        .filter_map(|(act, chord)| {
            let (group, does) = named(act)?;
            // Moving is the picture viewer's group in the launcher, so the
            // reader's is Scrolling there.
            let group = match group {
                "Marks" => "jumping".to_string(),
                "Moving" => "scrolling".to_string(),
                "In View" => "view".to_string(),
                "Links and Files" => "links".to_string(),
                "Text Objects" => "objects".to_string(),
                "The Page" => "page".to_string(),
                "Command Line" => "commands".to_string(),
                other => other.to_lowercase(),
            };
            Some(Listed {
                group,
                chord,
                does,
                act: spoken(act),
            })
        })
        .chain(
            line_keys()
                .filter(|(key, _, _)| !an_arrow(&[*key]))
                .map(|(key, does, act)| Listed {
                    group: "commands".to_string(),
                    chord: spelled(key),
                    does,
                    act,
                }),
        )
        .collect()
}
