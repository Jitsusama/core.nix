//! Sessions laid out as Agents shows them: each under the session that
//! orchestrates it, a step in, an orchestration standing where its most
//! urgent row would, so it moves as one (QEST-20261026-15J7N3's
//! agents-tree-look.md).

use std::collections::HashMap;

use crate::standing::{RUNNING, Stands};

/// The deepest a row sits under another (docs/design.md keeps no layer
/// more than two deep).
const DEEPEST: usize = 2;

/// The fewest waiting members an orchestration folds, since folding a
/// lone one saves no room and hides its name.
const FOLDS_AT: usize = 2;

/// A session as the tree reads it.
#[derive(Clone, Copy, Debug)]
pub struct Member<'a> {
    pub address: &'a str,
    /// The address of the session orchestrating it, or "".
    pub under: &'a str,
    pub stands: Stands,
    /// Whether it stands at Agents' top by name, as the void does.
    pub at_top: bool,
}

/// One entry of the laid-out list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Laid<'a> {
    /// A session's row: `at` its place in what was given, `under` the
    /// address of the row it's drawn under or "", and `orchestrates`
    /// whether any row is under it.
    Row {
        at: usize,
        depth: usize,
        under: &'a str,
        orchestrates: bool,
    },
    /// An orchestration's members that only wait, folded after its own
    /// rows (VKCF3Z, 2026-11-06), so a tree of sixty sessions shows what
    /// moves: `members` laid as the tree is, from depth 0, and `counts`
    /// how many of each state it holds, in the one order.
    Fold {
        depth: usize,
        under: &'a str,
        members: Vec<Laid<'a>>,
        counts: Vec<(Stands, usize)>,
    },
}

/// Sessions laid out from those given in the order Sessions keeps them.
/// Siblings stand in the order of how they stand, then the order given.
/// Orchestrators naming each other are cut where the first is met. With
/// `folds`, an orchestration's members that only wait, those listening,
/// paused or parked with nothing under them more urgent, fold into one
/// entry.
pub fn tree<'a>(sessions: &[Member<'a>], folds: bool) -> Vec<Laid<'a>> {
    let parent = parents(sessions);
    let mut children = vec![Vec::new(); sessions.len()];
    for (at, up) in parent.iter().enumerate() {
        if let Some(up) = up {
            children[*up].push(at);
        }
    }
    let mut best = vec![None; sessions.len()];
    for at in 0..sessions.len() {
        best_of(at, sessions, &children, &mut best);
    }
    let layer = Layer {
        sessions,
        parent: &parent,
        children: &children,
        best: best
            .into_iter()
            .map(|one| one.unwrap_or((Stands::Off, 0)))
            .collect(),
        folds,
    };
    let mut laid = Vec::new();
    let roots: Vec<_> = (0..sessions.len())
        .filter(|at| parent[*at].is_none())
        .collect();
    for at in layer.in_order(&roots) {
        layer.lay(&mut laid, at, 0);
    }
    laid
}

