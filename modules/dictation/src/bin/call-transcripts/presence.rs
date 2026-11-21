//! Who each voice told apart is, from who was in the Meet as it spoke: a
//! voice is someone only when that one person was the only other one
//! there whenever it said anything, once those another voice is sure to
//! be are set aside. Meet keeps only when each person first joined and
//! last left, so someone who stepped out is taken as there all along,
//! which names fewer voices, never more. Several people sitting at one
//! Meet are one person to it, so a voice is never named when another is
//! sure to be the same person.

/// How far apart Meet's clock and this laptop's, and when a take is said
/// to start and when its words were, may be: a person is taken as there
/// from this long before they joined to this long after they left.
pub const DRIFT: u64 = 30_000;

/// Someone in the Meet, Joel left out: when they first joined and last
/// left, in milliseconds since the epoch, with no leaving while they're
/// still in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Present {
    pub name: String,
    pub joined: u64,
    pub left: Option<u64>,
}

/// A take of theirs Parakeet heard words in: the voice it was told to be,
/// and when it began and ended, in milliseconds since the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spoke {
    pub voice: usize,
    pub from: u64,
    pub to: u64,
}

/// Who a voice is: the one person it can be, or those it may be, which is
/// nobody when no one was there each time it spoke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Who {
    Is(String),
    MayBe(Vec<String>),
}

/// Who each of `voices` voices is, by the takes of theirs `spoke` and who
/// was `present`. Each voice may be anyone there each time it spoke; one
/// that may be only one person is them, unless another may be only that
/// person too, and whoever a voice is is set aside from the rest, again
/// until none changes, so Bo is known by her voice from when Ada's was
/// sure. A voice with no take of words may be anyone, and sets nobody
/// aside, since it has no line to name.
pub fn who(present: &[Present], spoke: &[Spoke], voices: usize) -> Vec<Who> {
    let heard: Vec<bool> = (0..voices)
        .map(|voice| spoke.iter().any(|take| take.voice == voice))
        .collect();
    let mut can: Vec<Vec<usize>> = (0..voices)
        .map(|voice| {
            (0..present.len())
                .filter(|&person| {
                    spoke
                        .iter()
                        .filter(|take| take.voice == voice)
                        .all(|take| there(&present[person], take))
                })
                .collect()
        })
        .collect();
    loop {
        let sure = sure(&can, &heard);
        let mut changed = false;
        for voice in (0..voices).filter(|&voice| heard[voice] && sure[voice].is_none()) {
            let before = can[voice].len();
            can[voice].retain(|person| !sure.contains(&Some(*person)));
            changed |= can[voice].len() != before;
        }
        if !changed {
            return (0..voices)
                .map(|voice| match sure[voice] {
                    Some(person) => Who::Is(present[person].name.clone()),
                    None => Who::MayBe(names(&can[voice], present)),
                })
                .collect();
        }
    }
}

/// Whether `person` was in the Meet through `take`, give or take `DRIFT`.
fn there(person: &Present, take: &Spoke) -> bool {
    person.joined <= take.from.saturating_add(DRIFT)
        && person
            .left
            .is_none_or(|left| take.to <= left.saturating_add(DRIFT))
}

/// The person each voice is sure to be: the only one it `can` be, as long
/// as it was `heard` saying words and no other voice can be only them.
fn sure(can: &[Vec<usize>], heard: &[bool]) -> Vec<Option<usize>> {
    let only: Vec<Option<usize>> = can
        .iter()
        .zip(heard)
        .map(|(can, &heard)| match can.as_slice() {
            [one] if heard => Some(*one),
            _ => None,
        })
        .collect();
    only.iter()
        .map(|&one| one.filter(|_| only.iter().filter(|&&other| other == one).count() == 1))
        .collect()
}

