//! How each measure reads on the hour's card: its mark, figure and target,
//! and, for one past its target, the first few things behind the figure,
//! or every cell or quest behind a cell measure, so whoever steers the
//! work reads which merges, checks, dates or quests the count is made of
//! without opening the files it came from.

use super::measures::{Class, Measure, queues, rounds, spent};

/// How many of the things behind a figure the card names before it
/// counts the rest.
const NAMED: usize = 3;

/// A figure as the card prints it: whole when it is, else to a tenth.
pub fn figure(x: f64) -> String {
    if (x - x.round()).abs() < 0.05 {
        format!("{x:.0}")
    } else {
        format!("{x:.1}")
    }
}

/// One measure's line: its mark, what it says, its figure and target. An
/// unjudged queue has no figure, quiet or unread, so it reads its words
/// alone, which say why.
fn line(m: &Measure) -> String {
    match m.target {
        None if m.key.starts_with(queues::QUEUE) => format!("· {}", m.says),
        None => format!("· {}: {}, unjudged", m.says, figure(m.now)),
        Some(target) => {
            let mark = if m.past().is_some() { "✕" } else { "✓" };
            format!(
                "{mark} {}: {}, at most {}",
                m.says,
                figure(m.now),
                figure(target)
            )
        }
    }
}

/// The first few things behind a measure past its target, on a line of
/// their own under it, the rest counted. Joel's misses are listed in full
/// further down, and a clean zone's files go to their owners, so neither
/// is named here. A cell measure's lines are each a cell or quest its
/// owner must take up or park, so each is named on its own line, grouped
/// by owner, for the orchestrator to send on without reading the mesh.
/// The spend measure's lines each end with a quest instead, in its own
/// order, and are named though it is never judged, since they are what
/// it is for. The merges past five rounds are named whatever the median
/// reads, each with what its rounds past five caught, as WCJQSG asked.
fn behind(m: &Measure) -> Vec<String> {
    if m.key == spent::KEY {
        return m.behind.iter().map(|line| format!("  {line}")).collect();
    }
    let named_within = m.key == rounds::KEY;
    if matches!(m.class, Class::Miss | Class::Clean)
        || (m.past().is_none() && !named_within)
        || m.behind.is_empty()
    {
        return Vec::new();
    }
    if m.class == Class::Cells {
        let mut lines: Vec<&String> = m.behind.iter().collect();
        // Sorting is stable, so each owner's keep the order they were read in.
        lines.sort_by_key(|line| line.rsplit_once(" · ").map_or("", |(_, owner)| owner));
        return lines.into_iter().map(|line| format!("  {line}")).collect();
    }
    let mut named: Vec<String> = m.behind.iter().take(NAMED).cloned().collect();
    let rest = m.behind.len().saturating_sub(NAMED);
    if rest > 0 {
        named.push(format!("and {rest} more"));
    }
    vec![format!("  {}", named.join("; "))]
}

