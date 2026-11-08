//! Graphs in braille, ten characters wide, as the design sets every graph
//! (docs/design.md, a figure with a range): a spark for a history, filling
//! from the right with the newest last a step brighter, and a bar for a
//! level, filling from the left. On a terminal every cell the graph leaves
//! empty is drawn whole in the track's faint ink, so its ends show; off one
//! it's blank braille, which keeps the width.

use super::text::{Line, Role, run};

/// Every graph is this many characters, two readings to each.
pub const WIDE: usize = 10;
const SLOTS: usize = WIDE * 2;

/// A column of braille dots filled from the bottom, for the left and the
/// right of a cell.
const LEFT: [u32; 4] = [0x40, 0x04, 0x02, 0x01];
const RIGHT: [u32; 4] = [0x80, 0x20, 0x10, 0x08];
const FULL: char = '\u{28ff}';
const BLANK: char = '\u{2800}';

fn dots(left: usize, right: usize) -> char {
    let bits = LEFT[..left]
        .iter()
        .chain(&RIGHT[..right])
        .fold(0, |all, bit| all | bit);
    char::from_u32(0x2800 + bits).unwrap_or(BLANK)
}

fn track(coloured: bool) -> char {
    if coloured { FULL } else { BLANK }
}

/// How many dots high a reading stands, one to four, against the highest:
/// a reading of nothing still lies on the floor, so the graph's run shows.
fn height(value: f64, highest: f64) -> usize {
    if highest <= 0.0 || value <= 0.0 {
        return 1;
    }
    // A share of four rungs, so the cast holds a value from 0 to 3.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "0 to 3"
    )]
    let rung = ((value / highest).min(1.0) * 3.0).round() as usize;
    1 + rung
}

/// The newest readings of a history, against nothing at the bottom and its
/// highest at the top.
pub fn spark(values: &[f64], coloured: bool) -> Line {
    let shown = &values[values.len().saturating_sub(SLOTS)..];
    let highest = shown.iter().copied().fold(0.0, f64::max);
    let mut slots: Vec<Option<usize>> = vec![None; SLOTS - shown.len()];
    slots.extend(shown.iter().map(|value| Some(height(*value, highest))));
    let mut line = Vec::new();
    for (at, pair) in slots.chunks(2).enumerate() {
        let role = if at + 1 == WIDE {
            Role::Text
        } else {
            Role::Muted
        };
        match (pair[0], pair[1]) {
            (None, None) => line.push(run(track(coloured).to_string(), Role::Track)),
            (left, right) => {
                line.push(run(
                    dots(left.unwrap_or(0), right.unwrap_or(0)).to_string(),
                    role,
                ));
            }
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(line: &Line) -> String {
        line.iter().map(|run| run.text.as_str()).collect()
    }

    #[test]
    fn a_spark_fills_from_the_right_against_its_highest() {
        let line = spark(&[0.0, 1.0, 2.0, 4.0], false);
        assert_eq!(said(&line), "⠀⠀⠀⠀⠀⠀⠀⠀⣠⣾");
        assert_eq!(line[9].role, Role::Text, "the newest is a step brighter");
        assert_eq!(line[8].role, Role::Muted);
        assert_eq!(line[0].role, Role::Track);
    }

    #[test]
    fn a_spark_of_a_long_history_shows_its_newest_twenty() {
        let values: Vec<f64> = (0..25).map(f64::from).collect();
        let line = spark(&values, false);
        assert_eq!(said(&line).chars().count(), WIDE);
        assert_eq!(
            said(&line),
            spark(&values[5..], false)
                .iter()
                .map(|r| r.text.as_str())
                .collect::<String>()
        );
    }
}
