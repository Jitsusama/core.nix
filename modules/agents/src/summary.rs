//! How the sessions stand together, as Home's Agents row and the row
//! found by name both say it (VKCF3Z, 2026-11-07).

use crate::said::Said;
use crate::standing::Stands;

/// How the sessions stand together, from how each stands and what they
/// all spent today as spend-watch sums it (`spent`, "" while it's unsaid
/// or the screen is shared): those needing Joel first, as what waits on
/// him comes first everywhere it's said, then those working, then the
/// day's dollars, "1 needs you · 3 working · $214 today", the answers
/// to his talks he hasn't read (`answers`) just after those needing him,
/// "1 needs you · 1 answer · 3 working" (Joel, 2026-11-11). With none
/// needing him nor working, it names how the rest stand in the fold's
/// words and order, "3 listening, 1 paused", the parked not among them,
/// and nothing for what isn't there.
pub fn summary(states: &[Stands], answers: usize, spent: &str, said: &Said) -> String {
    let count = |stands: Stands| states.iter().filter(|one| **one == stands).count();
    let filled = |key: &str, many: usize| said.fill(key, &[("count", &many.to_string())]);
    let (needing, working) = (count(Stands::Needs), count(Stands::Working));
    let rest = if needing == 0 && working == 0 {
        let parts: Vec<_> = [Stands::InLine, Stands::Listening, Stands::Paused]
            .into_iter()
            .filter(|stands| count(*stands) > 0)
            .map(|stands| match stands {
                Stands::InLine => filled("inLineCount", count(stands)),
                _ => format!("{} {}", count(stands), said.word(stands.id())),
            })
            .collect();
        parts.join(", ")
    } else {
        String::new()
    };
    let parts = [
        match needing {
            0 => String::new(),
            1 => filled("needsCount", 1),
            _ => filled("needCount", needing),
        },
        match answers {
            0 => String::new(),
            1 => filled("answerCount", 1),
            _ => filled("answersCount", answers),
        },
        if working > 0 {
            filled("workingCount", working)
        } else {
            String::new()
        },
        rest,
        if spent.is_empty() {
            String::new()
        } else {
            said.fill("today", &[("spent", spent)])
        },
    ];
    let spoken: Vec<_> = parts.into_iter().filter(|part| !part.is_empty()).collect();
    spoken.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standing::RUNNING;

    #[test]
    fn together_they_say_who_needs_joel_who_works_and_the_days_dollars() {
        let said =
            Said::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read");
        let cases = [
            (
                "working working listening needs working",
                "$214",
                "1 needs you · 3 working · $214 today",
            ),
            ("needs needs paused", "", "2 need you"),
            (
                "paused listening parked listening listening",
                "$3.10",
                "3 listening, 1 paused · $3.10 today",
            ),
            ("listening inLine", "", "1 in line, 1 listening"),
            ("parked", "", ""),
            ("", "$0.40", "$0.40 today"),
            ("", "", ""),
        ];
        for (states, spent, want) in cases {
            let stands: Vec<Stands> = states
                .split_whitespace()
                .map(|id| {
                    *RUNNING
                        .iter()
                        .find(|one| one.id() == id)
                        .expect("a known state")
                })
                .collect();
            assert_eq!(
                summary(&stands, 0, spent, &said),
                want,
                "together, {states:?}"
            );
        }
    }

    #[test]
    fn the_answers_joel_hasnt_read_come_just_after_who_needs_him() {
        let said =
            Said::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read");
        let cases = [
            (
                &[Stands::Needs, Stands::Working][..],
                1,
                "1 needs you · 1 answer · 1 working",
            ),
            (
                &[Stands::Listening, Stands::Listening][..],
                2,
                "2 answers · 2 listening",
            ),
        ];
        for (stands, answers, want) in cases {
            assert_eq!(
                summary(stands, answers, "", &said),
                want,
                "with {answers} answers"
            );
        }
    }
}
