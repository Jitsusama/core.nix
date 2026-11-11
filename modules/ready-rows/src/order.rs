//! Where each row a daemon serves stands, as a spaced `order` the shell
//! places it by and never sorts: a making's rows keep the orders they had
//! where they still rise, and only those that moved take a new one,
//! midway between their neighbours, so one row moving sends one row.
//! Every row is spaced again only when a gap runs out, and that goes as
//! one change, so the shell moves nothing twice.

/// How far apart each row's `order` is set, so one row moving between
/// two takes a number between theirs and the rest keep their own.
pub const SPACING: i64 = 1024;

/// The `order` each row takes, in the order shown, from the order each
/// had before (`was`): the most rows that can keep theirs do (the longest
/// run still rising), and the rest take numbers spaced between their
/// neighbours, so one row moving changes one row. Where no room is left
/// between two, every row is spaced again.
pub fn spaced(was: &[Option<i64>]) -> Vec<i64> {
    let kept = rising(was);
    let mut order = vec![0; was.len()];
    let mut at = 0;
    let mut below: Option<i64> = None;
    for upto in kept.iter().copied().chain(std::iter::once(was.len())) {
        let above = was.get(upto).copied().flatten();
        let between = upto - at;
        let count = i64::try_from(between).unwrap_or(i64::MAX);
        let room = match (below, above) {
            (Some(low), Some(high)) => {
                (high - low > count).then(|| (low, (high - low) / (count + 1)))
            }
            (None, Some(high)) => Some((high - SPACING * (count + 1), SPACING)),
            (Some(low), None) => Some((low, SPACING)),
            (None, None) => Some((0, SPACING)),
        };
        let Some((from, step)) = room else {
            return (1..=i64::try_from(was.len()).unwrap_or(i64::MAX))
                .map(|n| n * SPACING)
                .collect();
        };
        for (step_at, one) in (1..).zip(at..upto) {
            order[one] = from + step * step_at;
        }
        if let Some(high) = above {
            order[upto] = high;
            below = Some(high);
        }
        at = upto + 1;
    }
    order
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
    fn spacing_keeps_the_longest_rising_run_and_spaces_again_when_full() {
        assert_eq!(spaced(&[None, None]), [1024, 2048]);
        assert_eq!(
            spaced(&[Some(5), Some(3), Some(4), None]),
            [-1021, 3, 4, 1028]
        );
        assert_eq!(
            spaced(&[Some(1), None, Some(2)]),
            [1024, 2048, 3072],
            "no room between"
        );
    }

    #[test]
    fn a_moved_row_goes_midway_and_the_rest_keep_theirs() {
        // C, shown last at 3072, now shows between A and B.
        assert_eq!(
            spaced(&[Some(1024), Some(3072), Some(2048)]),
            [1024, 1536, 2048]
        );
        // A new row first, one last, and one gone from between.
        assert_eq!(
            spaced(&[None, Some(1024), Some(3072), None]),
            [0, 1024, 3072, 4096]
        );
    }
}
