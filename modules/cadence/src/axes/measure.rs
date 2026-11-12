//! Readings from QCXZEB's daily measure, which counts what every pi turn
//! did and cost: the share of turns making one tool call, and the share of
//! cache writes spent rewriting what the cache lost. Each is read from the
//! newest measure as written rather than counted again, and stays unread
//! when its section is missing.

use super::{Bar, Reading, shown};

/// The share of the last day's turns that made exactly one tool call, from
/// the newest measure's "Turns Doing One Thing" table, whose first row is
/// the last day: `| ending | turns | one | none | several |`.
#[must_use]
pub fn one_call(measure: Option<&str>) -> Reading {
    let latest = measure.and_then(|text| {
        let (_, section) = text.split_once("## Turns Doing One Thing")?;
        let section = section.split("\n## ").next().unwrap_or_default();
        section
            .lines()
            .filter(|line| line.starts_with("| 20"))
            .find_map(|row| {
                let cells: Vec<f64> = row
                    .split('|')
                    .map(str::trim)
                    .skip(2)
                    .filter_map(|cell| cell.replace(',', "").parse().ok())
                    .collect();
                match cells.as_slice() {
                    [turns, one, ..] if *turns > 0.0 => Some((*one, *turns)),
                    _ => None,
                }
            })
    });
    Reading::new(
        "turns making one tool call {}",
        latest.map(|(one, turns)| one / turns * 100.0),
        "%",
        "QCXZEB",
    )
    .bar(Bar::AtMost(60.0))
    .detail(
        latest.map(|(one, turns)| {
            format!("{} of {} in the last day", shown(one, ""), shown(turns, ""))
        }),
    )
    .from("the measure's turns doing one thing")
}

/// The share of the day's cache writes that rewrote what the cache had
/// lost, from the measure's "Cache rewrites, by cause" list against the
/// "cache writes $X" its spend line gives. The first write after a
/// compaction is left out, since it writes new content rather than
/// rewriting old: QCXZEB, who owns spend, stands behind this as tokens'
/// one figure (2026-11-12), and aims it under 8% by 20 November. A share,
/// never dollars multiplied out, so it reads the same on a cheap day as
/// a dear one.
#[must_use]
pub fn rewrites(measure: Option<&str>) -> Reading {
    let read = measure.and_then(|text| {
        let writes = text
            .split_once("cache writes ")
            .and_then(|(_, rest)| money(rest))
            .filter(|w| *w > 0.0)?;
        let (_, list) = text.split_once("Cache rewrites, by cause:\n")?;
        let causes: Vec<(&str, &str, f64)> = list
            .lines()
            .map_while(|line| line.strip_prefix("- "))
            .filter_map(|line| {
                let (cause, rest) = line.split_once(": ")?;
                let (times, cost) = rest.split_once(", ")?;
                Some((cause, times, money(cost)?))
            })
            .filter(|(cause, ..)| *cause != "after compaction")
            .collect();
        let rewritten: f64 = causes.iter().map(|(.., cost)| cost).sum();
        let biggest = causes.iter().max_by(|a, b| a.2.total_cmp(&b.2));
        Some((
            rewritten / writes * 100.0,
            biggest.map(|(cause, times, _)| format!("mostly {cause}, {times}; ")),
        ))
    });
    Reading::new(
        "cache writes spent rewriting what the cache lost {}",
        read.as_ref().map(|(share, _)| *share),
        "%",
        "QCXZEB",
    )
    .bar(Bar::AtMost(8.0))
    .detail(read.map(|(_, biggest)| {
        format!(
            "{}the first write after a compaction is left out, since it writes new content",
            biggest.unwrap_or_default()
        )
    }))
    .from("the measure's cache rewrites by cause")
}

/// A dollar figure as the measure writes it, "$2,034" or "$9.82", with a
/// sentence's full stop after it left off.
fn money(text: &str) -> Option<f64> {
    let digits: String = text
        .trim_start_matches('$')
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .filter(|c| *c != ',')
        .collect();
    digits.trim_end_matches('.').parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_call_turns_are_the_last_days_share() {
        let measure = "# Measure\n\n## Turns Doing One Thing\n\n79 of 100 turns made one.\n\n\
            | Day ending | Turns | One call | None | Two or more |\n|---|---|---|---|---|\n\
            | 2026-11-12 | 1000 | 800 | 50 | 150 |\n| 2026-11-11 | 900 | 600 | 0 | 300 |\n\n\
            | Tool | One-call turns | Right after its own |\n|---|---|---|\n| bash | 400 | 200 |\n\n\
            ## Next\n";
        let reading = one_call(Some(measure));
        assert_eq!(
            reading.words(),
            "turns making one tool call 80%, at most 60%"
        );
        assert_eq!(one_call(Some("# Measure\n")).figure(), None);
        assert_eq!(one_call(None).figure(), None);
    }

    #[test]
    fn cache_rewrites_are_a_share_of_cache_writes_compaction_aside() {
        let measure = "Spend: $7,190 over 115422 turns. Cache reads $3,790, cache writes $2,034, \
                       output $1,364.

Cache rewrites, by cause:
- after compaction: 1152 times, $488
- idle over an hour: 149 times, $205
- partial miss, unexplained: 160 times, $20
- prompt changed: 85 times, $9.82
- after a typed message: 1 time, $1.12

Always-on context each turn reads, by repository (median session):
- world: 199k characters over 1 session, about $8.40 a day in reads.";
        let read = rewrites(Some(measure));
        assert_eq!(
            read.words(),
            "cache writes spent rewriting what the cache lost 12%, at most 8%"
        );
        assert!(read.out(), "12% is past the 8% bar");
        assert_eq!(
            read.detail.as_deref(),
            Some(
                "mostly idle over an hour, 149 times; the first write after a compaction \
                 is left out, since it writes new content"
            ),
            "the detail names the biggest cause by what it cost"
        );
        assert_eq!(rewrites(None).figure(), None);
        assert_eq!(
            rewrites(Some("Spend: $1 over 2 turns.")).figure(),
            None,
            "a measure without the section is unread, not 0%"
        );
    }
}
