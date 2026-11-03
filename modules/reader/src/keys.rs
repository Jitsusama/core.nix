//! The reader's keys: Neovim's own normal-mode keys, as design.md's Vi's
//! Keys gives them, read from one table so the desktop's key system can
//! replace it whole. A machine in front of the table keeps what vi keeps
//! between presses: a count, and a key waiting for its second.

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
    /// Down or up by lines, or across by columns, as vi's j, k, l and h.
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
    /// Escape at rest: nothing was in progress. In a window it does
    /// nothing, so a habitual second Escape never closes a page.
    Rest,
}

use Key::{Char, Ctrl};

/// Every sequence and what it does. A sequence whose last key is a letter
/// slot (m, ' and the backtick) takes its letter in `Machine`, not here.
pub const TABLE: &[(&[Key], Act)] = &[
    (&[Char('j')], Act::Down),
    (&[Key::Down], Act::Down),
    (&[Char('k')], Act::Up),
    (&[Key::Up], Act::Up),
    (&[Char('h')], Act::Left),
    (&[Key::Left], Act::Left),
    (&[Char('l')], Act::Right),
    (&[Key::Right], Act::Right),
    (&[Ctrl('e')], Act::ScrollDown),
    (&[Ctrl('y')], Act::ScrollUp),
    (&[Ctrl('d')], Act::HalfDown),
    (&[Ctrl('u')], Act::HalfUp),
    (&[Ctrl('f')], Act::ScreenDown),
    (&[Key::PageDown], Act::ScreenDown),
    (&[Ctrl('b')], Act::ScreenUp),
    (&[Key::PageUp], Act::ScreenUp),
    (&[Char('g'), Char('g')], Act::First),
    (&[Char('G')], Act::Last),
    (&[Char('/')], Act::SearchForward),
    (&[Char('?')], Act::SearchBackward),
    (&[Char('n')], Act::Next),
    (&[Char('N')], Act::Previous),
    (&[Char('\''), Char('\'')], Act::LastJump),
    (&[Char('`'), Char('`')], Act::LastJump),
    (&[Ctrl('o')], Act::JumpBack),
    (&[Ctrl('i')], Act::JumpForward),
    (&[Char('g'), Char('x')], Act::Links),
    (&[Char('q')], Act::Close),
    (&[Char('Z'), Char('Z')], Act::Close),
    (&[Char(' '), Char('w')], Act::Close),
    (&[Char(' '), Char('?')], Act::ShowKeys),
    (&[Ctrl('=')], Act::ZoomIn),
    (&[Ctrl('+')], Act::ZoomIn),
    (&[Ctrl('-')], Act::ZoomOut),
    (&[Ctrl('0')], Act::ZoomBack),
];

/// The key list's groups, in the order it shows them.
pub const GROUPS: &[&str] = &["Moving", "Jumping", "Searching", "Marks", "The Page"];

