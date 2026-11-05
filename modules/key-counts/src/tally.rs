//! The counts themselves: for each keyboard, layer, modifiers held and key,
//! how many times it was pressed and how many of those presses were held
//! long enough to repeat. Nothing else is kept, so what was typed can't be
//! read back: not the order keys came in, not when, and not a word.

use std::collections::{BTreeMap, HashMap};

/// Which keyboard a key came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Board {
    /// The ErgoDox, whose firmware lays out the layers.
    Ergodox,
    /// The laptop's own keyboard, as keyd sends it on.
    Laptop,
}

impl Board {
    fn name(self) -> &'static str {
        match self {
            Self::Ergodox => "ergodox",
            Self::Laptop => "laptop",
        }
    }
}

/// One line of the counts: keyboard, layer, modifiers and key.
type Entry = (String, String, String, String);

/// How many presses, and how many of them held.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
struct Count {
    presses: u64,
    holds: u64,
}

const MODIFIERS: [(u16, u8); 8] = [
    (29, CTRL),
    (97, CTRL << 4),
    (42, SHIFT),
    (54, SHIFT << 4),
    (56, ALT),
    (100, ALT << 4),
    (125, SUPER),
    (126, SUPER << 4),
];
const CTRL: u8 = 1;
const SHIFT: u8 = 2;
const ALT: u8 = 4;
const SUPER: u8 = 8;

fn modifier(code: u16) -> u8 {
    MODIFIERS
        .iter()
        .find(|(key, _)| *key == code)
        .map_or(0, |(_, bit)| *bit)
}

/// The modifiers held, either side, as `ctrl+shift`, or `-` for none.
fn modifiers(held: u8) -> String {
    let either = held | held >> 4;
    let names: Vec<_> = [
        (CTRL, "ctrl"),
        (SHIFT, "shift"),
        (ALT, "alt"),
        (SUPER, "super"),
    ]
    .iter()
    .filter(|(bit, _)| either & bit != 0)
    .map(|(_, name)| *name)
    .collect();
    if names.is_empty() {
        "-".into()
    } else {
        names.join("+")
    }
}

/// The ErgoDox's layer a key came from, told by the key alone: Coal has no
/// number row or punctuation, so those come from Punk, and the keypad's keys
/// come from Numb (modules/ergodox/keymap/keymap.c). A key Punk or Numb lets
/// through from Coal counts as Coal's, since it does Coal's job.
fn layer(board: Board, code: u16) -> &'static str {
    match (board, code) {
        (Board::Laptop, _) => "base",
        (Board::Ergodox, 2..=13 | 26 | 27 | 39..=41 | 43 | 51..=53 | 87 | 88) => "punk",
        (Board::Ergodox, 55 | 71..=83 | 96 | 98 | 99 | 116) => "numb",
        (Board::Ergodox, _) => "coal",
    }
}

/// A key's name as the kernel's input codes call it, lowercase.
fn key(code: u16) -> String {
    const ROWS: [(u16, &str); 4] = [
        (2, "1234567890-="),
        (16, "qwertyuiop[]"),
        (30, "asdfghjkl;'`"),
        (43, "\\zxcvbnm,./"),
    ];
    for (start, keys) in ROWS {
        if let Some(ch) = code
            .checked_sub(start)
            .and_then(|at| keys.chars().nth(at as usize))
        {
            return ch.to_string();
        }
    }
    let name = match code {
        1 => "escape",
        14 => "backspace",
        15 => "tab",
        28 => "enter",
        29 => "leftctrl",
        42 => "leftshift",
        54 => "rightshift",
        55 => "kpasterisk",
        56 => "leftalt",
        57 => "space",
        58 => "capslock",
        59..=68 => return format!("f{}", code - 58),
        71..=83 => {
            return format!(
                "kp{}",
                [
                    "7", "8", "9", "minus", "4", "5", "6", "plus", "1", "2", "3", "0", "dot"
                ][usize::from(code - 71)]
            );
        }
        87 => "f11",
        88 => "f12",
        96 => "kpenter",
        97 => "rightctrl",
        98 => "kpslash",
        99 => "print",
        100 => "rightalt",
        102 => "home",
        103 => "up",
        104 => "pageup",
        105 => "left",
        106 => "right",
        107 => "end",
        108 => "down",
        109 => "pagedown",
        110 => "insert",
        111 => "delete",
        113 => "mute",
        114 => "volumedown",
        115 => "volumeup",
        116 => "power",
        125 => "leftmeta",
        126 => "rightmeta",
        127 => "compose",
        163 => "nextsong",
        164 => "playpause",
        165 => "previoussong",
        224 => "brightnessdown",
        225 => "brightnessup",
        _ => return format!("code{code}"),
    };
    name.into()
}

