//! The machine in front of the table: what vi keeps between presses, a
//! count and a key waiting for its second, and what a press then does.

use super::Key::Char;
use super::{Act, Key, Slot, table};

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
    /// An operator's key and act, waiting for its motion.
    operator: Option<(Key, Act)>,
}

impl Machine {
    /// Takes one key, at rest or while selecting. Gives an act and its
    /// count once a sequence is whole; nothing while one waits for more,
    /// or when a key means nothing here, which drops whatever was in
    /// progress, as vi does.
    pub fn press(&mut self, key: Key, selecting: bool) -> Option<(Act, Option<u32>)> {
        if key == Key::Escape {
            let busy = self.waiting();
            self.reset();
            return if busy { None } else { Some((Act::Rest, None)) };
        }
        if let Char(digit @ '0'..='9') = key {
            // A 0 starts no count, as vi's 0 is its own motion: to the
            // line's start while selecting, and nothing at rest, where the
            // cursor is a whole line.
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
        if let Some(operator) = self.operator {
            return self.motion(operator, key);
        }
        if self.pending.is_empty()
            && let Some(act) = operator_of(key, selecting)
        {
            // While selecting, the selection is what it acts on.
            if selecting {
                return Some(self.finish(act));
            }
            self.operator = Some((key, act));
            return None;
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

    /// After an operator, a key of its motion: the motion whole gives the
    /// operator's act, its own key again the line's act the language has
    /// for the two, as d d is dd's, and a key that's no motion drops it, as
    /// vi does. A mark's motion takes its letter, as d'a does, and a text
    /// object is a motion, as in d i p, though the page has them only
    /// while selecting.
    fn motion(&mut self, (first, act): (Key, Act), key: Key) -> Option<(Act, Option<u32>)> {
        let every = || table().sequences.iter();
        if let [mark] = self.pending[..]
            && takes_a_letter(mark, false).is_some()
        {
            return if matches!(key, Char(letter) if letter.is_ascii_alphabetic()) {
                Some(self.finish(act))
            } else {
                self.reset();
                None
            };
        }
        if self.pending.is_empty() && takes_a_letter(key, false).is_some() {
            self.pending.push(key);
            return None;
        }
        self.pending.push(key);
        let line: Vec<Key> = std::iter::once(first)
            .chain(self.pending.iter().copied())
            .collect();
        if let Some(&(_, act, _)) = every().find(|(keys, _, _)| *keys == line) {
            return Some(self.finish(act));
        }
        if self.pending == [first] || every().any(|(keys, _, _)| *keys == self.pending) {
            return Some(self.finish(act));
        }
        let waits =
            every().any(|(keys, _, _)| keys.starts_with(&self.pending) || keys.starts_with(&line));
        if !waits {
            self.reset();
        }
        None
    }

    /// Whether a key is waiting for more, or a count is being typed.
    fn waiting(&self) -> bool {
        self.count.is_some() || !self.pending.is_empty() || self.operator.is_some()
    }

    /// Whether a key is waiting for its second, or a count is being typed.
    #[cfg(test)]
    pub fn busy(&self) -> bool {
        self.waiting()
    }

    fn finish(&mut self, act: Act) -> (Act, Option<u32>) {
        let count = self.count;
        self.reset();
        (act, count)
    }

    /// Takes a key the keyboard repeats while it's held. Only an act that
    /// moves goes on; a repeat neither starts a sequence nor ends one, so
    /// a held y copies once, from its press, and leaves nothing waiting.
    pub fn held(&mut self, key: Key, selecting: bool) -> Option<(Act, Option<u32>)> {
        if self.waiting() {
            return None;
        }
        let done = self.press(key, selecting).filter(|(act, _)| act.repeats());
        self.reset();
        done
    }

    fn reset(&mut self) {
        self.count = None;
        self.pending.clear();
        self.operator = None;
    }
}

/// An operator: these keys wait for a motion, where they work.
fn operator_of(key: Key, selecting: bool) -> Option<Act> {
    table()
        .operators
        .iter()
        .find(|&&(operator, _, held)| operator == key && held.holds(selecting))
        .map(|&(_, act, _)| act)
}
