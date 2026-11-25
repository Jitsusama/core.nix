//! What the kernel's input layer says a device takes, read alike for a
//! keyboard or mouse on Bluetooth and on anything else.

use crate::{Facts, fact};

/// The kernel's mouse buttons, `BTN_LEFT` to `BTN_TASK`.
pub(crate) const MOUSE_BUTTONS: std::ops::RangeInclusive<usize> = 0x110..=0x117;
/// A keyboard's keys, `KEY_ESC` to the last of the first page's.
pub(crate) const KEYBOARD_KEYS: std::ops::RangeInclusive<usize> = 0x01..=0xFF;
/// Fewer keys than this are a mouse's or a remote's, not a keyboard's.
pub(crate) const FEWEST_KEYS: usize = 20;
/// The wheels, as relative axes.
const REL_HWHEEL: usize = 0x06;
const REL_WHEEL: usize = 0x08;

/// The bits a capabilities file sets: hex words, the highest first.
pub(crate) fn set_bits(text: &str) -> Vec<usize> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut set = Vec::new();
    for (place, word) in words.iter().rev().enumerate() {
        let Ok(value) = u64::from_str_radix(word, 16) else {
            return Vec::new();
        };
        set.extend(
            (0..64)
                .filter(|bit| value >> bit & 1 == 1)
                .map(|bit| place * 64 + bit),
        );
    }
    set
}

/// What one device's inputs have said, and how it's carried, its id and
/// its release where they're known.
#[derive(Default)]
pub(crate) struct Heard {
    pub(crate) link: String,
    pub(crate) id: String,
    pub(crate) release: String,
    pub(crate) keys: Vec<usize>,
    pub(crate) axes: Vec<usize>,
}

impl Heard {
    pub(crate) fn facts(mut self) -> Facts {
        self.keys.sort_unstable();
        self.keys.dedup();
        let buttons = self
            .keys
            .iter()
            .filter(|key| MOUSE_BUTTONS.contains(key))
            .count();
        let keys = self
            .keys
            .iter()
            .filter(|key| KEYBOARD_KEYS.contains(key))
            .count();
        let wheel = match (
            self.axes.contains(&REL_WHEEL),
            self.axes.contains(&REL_HWHEEL),
        ) {
            (true, true) => "vertical and horizontal",
            (true, false) => "vertical",
            (false, true) => "horizontal",
            (false, false) => "",
        };
        let mut facts = Facts::new();
        fact(&mut facts, "connection", self.link);
        if buttons > 0 {
            fact(&mut facts, "buttons", buttons.to_string());
        }
        fact(&mut facts, "wheel", wheel);
        if keys >= FEWEST_KEYS {
            fact(&mut facts, "keys", keys.to_string());
        }
        fact(&mut facts, "id", self.id);
        fact(&mut facts, "release", self.release);
        facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capabilities_file_s_words_give_their_bits_the_highest_word_first() {
        assert_eq!(set_bits("903"), vec![0, 1, 8, 11]);
        assert_eq!(set_bits("1f0000 0 0 0 0"), vec![272, 273, 274, 275, 276]);
        assert_eq!(set_bits("0"), Vec::<usize>::new());
        assert_eq!(set_bits("not hex"), Vec::<usize>::new());
    }
}