/// The counts, and what each keyboard holds down now, which is forgotten
/// as each key is let go.
#[derive(Default)]
pub struct Tally {
    counts: BTreeMap<Entry, Count>,
    held: HashMap<(u64, u16), (Entry, bool)>,
    modifiers: HashMap<u64, u8>,
    changed: bool,
}

impl Tally {
    /// Takes a key's change on `device`, one of `board`'s: `value` is the
    /// kernel's, 1 pressed, 2 repeating and 0 let go.
    pub fn key(&mut self, device: u64, board: Board, code: u16, value: i32) {
        let bit = modifier(code);
        match value {
            1 => {
                let held = self.modifiers.get(&device).copied().unwrap_or(0) & !bit;
                let entry = (
                    board.name().to_owned(),
                    layer(board, code).to_owned(),
                    modifiers(held),
                    key(code),
                );
                self.counts.entry(entry.clone()).or_default().presses += 1;
                self.held.insert((device, code), (entry, false));
                *self.modifiers.entry(device).or_default() |= bit;
                self.changed = true;
            }
            2 => {
                if let Some((entry, repeated @ false)) = self.held.get_mut(&(device, code)) {
                    *repeated = true;
                    self.counts.entry(entry.clone()).or_default().holds += 1;
                    self.changed = true;
                }
            }
            0 => {
                self.held.remove(&(device, code));
                *self.modifiers.entry(device).or_default() &= !bit;
            }
            _ => {}
        }
    }

    /// Forgets what a keyboard that went was holding.
    pub fn gone(&mut self, device: u64) {
        self.held.retain(|(held, _), _| *held != device);
        self.modifiers.remove(&device);
    }

    /// Whether a count changed since the counts were last taken.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Whether a count changed since it was last asked, which clears it.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// The counts as the file keeps them: a header, then a line for each
    /// keyboard, layer, modifiers and key, in that sorted order, so the
    /// file says nothing of which came first.
    pub fn render(&self) -> String {
        let lines = self
            .counts
            .iter()
            .map(|((board, layer, held, key), count)| {
                format!(
                    "{board}\t{layer}\t{held}\t{key}\t{}\t{}\n",
                    count.presses, count.holds
                )
            });
        std::iter::once("board\tlayer\tmodifiers\tkey\tpresses\tholds\n".to_owned())
            .chain(lines)
            .collect()
    }

