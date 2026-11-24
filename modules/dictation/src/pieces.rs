//! How a take of theirs is cut for telling voices apart: in pieces of
//! `PIECE`, each embedded and grouped on its own (voices.rs), so a take
//! that holds two people can be split where the voice changes rather than
//! sit whole under one. On the three AMI meetings the voices and naming
//! checks measure, 3 s pieces put more of the far end's speech under the
//! right voice and the right name than whole takes did; at 2 s, TitaNet
//! heard one person as several. The checks score each piece under its
//! own voice; the daemon's lines, split from them where the voice
//! changes (call-transcripts' split.rs), are proved by that module's
//! cases, not by the checks' numbers.
//!
//! A piece grouped into no voice is most often someone's short
//! interjection inside another's take, but it's under the voice of the
//! nearest piece of its take given one, since leaving it under Others
//! left more of the speech unhandled than it named wrongly.

use crate::voices::RATE;

/// How long a piece of a take is.
pub const PIECE: usize = 3 * RATE;

/// Where each piece of a take of `samples` starts and ends: `PIECE` each,
/// the last taking in what's left when that's shorter than half of one,
/// so the last holds from 1.5 s to under 4.5 s, as the checks measured. A
/// take shorter than one piece and a half is one piece.
#[must_use]
pub fn pieces(samples: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    while samples - from >= PIECE + PIECE / 2 {
        out.push((from, from + PIECE));
        from += PIECE;
    }
    out.push((from, samples));
    out
}

/// Each piece's voice, from `voices`, where a piece given none takes the
/// voice of the nearest piece of its take given one, the earlier when two
/// are as near; `takes` says which take each piece is of, a take's pieces
/// next to each other in order.
#[must_use]
pub fn within_takes(takes: &[usize], voices: &[Option<usize>]) -> Vec<Option<usize>> {
    (0..voices.len())
        .map(|i| voices[i].or_else(|| nearest(takes, voices, i)))
        .collect()
}

/// The voice of the nearest piece to `i` of its take given one, and none
/// when no piece of it is.
fn nearest(takes: &[usize], voices: &[Option<usize>], i: usize) -> Option<usize> {
    let same = |j: &usize| takes[*j] == takes[i];
    let before = (0..i)
        .rev()
        .take_while(same)
        .find_map(|j| voices[j].map(|voice| (i - j, voice)));
    let after = (i + 1..voices.len())
        .take_while(same)
        .find_map(|j| voices[j].map(|voice| (j - i, voice)));
    match (before, after) {
        (Some(before), Some(after)) if after.0 < before.0 => Some(after.1),
        (before, after) => before.or(after).map(|(_, voice)| voice),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_take_is_cut_in_pieces_the_last_taking_in_a_short_remainder() {
        let second = RATE;
        assert_eq!(
            pieces(10 * second),
            [(0, PIECE), (PIECE, 2 * PIECE), (2 * PIECE, 10 * second)],
            "a 10 s take wasn't cut at 3 s and 6 s, the last 4 s whole"
        );
        assert_eq!(
            pieces(9 * second),
            [(0, PIECE), (PIECE, 2 * PIECE), (2 * PIECE, 9 * second)],
            "a 9 s take wasn't three pieces of 3 s"
        );
    }

    #[test]
    fn a_take_shorter_than_one_piece_and_a_half_is_whole() {
        assert_eq!(pieces(PIECE / 2), [(0, PIECE / 2)], "a short take was cut");
        assert_eq!(
            pieces(PIECE + PIECE / 2 - 1),
            [(0, PIECE + PIECE / 2 - 1)],
            "a take just short of one piece and a half was cut"
        );
        assert_eq!(
            pieces(PIECE + PIECE / 2),
            [(0, PIECE), (PIECE, PIECE + PIECE / 2)],
            "a take of one piece and a half wasn't cut"
        );
    }

    #[test]
    fn a_piece_given_no_voice_takes_its_nearest_voiced_neighbours_in_its_take() {
        // Take 0's second piece sits as near voice 1 as 2, and its last
        // has only voice 2 before it; take 1's first has only voice 0
        // after it; take 2's has none in its take, though take 1's last
        // is voiced.
        let takes = [0, 0, 0, 0, 1, 1, 2];
        let voices = [Some(1), None, Some(2), None, None, Some(0), None];
        assert_eq!(
            within_takes(&takes, &voices),
            [Some(1), Some(1), Some(2), Some(2), Some(0), Some(0), None],
            "a piece took a voice from another take, or from a further piece"
        );
    }

    #[test]
    fn a_piece_between_two_as_near_takes_the_earlier_voice() {
        let voices = within_takes(&[0, 0, 0], &[Some(3), None, Some(4)]);
        assert_eq!(voices[1], Some(3), "a piece between two took the later");
    }
}