/// The names of `people`, each once, in the order they're `present`.
fn names(people: &[usize], present: &[Present]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for &person in people {
        if !names.contains(&present[person].name) {
            names.push(present[person].name.clone());
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: u64 = 60_000;

    fn present(name: &str, joined: u64, left: Option<u64>) -> Present {
        Present {
            name: name.to_owned(),
            joined: joined * MINUTE,
            left: left.map(|left| left * MINUTE),
        }
    }

    fn spoke(voice: usize, from: u64) -> Spoke {
        Spoke {
            voice,
            from: from * MINUTE,
            to: from * MINUTE + 5_000,
        }
    }

    fn is(name: &str) -> Who {
        Who::Is(name.to_owned())
    }

    fn may_be(names: &[&str]) -> Who {
        Who::MayBe(names.iter().map(|&name| name.to_owned()).collect())
    }

    #[test]
    fn a_voice_heard_only_while_one_other_was_in_takes_that_name() {
        let present = [present("Ada", 0, None), present("Bo", 10, Some(20))];
        let named = who(&present, &[spoke(0, 2), spoke(0, 25)], 1);
        assert_eq!(
            named,
            [is("Ada")],
            "a voice heard only while Ada was the one other in wasn't named Ada"
        );
    }

    #[test]
    fn a_voice_heard_while_two_were_in_may_be_either() {
        let present = [present("Ada", 0, None), present("Bo", 0, None)];
        let named = who(&present, &[spoke(0, 2), spoke(0, 9)], 1);
        assert_eq!(
            named,
            [may_be(&["Ada", "Bo"])],
            "a voice heard while Ada and Bo were both in wasn't said to be either"
        );
    }

    #[test]
    fn one_who_joined_later_is_known_by_setting_aside_the_one_already_named() {
        let present = [present("Ada", 0, None), present("Bo", 10, None)];
        let spoken = [spoke(0, 2), spoke(1, 12), spoke(0, 15)];
        assert_eq!(
            who(&present, &spoken, 2),
            [is("Ada"), is("Bo")],
            "Bo's voice wasn't named once Ada's was sure"
        );
    }

    #[test]
    fn two_voices_sure_of_the_same_person_are_neither_named() {
        let present = [present("Ada", 0, None), present("Bo", 10, None)];
        let spoken = [spoke(0, 2), spoke(1, 4), spoke(0, 12)];
        assert_eq!(
            who(&present, &spoken, 2),
            [may_be(&["Ada"]), may_be(&["Ada"])],
            "two voices heard while only Ada was in were named, though one isn't her"
        );
    }

    #[test]
    fn a_take_just_before_a_join_counts_within_the_drift() {
        let present = [present("Ada", 0, None), present("Bo", 10, None)];
        let early = Spoke {
            voice: 1,
            from: 10 * MINUTE - DRIFT / 2,
            to: 10 * MINUTE,
        };
        let spoken = [spoke(0, 2), early, spoke(1, 12)];
        assert_eq!(
            who(&present, &spoken, 2),
            [is("Ada"), is("Bo")],
            "a take a little before Bo joined by Meet's clock kept Bo from her voice"
        );
    }

    #[test]
    fn a_voice_heard_when_nobody_was_in_is_nobody_known() {
        let present = [present("Ada", 10, Some(20))];
        assert_eq!(
            who(&present, &[spoke(0, 2), spoke(0, 12)], 1),
            [may_be(&[])],
            "a voice heard before anyone was in was given someone"
        );
    }

    #[test]
    fn two_people_of_one_name_are_said_once_and_never_named() {
        let present = [present("Ada", 0, None), present("Ada", 0, None)];
        assert_eq!(
            who(&present, &[spoke(0, 2)], 1),
            [may_be(&["Ada"])],
            "a voice that may be either of two Adas was named, or Ada said twice"
        );
    }

    #[test]
    fn a_voice_that_never_spoke_words_may_be_anyone_in() {
        let present = [present("Ada", 0, None), present("Bo", 0, None)];
        assert_eq!(
            who(&present, &[spoke(0, 2)], 2)[1],
            may_be(&["Ada", "Bo"]),
            "a voice with no worded take was given a name or nobody"
        );
    }
}