    /// Adds the counts a file kept, so a restart carries on from them. A line
    /// that isn't six fields, its last two numbers, is left out.
    pub fn load(&mut self, text: &str) {
        for line in text.lines().skip(1) {
            let fields: Vec<_> = line.split('\t').collect();
            let [board, layer, held, key, presses, holds] = fields[..] else {
                continue;
            };
            let (Ok(presses), Ok(holds)) = (presses.parse::<u64>(), holds.parse::<u64>()) else {
                continue;
            };
            let count = self
                .counts
                .entry((board.into(), layer.into(), held.into(), key.into()))
                .or_default();
            count.presses += presses;
            count.holds += holds;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: u64 = 3;
    const LAPTOP: u64 = 5;
    const A: u16 = 30;
    const B: u16 = 48;
    const E: u16 = 18;
    const NINE: u16 = 10;
    const COMMA: u16 = 51;
    const KP_ONE: u16 = 79;
    const LEFT_SHIFT: u16 = 42;
    const RIGHT_CTRL: u16 = 97;
    const ESCAPE: u16 = 1;

    fn tap(tally: &mut Tally, device: u64, board: Board, code: u16) {
        tally.key(device, board, code, 1);
        tally.key(device, board, code, 0);
    }

    fn lines(tally: &Tally) -> Vec<String> {
        tally.render().lines().skip(1).map(str::to_owned).collect()
    }

    #[test]
    fn the_same_keys_in_any_order_keep_the_same_file() {
        let keys = [A, B, E, A, COMMA, ESCAPE, E, E];
        let mut forward = Tally::default();
        let mut backward = Tally::default();
        for code in keys {
            tap(&mut forward, BOARD, Board::Ergodox, code);
        }
        for code in keys.iter().rev() {
            tap(&mut backward, BOARD, Board::Ergodox, *code);
        }
        assert_eq!(forward.render(), backward.render());
    }

    #[test]
    fn the_file_holds_only_totals_by_key() {
        let mut tally = Tally::default();
        for code in [E, A, E, B, E] {
            tap(&mut tally, BOARD, Board::Ergodox, code);
        }
        assert_eq!(
            lines(&tally),
            [
                "ergodox\tcoal\t-\ta\t1\t0",
                "ergodox\tcoal\t-\tb\t1\t0",
                "ergodox\tcoal\t-\te\t3\t0"
            ]
        );
        for line in lines(&tally) {
            assert_eq!(line.split('\t').count(), 6, "{line}");
        }
    }

    #[test]
    fn a_key_held_until_it_repeats_counts_one_hold() {
        let mut tally = Tally::default();
        tally.key(BOARD, Board::Ergodox, RIGHT_CTRL, 1);
        for _ in 0..5 {
            tally.key(BOARD, Board::Ergodox, RIGHT_CTRL, 2);
        }
        tally.key(BOARD, Board::Ergodox, RIGHT_CTRL, 0);
        tap(&mut tally, BOARD, Board::Ergodox, RIGHT_CTRL);
        assert_eq!(lines(&tally), ["ergodox\tcoal\t-\trightctrl\t2\t1"]);
    }

    #[test]
    fn a_chord_counts_under_the_modifiers_held() {
        let mut tally = Tally::default();
        tally.key(BOARD, Board::Ergodox, LEFT_SHIFT, 1);
        tap(&mut tally, BOARD, Board::Ergodox, NINE);
        tally.key(BOARD, Board::Ergodox, LEFT_SHIFT, 0);
        tap(&mut tally, BOARD, Board::Ergodox, NINE);
        assert_eq!(
            lines(&tally),
            [
                "ergodox\tcoal\t-\tleftshift\t1\t0",
                "ergodox\tpunk\t-\t9\t1\t0",
                "ergodox\tpunk\tshift\t9\t1\t0",
            ]
        );
    }

    #[test]
    fn each_layer_is_told_by_its_keys() {
        let mut tally = Tally::default();
        for code in [A, COMMA, KP_ONE] {
            tap(&mut tally, BOARD, Board::Ergodox, code);
        }
        tap(&mut tally, LAPTOP, Board::Laptop, COMMA);
        assert_eq!(
            lines(&tally),
            [
                "ergodox\tcoal\t-\ta\t1\t0",
                "ergodox\tnumb\t-\tkp1\t1\t0",
                "ergodox\tpunk\t-\t,\t1\t0",
                "laptop\tbase\t-\t,\t1\t0",
            ]
        );
    }

    #[test]
    fn one_keyboards_modifiers_never_reach_another() {
        let mut tally = Tally::default();
        tally.key(LAPTOP, Board::Laptop, LEFT_SHIFT, 1);
        tap(&mut tally, BOARD, Board::Ergodox, A);
        assert!(lines(&tally).contains(&"ergodox\tcoal\t-\ta\t1\t0".to_owned()));
    }

    #[test]
    fn a_keyboard_that_goes_lets_go_of_what_it_held() {
        let mut tally = Tally::default();
        tally.key(BOARD, Board::Ergodox, LEFT_SHIFT, 1);
        tally.gone(BOARD);
        tap(&mut tally, BOARD, Board::Ergodox, A);
        assert!(lines(&tally).contains(&"ergodox\tcoal\t-\ta\t1\t0".to_owned()));
    }

    #[test]
    fn a_restart_carries_on_from_the_file() {
        let mut before = Tally::default();
        tap(&mut before, BOARD, Board::Ergodox, ESCAPE);
        let mut after = Tally::default();
        after.load(&before.render());
        after.load("board\tlayer\tmodifiers\tkey\tpresses\tholds\nnot a line\n");
        tap(&mut after, BOARD, Board::Ergodox, ESCAPE);
        assert_eq!(lines(&after), ["ergodox\tcoal\t-\tescape\t2\t0"]);
    }

    #[test]
    fn only_a_change_needs_writing() {
        let mut tally = Tally::default();
        assert!(!tally.take_changed());
        tap(&mut tally, BOARD, Board::Ergodox, A);
        assert!(tally.take_changed());
        assert!(!tally.take_changed());
    }
}
