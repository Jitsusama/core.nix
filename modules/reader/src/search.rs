//! What / and ? do, kept apart from the window so it can be tested: the
//! words typed after them, as Neovim's command line takes them, and which
//! match a search goes to.
use crate::keys::Key;
use crate::text::Found;

/// What a key did to the words being typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typed {
    /// They changed, so the matches are found again.
    Changed,
    /// Return: the search is made.
    Done,
    /// Escape, or Backspace with nothing left: back to where it started.
    Cancelled,
    /// A key the command line takes no notice of.
    Nothing,
}

/// A search being typed.
pub struct Typing {
    pub forward: bool,
    pub typed: String,
    /// Where the view's top was when / or ? was pressed, in points.
    pub from_pt: f64,
}

impl Typing {
    pub fn new(forward: bool, from_pt: f64) -> Self {
        Typing {
            forward,
            typed: String::new(),
            from_pt,
        }
    }

    /// Takes a key as Neovim's command line does: Ctrl+U clears it, Ctrl+W
    /// takes the last word, Ctrl+H is Backspace.
    pub fn key(&mut self, key: Key) -> Typed {
        match key {
            Key::Escape | Key::Ctrl('c') => Typed::Cancelled,
            Key::Return => Typed::Done,
            Key::Backspace | Key::Ctrl('h') => {
                if self.typed.pop().is_some() {
                    Typed::Changed
                } else {
                    Typed::Cancelled
                }
            }
            Key::Ctrl('u') => {
                self.typed.clear();
                Typed::Changed
            }
            Key::Ctrl('w') => {
                let kept = self
                    .typed
                    .trim_end()
                    .trim_end_matches(|c: char| !c.is_whitespace())
                    .len();
                self.typed.truncate(kept);
                Typed::Changed
            }
            Key::Char(c) if !c.is_control() => {
                self.typed.push(c);
                Typed::Changed
            }
            _ => Typed::Nothing,
        }
    }
}

/// Where a match's top sits, in points down the page.
pub fn top_pt(found: &Found) -> f64 {
    found
        .pieces
        .first()
        .map_or(0.0, |piece| piece.baseline_pt - piece.size_pt)
}

/// Where a match's foot sits, its descenders' room in, in points.
pub fn foot_pt(found: &Found) -> f64 {
    found
        .pieces
        .last()
        .map_or(0.0, |piece| piece.baseline_pt + piece.size_pt * 0.3)
}

/// The match a search starting at `at_pt` goes to: forward, the first at or
/// below it; back, the last above it; round the page's end either way, as
/// Neovim's wrapscan.
pub fn from(found: &[Found], at_pt: f64, forward: bool) -> Option<usize> {
    if found.is_empty() {
        return None;
    }
    if forward {
        Some(found.iter().position(|f| top_pt(f) >= at_pt).unwrap_or(0))
    } else {
        Some(
            found
                .iter()
                .rposition(|f| top_pt(f) < at_pt)
                .unwrap_or(found.len() - 1),
        )
    }
}

/// The match after `current` in a search's direction, or before it against
/// it, round the page's end.
pub fn step(count: usize, current: usize, forward: bool) -> usize {
    if forward {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Piece;

    fn found(baselines: &[f64]) -> Vec<Found> {
        baselines
            .iter()
            .map(|&baseline_pt| Found {
                pieces: vec![Piece {
                    baseline_pt,
                    size_pt: 10.0,
                    from_pt: 0.0,
                    to_pt: 5.0,
                }],
            })
            .collect()
    }

    fn typing(keys: &[Key]) -> (Typing, Vec<Typed>) {
        let mut typing = Typing::new(true, 0.0);
        let said = keys.iter().map(|&key| typing.key(key)).collect();
        (typing, said)
    }

    #[test]
    fn typing_builds_the_words_and_backspace_takes_them() {
        let (typing, said) = typing(&[
            Key::Char('c'),
            Key::Char('a'),
            Key::Char('t'),
            Key::Backspace,
        ]);
        assert_eq!(typing.typed, "ca");
        assert_eq!(said.last(), Some(&Typed::Changed));
    }

    #[test]
    fn backspace_on_nothing_cancels_as_neovims_does() {
        let (_, said) = typing(&[Key::Char('a'), Key::Backspace, Key::Backspace]);
        assert_eq!(said, vec![Typed::Changed, Typed::Changed, Typed::Cancelled]);
    }

    #[test]
    fn ctrl_w_takes_the_last_word_and_ctrl_u_the_line() {
        let (mut typing, _) = typing(&[
            Key::Char('o'),
            Key::Char('n'),
            Key::Char('e'),
            Key::Char(' '),
            Key::Char('t'),
        ]);
        typing.key(Key::Ctrl('w'));
        assert_eq!(typing.typed, "one ");
        typing.key(Key::Ctrl('u'));
        assert_eq!(typing.typed, "");
    }

    #[test]
    fn escape_cancels_and_return_searches() {
        assert_eq!(typing(&[Key::Escape]).1, vec![Typed::Cancelled]);
        assert_eq!(typing(&[Key::Return]).1, vec![Typed::Done]);
        assert_eq!(typing(&[Key::Down]).1, vec![Typed::Nothing]);
    }

    #[test]
    fn a_search_goes_to_the_nearest_match_its_way_and_wraps() {
        let found = found(&[20.0, 120.0, 220.0]);
        assert_eq!(from(&found, 100.0, true), Some(1));
        assert_eq!(from(&found, 300.0, true), Some(0));
        assert_eq!(from(&found, 100.0, false), Some(0));
        assert_eq!(from(&found, 0.0, false), Some(2));
        assert_eq!(from(&[], 0.0, true), None);
    }

    #[test]
    fn n_and_shift_n_step_round_the_page() {
        assert_eq!(step(3, 2, true), 0);
        assert_eq!(step(3, 0, false), 2);
        assert_eq!(step(3, 1, true), 2);
    }
}
