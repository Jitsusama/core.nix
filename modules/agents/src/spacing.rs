//! The `order` each of a place's rows takes, spaced so a row moving
//! between two others changes that row alone, and a full gap widens over
//! the rows beside it rather than numbering every row again.

/// How far apart each row's `order` is set, so one row moving between
/// two takes a number between theirs and the rest keep their own.
pub const SPACING: i64 = 1024;
/// The furthest from naught an `order` is set, the largest whole number
/// the launcher's JavaScript holds exactly, as it reads them.
const MOST: i128 = (1 << 53) - 1;

/// The `order` each row takes, in the order shown, from the order each
/// had before (`was`): the most rows that can keep theirs do (the longest
/// run still rising), and the rest take numbers spaced between their
/// neighbours, so one row moving changes one row. Where no room is left
/// between two, the gap widens over the kept rows nearest it, below and
/// above in turn, until it has room, so only those rows move with it:
/// spacing every row again told all of them whole each time one gap
/// filled, 460 rows and 405 KB about every 100 s (agents-top,
/// 2026-11-18). Every row is spaced again only once no kept row is left
/// to widen over, as past the furthest order the launcher reads exactly
/// (`MOST`).
pub fn spaced(was: &[Option<i64>]) -> Vec<i64> {
    let mut kept = rising(was);
    let mut order = vec![0; was.len()];
    let held = |place: usize| was[place].unwrap_or(0);
    // The gap before `kept[gap]`, or past the last kept row.
    let mut gap = 0;
    let mut below_next = true;
    while gap <= kept.len() {
        let low = gap.checked_sub(1).map(|one| kept[one]);
        let high = kept.get(gap).copied();
        let at = low.map_or(0, |one| one + 1);
        let upto = high.unwrap_or(was.len());
        let count = i128::try_from(upto - at).unwrap_or(i128::MAX);
        let Some((from, step)) = room(low.map(held), high.map(held), count) else {
            // A full gap has a kept row on at least one side, since one
            // with none has room for any count.
            if low.is_some() && (below_next || high.is_none()) {
                kept.remove(gap - 1);
                gap -= 1;
            } else {
                kept.remove(gap);
            }
            below_next = !below_next;
            continue;
        };
        for (step_at, one) in (1..).zip(at..upto) {
            // It lies between the orders below and above, so it fits.
            order[one] = i64::try_from(from + step * step_at).unwrap_or(i64::MAX);
        }
        if let Some(high) = high {
            order[high] = held(high);
        }
        gap += 1;
        below_next = true;
    }
    order
}

/// Where `count` rows go between the orders below and above them, if
/// they fit: the order they count up from, and the step between them,
/// reckoned wide enough that neither can overflow.
fn room(low: Option<i64>, high: Option<i64>, count: i128) -> Option<(i128, i128)> {
    let spacing = i128::from(SPACING);
    match (low.map(i128::from), high.map(i128::from)) {
        (Some(low), Some(high)) => {
            let step = (high - low) / (count + 1);
            (step > 0).then_some((low, step))
        }
        (None, Some(high)) => {
            let from = high - spacing * (count + 1);
            (from + spacing >= -MOST).then_some((from, spacing))
        }
        (Some(low), None) => (low + spacing * count <= MOST).then_some((low, spacing)),
        (None, None) => Some((0, spacing)),
    }
}