/// A class's lines on the card. Clean zones read some sixty measures, so
/// only those past their target are listed, and the rest are counted in a
/// line of their own; every other class lists each of its measures, with
/// what's behind one past its target. A merge whose rounds past five caught
/// nothing is already named under the rounds median, so its own measure,
/// kept only to be filed to its owner, isn't listed again.
pub fn shown_lines(class: Class, of_class: &[&Measure]) -> Vec<String> {
    if class != Class::Clean {
        return of_class
            .iter()
            .filter(|m| !m.key.starts_with(rounds::FRUITLESS))
            .flat_map(|m| std::iter::once(line(m)).chain(behind(m)))
            .collect();
    }
    let mut lines: Vec<String> = of_class
        .iter()
        .filter(|m| m.past().is_some())
        .map(|m| line(m))
        .collect();
    let unjudged = of_class.iter().filter(|m| m.target.is_none()).count();
    let within = of_class.len() - lines.len() - unjudged;
    let mut rest = Vec::new();
    if within > 0 {
        rest.push(if lines.is_empty() {
            format!("all {within} within target")
        } else {
            format!("{within} more within target")
        });
    }
    if unjudged > 0 {
        rest.push(format!("{unjudged} unjudged"));
    }
    if !rest.is_empty() {
        lines.push(rest.join(", "));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quiet_or_unread_queue_reads_its_words_with_no_figure() {
        let read = queues::Records {
            outcomes: Some(""),
            sends: None,
            landed: Some(""),
            aboard: Some(&[]),
        };
        let measures = queues::queues(&read, 1_795_262_400);
        let shown = shown_lines(Class::Queues, &measures.iter().collect::<Vec<_>>());
        assert_eq!(
            shown,
            [
                "· seconds waiting for a World git turn: none through in a day",
                "· seconds waiting for a build on this machine: none through in a day",
                "· p90 seconds waiting for a VM run's builder (its record unread)",
                "· seconds waiting for core's merge train, aboard: none through in a day",
            ],
            "a queue with no figure read as a measured 0"
        );
    }

    fn measure(class: Class, now: f64, behind: &[&str]) -> Measure {
        Measure {
            key: "reverts".to_owned(),
            class,
            says: "merges the net reverted".to_owned(),
            now,
            target: Some(0.0),
            owner: "OWNER".to_owned(),
            behind: behind.iter().map(|b| (*b).to_owned()).collect(),
        }
    }

    #[test]
    fn a_figure_past_its_target_names_what_it_is_made_of() {
        let five = measure(
            Class::Functional,
            5.0,
            &["#4426", "#4430", "#4431", "#4440", "#4452"],
        );
        assert_eq!(
            shown_lines(Class::Functional, &[&five]),
            [
                "✕ merges the net reverted: 5, at most 0",
                "  #4426; #4430; #4431; and 2 more",
            ],
            "the merges behind the count weren't named, or the rest weren't counted"
        );
        let two = measure(Class::Slip, 2.0, &["the pager · XY9H5C", "a lock · 16V6F4"]);
        assert_eq!(
            shown_lines(Class::Slip, &[&two])[1],
            "  the pager · XY9H5C; a lock · 16V6F4",
            "every one of a few wasn't named"
        );
    }

    #[test]
    fn rounds_per_merge_names_its_merges_past_five_though_within_target() {
        let mut rounds = measure(Class::Ship, 2.0, &["core.nix#12: 6 rounds"]);
        rounds.key = rounds::KEY.to_owned();
        rounds.target = Some(rounds::TARGET);
        assert_eq!(
            shown_lines(Class::Ship, &[&rounds])[1],
            "  core.nix#12: 6 rounds"
        );
        let mut other = measure(Class::Ship, 2.0, &["x"]);
        other.target = None;
        assert_eq!(
            shown_lines(Class::Ship, &[&other]).len(),
            1,
            "others stay bare"
        );
        let mut fruitless = measure(Class::Ship, 6.0, &[]);
        fruitless.key = format!("{}core.nix#12", rounds::FRUITLESS);
        fruitless.target = Some(5.0);
        assert_eq!(
            shown_lines(Class::Ship, &[&rounds, &fruitless]).len(),
            2,
            "a merge named under the median was listed again on its own line"
        );
    }

    #[test]
    fn a_cell_measure_names_every_quest_behind_it_by_owner_one_a_line() {
        let ended = measure(
            Class::Cells,
            5.0,
            &[
                "AAAAAA driving, every cell ended · WM4KSZ",
                "BBBBBB active, every cell ended · 1GB1PI",
                "CCCCCC driving, every cell ended · WM4KSZ",
                "DDDDDD active, every cell ended · 1GB1PI",
                "EEEEEE active, every cell ended · IHHRPH",
            ],
        );
        assert_eq!(
            shown_lines(Class::Cells, &[&ended])[1..],
            [
                "  BBBBBB active, every cell ended · 1GB1PI",
                "  DDDDDD active, every cell ended · 1GB1PI",
                "  EEEEEE active, every cell ended · IHHRPH",
                "  AAAAAA driving, every cell ended · WM4KSZ",
                "  CCCCCC driving, every cell ended · WM4KSZ",
            ],
            "a cell measure cut its quests short, or didn't group them by owner"
        );
    }

    #[test]
    fn the_spend_measure_names_its_quests_unjudged_in_its_own_order() {
        let spent = Measure {
            key: spent::KEY.to_owned(),
            target: None,
            ..measure(
                Class::Cells,
                1.0,
                &[
                    "$206 today, landed nothing · ZZZZZZ",
                    "$90 today, landed a#1 · AAAAAA",
                ],
            )
        };
        assert_eq!(
            shown_lines(Class::Cells, &[&spent])[1..],
            [
                "  $206 today, landed nothing · ZZZZZZ",
                "  $90 today, landed a#1 · AAAAAA"
            ],
            "the spend measure hid its quests, or sorted them as if each named an owner"
        );
        let unread = Measure {
            target: None,
            ..measure(Class::Cells, 1.0, &["AAAAAA idle · WM4KSZ"])
        };
        assert_eq!(
            shown_lines(Class::Cells, &[&unread]).len(),
            1,
            "a cell measure left unjudged, its read untrusted, named what's behind it"
        );
    }

    #[test]
    fn a_figure_in_or_a_miss_names_nothing_behind_it() {
        let within = Measure {
            target: Some(9.0),
            ..measure(Class::Ship, 5.0, &["#4534 waiting 2 min"])
        };
        assert_eq!(
            shown_lines(Class::Ship, &[&within]).len(),
            1,
            "a figure within its target listed what's behind it"
        );
        let miss = measure(Class::Miss, 1.0, &["the cat waits · XY9H5C"]);
        assert_eq!(
            shown_lines(Class::Miss, &[&miss]).len(),
            1,
            "a miss was named twice, here and in the card's own list of misses"
        );
    }
}