/// Each session's orchestrator, where that one's listed, by its place:
/// the first listed of an address, never itself, and cut where following
/// them up comes round again.
fn parents(sessions: &[Member<'_>]) -> Vec<Option<usize>> {
    let mut by_address = HashMap::new();
    for (at, one) in sessions.iter().enumerate() {
        by_address.entry(one.address).or_insert(at);
    }
    let mut parent: Vec<_> = sessions
        .iter()
        .enumerate()
        .map(|(at, one)| {
            (!one.under.is_empty())
                .then(|| by_address.get(one.under).copied())
                .flatten()
                .filter(|found| *found != at)
        })
        .collect();
    for at in 0..parent.len() {
        let mut seen = vec![at];
        let mut up = parent[at];
        while let Some(one) = up {
            if seen.contains(&one) {
                parent[at] = None;
                break;
            }
            seen.push(one);
            up = parent[one];
        }
    }
    parent
}

/// The most urgent of a row and those under it, as how it stands and
/// its place, kept once found.
fn best_of(
    at: usize,
    sessions: &[Member<'_>],
    children: &[Vec<usize>],
    best: &mut [Option<(Stands, usize)>],
) -> (Stands, usize) {
    if let Some(found) = best[at] {
        return found;
    }
    let own = (sessions[at].stands, at);
    let found = children[at]
        .iter()
        .map(|child| best_of(*child, sessions, children, best))
        .fold(own, std::cmp::min);
    best[at] = Some(found);
    found
}

/// What laying the rows out reads.
struct Layer<'s, 'a> {
    sessions: &'s [Member<'a>],
    parent: &'s [Option<usize>],
    children: &'s [Vec<usize>],
    best: Vec<(Stands, usize)>,
    folds: bool,
}

impl<'a> Layer<'_, 'a> {
    fn in_order(&self, list: &[usize]) -> Vec<usize> {
        let mut ordered = list.to_vec();
        ordered.sort_by_key(|at| self.best[*at]);
        ordered
    }

    fn lay(&self, into: &mut Vec<Laid<'a>>, at: usize, depth: usize) {
        into.push(Laid::Row {
            at,
            depth: depth.min(DEEPEST),
            under: self.parent[at].map_or("", |up| self.sessions[up].address),
            orchestrates: !self.children[at].is_empty(),
        });
        let under = self.in_order(&self.children[at]);
        let waiting: Vec<_> = if self.folds {
            under
                .iter()
                .copied()
                .filter(|one| self.best[*one].0.waits())
                .collect()
        } else {
            Vec::new()
        };
        let folded = if waiting.len() >= FOLDS_AT {
            waiting
        } else {
            Vec::new()
        };
        for child in under.iter().filter(|one| !folded.contains(one)) {
            self.lay(into, *child, depth + 1);
        }
        if !folded.is_empty() {
            let mut members = Vec::new();
            for child in &folded {
                self.lay(&mut members, *child, 0);
            }
            let counts = self.counted(&members);
            into.push(Laid::Fold {
                depth: (depth + 1).min(DEEPEST),
                under: self.sessions[at].address,
                members,
                counts,
            });
        }
    }

    /// How many of each state a fold holds, one folded within it too.
    fn counted(&self, members: &[Laid<'_>]) -> Vec<(Stands, usize)> {
        let mut held = Vec::new();
        gather(members, &mut held);
        RUNNING
            .iter()
            .map(|each| {
                let count = held
                    .iter()
                    .filter(|at| self.sessions[**at].stands == *each)
                    .count();
                (*each, count)
            })
            .filter(|(_, count)| *count > 0)
            .collect()
    }
}

/// Every session a fold holds, one folded within it too.
fn gather(members: &[Laid<'_>], into: &mut Vec<usize>) {
    for one in members {
        match one {
            Laid::Row { at, .. } => into.push(*at),
            Laid::Fold { members, .. } => gather(members, into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sessions said as "address:state>orchestrator", in the order
    /// Sessions keeps them: "B:working>A" is B, working, under A.
    fn sessions(said: &str) -> Vec<Member<'_>> {
        said.split_whitespace()
            .map(|one| {
                let (own, under) = one.split_once('>').unwrap_or((one, ""));
                let (address, state) = own.split_once(':').expect("each says its state");
                let stands = RUNNING
                    .iter()
                    .copied()
                    .find(|stands| stands.id() == state)
                    .expect("a state the tree knows");
                Member {
                    address,
                    under,
                    stands,
                    at_top: false,
                }
            })
            .collect()
    }

    #[test]
    fn one_standing_at_the_top_follows_those_needing_joel_and_leads_the_rest() {
        let mut list = sessions("A:working B:needs V:listening C:working>A D:paused");
        list[2].at_top = true;
        assert_eq!(
            read(&list, &tree(&list, false)),
            "B V A .C D",
            "the void keeps one place, however it and the rest stand"
        );
        list[2].stands = Stands::Needs;
        assert_eq!(
            read(&list, &tree(&list, false)),
            "B V A .C D",
            "needing Joel itself, it stands among those that do, in its order"
        );
    }

    /// The laid-out list as it reads: each address, a dot for each step
    /// in, and a fold as how many of each state it holds, then its members.
    fn read(list: &[Member<'_>], laid: &[Laid<'_>]) -> String {
        let entries: Vec<_> = laid
            .iter()
            .map(|one| match one {
                Laid::Row { at, depth, .. } => {
                    format!("{}{}", ".".repeat(*depth), list[*at].address)
                }
                Laid::Fold {
                    depth,
                    members,
                    counts,
                    ..
                } => {
                    let counted: Vec<_> = counts
                        .iter()
                        .map(|(stands, count)| format!("{count} {}", stands.id()))
                        .collect();
                    let dots = ".".repeat(*depth);
                    format!("{dots}({}: {})", counted.join(", "), read(list, members))
                }
            })
            .collect();
        entries.join(" ")
    }

    #[test]
    fn sessions_sit_under_their_orchestrators() {
        let cases = [
            (
                "sessions alone keep the order of how they stand",
                "A:listening B:needs C:working D:paused E:inLine",
                "B C E A D",
            ),
            (
                "and within a state the order they came in",
                "A:working B:working",
                "A B",
            ),
            (
                "a session follows its orchestrator, a step in",
                "A:working B:working C:working>A",
                "A .C B",
            ),
            (
                "its orchestrator's orchestrator two steps in",
                "P:listening O:working>P B:working>O",
                "P .O ..B",
            ),
            (
                "and never deeper than two",
                "P:working O:working>P B:working>O C:working>B",
                "P .O ..B ..C",
            ),
            (
                "rows under one orchestrator keep the order of how they stand",
                "O:working A:listening>O B:needs>O C:working>O",
                "O .B .C .A",
            ),
            (
                "one parked sits under its orchestrator, last",
                "O:parked A:parked>O B:listening>O X:paused",
                "O .B .A X",
            ),
            (
                "an orchestration stands where its most urgent row would",
                "X:working O:listening A:needs>O",
                "O .A X",
            ),
            (
                "and so does one under it, with its own",
                "O:working P:listening>O Q:working>O B:needs>P",
                "O .P ..B .Q",
            ),
            (
                "one whose orchestrator isn't listed stands at the top, not in",
                "A:working B:working>GONE",
                "A B",
            ),
            ("one naming itself is its own", "A:working>A", "A"),
            (
                "orchestrators naming each other are cut where they meet",
                "A:working>B B:working>A",
                "A .B",
            ),
        ];
        for (what, said, want) in cases {
            let list = sessions(said);
            assert_eq!(read(&list, &tree(&list, false)), want, "{what}");
        }
    }

    #[test]
    fn an_orchestrations_waiting_members_fold() {
        let cases = [
            (
                "the waiting fold after those that move",
                "O:working A:listening>O B:working>O C:paused>O D:parked>O",
                "O .B .(1 listening, 1 paused, 1 parked: A C D)",
            ),
            (
                "one waiting alone stays a row",
                "O:working A:listening>O B:working>O",
                "O .B .A",
            ),
            (
                "one under it that moves keeps its orchestrator out of the fold",
                "O:listening P:listening>O Q:listening>O R:listening>O B:working>P",
                "O .P ..B .(2 listening: Q R)",
            ),
            (
                "one that waits with its own that wait folds whole",
                "O:working P:listening>O Q:paused>O B:listening>P",
                "O .(2 listening, 1 paused: P .B Q)",
            ),
            (
                "those under none never fold",
                "A:listening B:listening C:paused",
                "A B C",
            ),
        ];
        for (what, said, want) in cases {
            let list = sessions(said);
            assert_eq!(read(&list, &tree(&list, true)), want, "folded, {what}");
        }
    }

    #[test]
    fn each_row_says_whom_it_is_under_and_whether_it_orchestrates() {
        let list = sessions("O:working A:working>O B:working>GONE");
        let said: Vec<_> = tree(&list, false)
            .iter()
            .map(|one| match one {
                Laid::Row {
                    at,
                    under,
                    orchestrates,
                    ..
                } => {
                    let mark = if *orchestrates { "*" } else { "" };
                    format!("{}<{under}{mark}", list[*at].address)
                }
                Laid::Fold { .. } => "fold".to_owned(),
            })
            .collect();
        assert_eq!(said.join(" "), "O<* A<O B<");
    }
}