/// The places of the longest run of orders still rising, among those
/// that had one.
fn rising(was: &[Option<i64>]) -> Vec<usize> {
    // Patience: `tails[n]` is the place ending the best run of n + 1.
    let mut tails: Vec<usize> = Vec::new();
    let mut before = vec![None; was.len()];
    for (place, order) in was.iter().enumerate() {
        let Some(order) = order else { continue };
        let at = tails.partition_point(|tail| was[*tail].is_some_and(|tail| tail < *order));
        before[place] = at.checked_sub(1).map(|one| tails[one]);
        if at == tails.len() {
            tails.push(place);
        } else {
            tails[at] = place;
        }
    }
    let mut run = Vec::new();
    let mut place = tails.last().copied();
    while let Some(one) = place {
        run.push(one);
        place = before[one];
    }
    run.reverse();
    run
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_keeps_the_longest_rising_run_and_widens_a_full_gap() {
        assert_eq!(spaced(&[None, None]), [1024, 2048]);
        assert_eq!(
            spaced(&[Some(5), Some(3), Some(4), None]),
            [-1021, 3, 4, 1028]
        );
        assert_eq!(
            spaced(&[Some(1), None, Some(2)]),
            [-2046, -1022, 2],
            "no room between, so the row below lets go of its order to make some"
        );
        assert_eq!(
            spaced(&[Some(1), None, Some(3)]),
            [1, 2, 3],
            "a step of one is room enough"
        );
    }

    #[test]
    fn a_full_gap_moves_only_the_rows_it_widens_over() {
        // Every row whole was told again each time one gap filled, about
        // every 100 s on Joel's laptop: 460 rows, 405 KB, for one row placed
        // (agents-top, 2026-11-18).
        assert_eq!(
            spaced(&[
                Some(1024),
                Some(2048),
                None,
                Some(2049),
                Some(4096),
                Some(5120)
            ]),
            [1024, 1365, 1706, 2049, 4096, 5120],
            "only the row below the full gap moves with the one placed"
        );
        assert_eq!(
            spaced(&[Some(1024), Some(2048), None, None, Some(2050), Some(4096)]),
            [1024, 1280, 1536, 1792, 2050, 4096],
            "two placed in a full gap move only the row below it too"
        );
        assert_eq!(
            spaced(&[Some(1024), None, Some(2048), None, Some(2049), Some(4096)]),
            [1024, 1280, 1536, 1792, 2049, 4096],
            "the gap below the row let go is spaced again with it"
        );
    }

    #[test]
    fn a_gap_still_full_once_widened_below_widens_above() {
        assert_eq!(
            spaced(&[
                Some(1024),
                Some(2048),
                Some(2049),
                None,
                Some(2050),
                Some(4096)
            ]),
            [1024, 2048, 2560, 3072, 3584, 4096],
            "below, then above, then the row below that stays"
        );
        assert_eq!(
            spaced(&[
                Some(1024),
                Some(2046),
                Some(2047),
                Some(2048),
                None,
                Some(2049),
                Some(2050),
                Some(4096)
            ]),
            [1024, 2046, 2387, 2728, 3069, 3410, 3751, 4096],
            "below, above, below again, then above again"
        );
        let most = i64::try_from(MOST).unwrap_or(i64::MAX);
        assert_eq!(
            spaced(&[None, Some(1 - most), Some(5)]),
            [-2043, -1019, 5],
            "with no row below, the one above is let go"
        );
    }

    #[test]
    fn spacing_again_is_left_for_when_no_kept_row_is_left() {
        let most = i64::try_from(MOST).unwrap_or(i64::MAX);
        assert_eq!(
            spaced(&[Some(1), Some(most - 1), None]),
            [1, 1025, 2049],
            "past the furthest order, the row below widens it first"
        );
        assert_eq!(
            spaced(&[Some(most - 1), None]),
            [1024, 2048],
            "and with no row left to let go of, every row is spaced again"
        );
    }

    #[test]
    fn every_order_rises_in_the_order_shown_and_stays_where_it_reads_exactly() {
        let most = i64::try_from(MOST).unwrap_or(i64::MAX);
        let mut next = draws();
        for _ in 0..2000 {
            // Orders drawn from a narrow range, so gaps fill, some near
            // either end of what's read exactly or of an i64, and some none.
            let was: Vec<Option<i64>> = (0..next(40))
                .map(|_| {
                    let near = i64::try_from(next(4096)).unwrap_or(0);
                    match next(10) {
                        0 => None,
                        1 => Some(most - near),
                        2 => Some(near - most),
                        3 => Some(i64::MAX - near),
                        4 => Some(i64::MIN + near),
                        _ => Some(near % 64),
                    }
                })
                .collect();
            let order = spaced(&was);
            assert_eq!(order.len(), was.len(), "{was:?} gave {order:?}");
            assert!(
                order.windows(2).all(|two| two[0] < two[1]),
                "{was:?} gave {order:?}, not rising"
            );
            assert!(
                order.iter().all(|one| i128::from(*one).abs() <= MOST),
                "{was:?} gave {order:?}, past what's read exactly"
            );
            // Still rising and in bounds, so every row keeps it: shifted
            // by its place, so no fixed numbering passes by chance.
            let again: Vec<i64> = order.iter().zip(0..).map(|(one, at)| one + at).collect();
            if again.iter().all(|one| i128::from(*one).abs() <= MOST) {
                let kept: Vec<Option<i64>> = again.iter().copied().map(Some).collect();
                assert_eq!(
                    spaced(&kept),
                    again,
                    "{was:?} gave {order:?}, which every row can keep"
                );
            }
        }
    }

    #[test]
    fn rows_placed_again_and_again_in_one_spot_move_few_others() {
        // As sessions turning come to the top: renumbering every row each
        // time a gap there filled told all 460 again (agents-top,
        // 2026-11-18). In the middle too, with more rows on either
        // side, so moving a whole side would show.
        for first in [1, 16] {
            placed_again_and_again_from(first);
        }
    }

    fn placed_again_and_again_from(first: usize) {
        let mut next = draws();
        let mut order: Vec<i64> = (1..=30).map(|n| n * SPACING).collect();
        let mut all_moved = 0;
        for placed in 0..300 {
            let at = usize::try_from(next(3)).unwrap_or(0) + first;
            let mut was: Vec<Option<i64>> = order.iter().copied().map(Some).collect();
            was.insert(at, None);
            let now = spaced(&was);
            let moved = was
                .iter()
                .zip(&now)
                .filter(|(was, now)| was.is_some_and(|was| was != **now))
                .count();
            assert!(
                moved <= 12,
                "placing {placed} at {at} moved {moved} rows: {was:?} gave {now:?}"
            );
            all_moved += moved;
            order = now;
        }
        // Two a placement on the whole: renumbering moved over a hundred.
        assert!(
            all_moved <= 600,
            "300 placements from {first} moved {all_moved} rows"
        );
    }

    #[test]
    fn each_full_gap_widens_below_first() {
        assert_eq!(
            spaced(&[
                Some(1024),
                Some(2048),
                None,
                Some(2049),
                Some(4096),
                Some(5120),
                None,
                Some(5121),
                Some(8192)
            ]),
            [1024, 1365, 1706, 2049, 4096, 4437, 4778, 5121, 8192],
            "the second full gap lets go of the row below it, as the first did"
        );
    }

    #[test]
    fn orders_are_reckoned_past_an_i64_and_bounded_at_what_is_read_exactly() {
        assert_eq!(
            spaced(&[Some(i64::MIN), None, Some(i64::MAX)]),
            [1024, 2048, 3072],
            "orders past what's read exactly are let go, reckoned without overflow"
        );
        let most = i64::try_from(MOST).unwrap_or(i64::MAX);
        assert_eq!(
            spaced(&[Some(-most), None, Some(i64::MAX)]),
            [-most, SPACING - most, 2 * SPACING - most],
            "a span between two kept rows past an i64's reach is reckoned without overflow"
        );
        assert_eq!(
            spaced(&[Some(most - SPACING), None]),
            [most - SPACING, most],
            "the furthest order is taken"
        );
        assert_eq!(
            spaced(&[Some(most - SPACING + 1), None]),
            [1024, 2048],
            "one past it isn't"
        );
        assert_eq!(
            spaced(&[None, Some(SPACING - most)]),
            [-most, SPACING - most],
            "the furthest below is taken"
        );
        assert_eq!(
            spaced(&[None, Some(SPACING - most - 1)]),
            [1024, 2048],
            "one past it below isn't"
        );
    }

    /// Draws below a bound, the same each run (xorshift).
    fn draws() -> impl FnMut(u64) -> u64 {
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        move |below| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % below
        }
    }
}