/// What the key list says an act does, and its group, in the words his
/// Neovim's key map uses: a sentence, as keys.nix's `does`. Acts that do
/// the same on a page share their words, so the list gives them one row;
/// h and l, which do nothing on a page that fits the window, and Escape at
/// rest aren't listed.
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
        Act::ZoomIn => ("The Page", "Larger"),
        Act::ZoomOut => ("The Page", "Smaller"),
        Act::ZoomBack => ("The Page", "Back to its own size"),
        Act::ShowKeys => ("The Page", "Show every key here"),
        Act::Close => ("The Page", "Close the page"),
        Act::Left | Act::Right | Act::Rest => return None,
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
    let typed = TABLE.iter().map(|(keys, act)| (*act, shown_all(keys)));
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
        .map(|(keys, act)| {
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
    /// Takes one key. Gives an act and its count once a sequence is whole;
    /// nothing while one waits for more, or when a key means nothing here,
    /// which drops whatever was in progress, as vi does.
    pub fn press(&mut self, key: Key) -> Option<(Act, Option<u32>)> {
        if key == Key::Escape {
            let busy = self.count.is_some() || !self.pending.is_empty();
            self.reset();
            return if busy { None } else { Some((Act::Rest, None)) };
        }
        if let Char(digit @ '0'..='9') = key {
            // A 0 starts no count, as vi's 0 is its own motion; there's no
            // column to go to on a page, so it's left meaning nothing.
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
        if let Some(&(_, act)) = TABLE
            .iter()
            .find(|(keys, _)| *keys == self.pending.as_slice())
        {
            return Some(self.finish(act));
        }
        let waits = TABLE
            .iter()
            .any(|(keys, _)| keys.starts_with(&self.pending))
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

    #[test]
    fn the_list_gives_each_thing_one_row_with_every_way_to_type_it() {
        let listing = listing();
        let moving = &listing
            .iter()
            .find(|(group, _)| *group == "Moving")
            .expect("Moving")
            .1;
        let down = moving
            .iter()
            .find(|(_, does)| *does == "Down a line")
            .expect("a line down");
        assert_eq!(down.0, ["j", "Down", "Ctrl+E"]);
        let jumping = &listing
            .iter()
            .find(|(group, _)| *group == "Jumping")
            .expect("Jumping")
            .1;
        assert!(jumping.iter().any(|(ways, _)| ways == &["gg"]));
        let page = &listing
            .iter()
            .find(|(group, _)| *group == "The Page")
            .expect("The Page")
            .1;
        assert!(page.iter().any(|(ways, _)| ways == &["q", "ZZ", "Space w"]));
        assert!(
            page.iter()
                .any(|(ways, _)| ways == &["Ctrl+Equal", "Ctrl+Plus"])
        );
        assert!(listing.iter().all(|(_, rows)| !rows.is_empty()));
    }

    #[test]
    fn every_listed_act_has_a_group_and_no_two_groups_share_a_name() {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for &(_, act) in TABLE {
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
        text.chars()
            .filter_map(|c| machine.press(Char(c)))
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
        assert_eq!(machine.press(Key::Down), Some((Act::Down, None)));
        assert_eq!(machine.press(Key::PageUp), Some((Act::ScreenUp, None)));
    }

    #[test]
    fn a_count_goes_with_the_motion_after_it() {
        let mut machine = Machine::default();
        assert_eq!(typed(&mut machine, "12j"), [(Act::Down, Some(12))]);
        assert_eq!(typed(&mut machine, "3G"), [(Act::Last, Some(3))]);
        assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
    }

    #[test]
    fn a_zero_alone_starts_no_count() {
        let mut machine = Machine::default();
        assert_eq!(typed(&mut machine, "0j"), [(Act::Down, None)]);
        assert_eq!(typed(&mut machine, "10j"), [(Act::Down, Some(10))]);
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
        assert_eq!(machine.press(Ctrl('o')), Some((Act::JumpBack, None)));
        assert_eq!(machine.press(Ctrl('i')), Some((Act::JumpForward, None)));
    }

    #[test]
    fn escape_cancels_what_is_in_progress() {
        let mut machine = Machine::default();
        typed(&mut machine, "5g");
        assert_eq!(machine.press(Key::Escape), None);
        assert!(!machine.busy());
        assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
    }

    #[test]
    fn escape_at_rest_says_so_and_closes_nothing() {
        let mut machine = Machine::default();
        assert_eq!(machine.press(Key::Escape), Some((Act::Rest, None)));
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
        for (i, (keys, _)) in TABLE.iter().enumerate() {
            assert!(
                !TABLE[..i].iter().any(|(earlier, _)| earlier == keys),
                "{keys:?} is in the table twice"
            );
            // No whole sequence is the start of a longer one, or the longer
            // could never be typed.
            assert!(
                !TABLE
                    .iter()
                    .any(|(other, _)| other.len() > keys.len() && other.starts_with(keys)),
                "{keys:?} hides a longer sequence"
            );
        }
    }
}
