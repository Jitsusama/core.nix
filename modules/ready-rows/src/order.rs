//! Where each row a daemon serves stands, as a spaced `order` the shell
//! places it by and never sorts: a making's rows keep the orders they had
//! where they still rise, and only those that moved take a new one,
//! midway between their neighbours, so one row moving sends one row.
//! Where a gap runs out, only the rows nearest it are spaced again, as
//! many as give each its share of the spacing, so rows arriving again and
//! again in one place move their neighbours seldom and the rest never.

/// How far apart each row's `order` is set, so one row moving between
/// two takes a number between theirs and the rest keep their own.
pub const SPACING: i64 = 1024;

/// The `order` each row takes, in the order shown, from the order each
/// had before (`was`): the most rows that can keep theirs do (the longest
/// run still rising), and the rest take numbers spaced between their
/// neighbours, so one row moving changes one row. Where no room is left
/// between two, the rows around them are spaced again (see [`widened`]).
pub fn spaced(was: &[Option<i64>]) -> Vec<i64> {
    let mut kept = rising(was);
    loop {
        match placed(was, &kept) {
            Ok(order) => return order,
            Err(full) => widened(was, &mut kept, full),
        }
    }
}

/// Each row's order, keeping those of the rows at `kept` and spacing the
/// rest between them, or the place in `kept` of the row above the first
/// gap with no room for the rows that go in it.
fn placed(was: &[Option<i64>], kept: &[usize]) -> Result<Vec<i64>, usize> {
    let mut order = vec![0; was.len()];
    let mut at = 0;
    let mut below: Option<i64> = None;
    let bounds = kept.iter().copied().chain(std::iter::once(was.len()));
    for (anchor, upto) in bounds.enumerate() {
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
            return Err(anchor);
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
    Ok(order)
}

/// Lets go of the kept rows nearest a full gap, the one above it at
/// `full` in `kept` and the one below in turn, until the rows between
/// those still kept have half the spacing each, or one side reaches an
/// end. Spaced again that far, the gap takes many more rows before it
/// runs out, and nothing beyond it moves.
fn widened(was: &[Option<i64>], kept: &mut Vec<usize>, full: usize) {
    let order = |place: usize| was[place].unwrap_or_default();
    let (mut lo, mut hi) = (full, full);
    while let (Some(low), Some(high)) = (lo.checked_sub(1).map(|k| kept[k]), kept.get(hi)) {
        let rows = i64::try_from(high - low).unwrap_or(i64::MAX);
        if (order(*high) - order(low)) / rows >= SPACING / 2 {
            break;
        }
        if hi - full <= full - lo {
            hi += 1;
        } else {
            lo -= 1;
        }
    }
    kept.drain(lo..hi);
}

/// A name broken into runs of digits and of the rest, lowered, as the
/// launcher's Order.js reads names.
fn pieces(name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in name.to_lowercase().chars() {
        match out.last_mut() {
            Some(last) if last.starts_with(|d: char| d.is_ascii_digit()) == c.is_ascii_digit() => {
                last.push(c);
            }
            _ => out.push(c.to_string()),
        }
    }
    out
}

/// Two names A to Z as Order.js's `byName` reads them, so a daemon's
/// rows stand as the shell's own lists do: case alike, and a run of
/// digits by its worth, so "Tab 9" comes before "Tab 10".
#[must_use]
pub fn by_name(one: &str, other: &str) -> std::cmp::Ordering {
    let (a, b) = (pieces(one), pieces(other));
    for (x, y) in a.iter().zip(&b) {
        let digits = |p: &str| p.starts_with(|c: char| c.is_ascii_digit());
        let said = if digits(x) && digits(y) {
            let worth = |p: &str| p.parse::<f64>().unwrap_or(0.0);
            worth(x).total_cmp(&worth(y))
        } else {
            x.cmp(y)
        };
        if said.is_ne() {
            return said;
        }
    }
    a.len().cmp(&b.len())
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
    use std::collections::BTreeMap;

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
            [1, 1025, 2049],
            "no room between"
        );
    }

    #[test]
    fn names_go_a_to_z_with_case_alike_and_numbers_by_their_worth() {
        let mut names = ["tab 10", "Zathura", "Tab 9", "inkscape", "Tab"];
        names.sort_by(|one, other| by_name(one, other));
        assert_eq!(names, ["inkscape", "Tab", "Tab 9", "tab 10", "Zathura"]);
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

    #[test]
    fn streams_arriving_one_after_another_move_only_their_neighbour() {
        // A program's streams, each new one shown after the last before
        // that one goes, as a notification sound's do (2026-11-11): each
        // arrives midway between the last and the row after them.
        let mut shown: Vec<(String, i64)> = Vec::new();
        let mut moves: BTreeMap<String, usize> = BTreeMap::new();
        let names = |streams: &[usize]| {
            ["a", "b", "program"]
                .into_iter()
                .map(str::to_owned)
                .chain(streams.iter().map(|stream| format!("stream {stream}")))
                .chain(["next", "c", "d"].into_iter().map(str::to_owned))
                .collect::<Vec<_>>()
        };
        let arrivals = 200;
        let makings = (1..=arrivals).flat_map(|stream| [vec![stream - 1, stream], vec![stream]]);
        for streams in [vec![0]].into_iter().chain(makings) {
            let rows = names(&streams);
            let was: Vec<Option<i64>> = rows
                .iter()
                .map(|row| shown.iter().find(|(key, _)| key == row).map(|(_, at)| *at))
                .collect();
            let orders = spaced(&was);
            for ((row, before), after) in rows.iter().zip(&was).zip(&orders) {
                if before.is_some_and(|before| before != *after) {
                    *moves.entry(row.clone()).or_default() += 1;
                }
            }
            shown = rows.into_iter().zip(orders).collect();
        }
        let far: Vec<_> = moves
            .iter()
            .filter(|(row, _)| ["a", "b", "c", "d"].contains(&row.as_str()))
            .collect();
        assert!(far.is_empty(), "rows away from the streams moved: {far:?}");
        let next = moves.get("next").copied().unwrap_or(0);
        assert!(
            next * 8 <= arrivals,
            "the row after the streams moved {next} times in {arrivals} arrivals"
        );
    }
}
