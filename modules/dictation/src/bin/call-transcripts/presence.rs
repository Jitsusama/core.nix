//! Who each voice told apart is, from who was in the Meet as it spoke: a
//! voice is someone when that one name was the only other one there
//! whenever it said anything. Once some are, setting them aside from the
//! other voices may leave a voice one name, but that rests on each voice
//! being a different person, which a split voice or two people at one
//! seat break, so such a name is only likely, never sure. Several people
//! sitting at one Meet are one person to it, so a voice is never named
//! when another is sure to be the same person. Meet keeps only when each
//! person first joined and last left, so someone who stepped out is taken
//! as there all along: that keeps a voice from being sure of anyone
//! while they were out, and may set aside the wrong one for a likely
//! name.

use dictation::transcript::voice_called;

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

/// Who a voice is: the one name there each time it spoke; the one left
/// once those other voices are have been set aside, which is only likely;
/// or those it may be, which is nobody when no one was there each time it
/// spoke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Who {
    Is(String),
    Likely(String),
    MayBe(Vec<String>),
}

/// Who each of `voices` voices is, by the takes of theirs `spoke` and who
/// was `present`. Each voice may be any name there each time it spoke; one
/// that may be only one name is them, unless another may be only that name
/// too. Whoever a voice is is set aside from the rest, again until none
/// changes, so Bo is likely her voice from when Ada's was sure. A voice
/// with no take of words may be anyone, and sets nobody aside, since it
/// has no line to name; one with everyone set aside may be any it was
/// before.
pub fn who(present: &[Present], spoke: &[Spoke], voices: usize) -> Vec<Who> {
    let people = names(present);
    let heard: Vec<bool> = (0..voices)
        .map(|voice| spoke.iter().any(|take| take.voice == voice))
        .collect();
    let could: Vec<Vec<usize>> = (0..voices)
        .map(|voice| {
            (0..people.len())
                .filter(|&person| {
                    spoke
                        .iter()
                        .filter(|take| take.voice == voice)
                        .all(|take| there(people[person], present, take))
                })
                .collect()
        })
        .collect();
    let direct = sure(&could, &heard);
    let mut can = could.clone();
    loop {
        let sure = sure(&can, &heard);
        let mut changed = false;
        for voice in (0..voices).filter(|&voice| heard[voice] && sure[voice].is_none()) {
            let before = can[voice].len();
            can[voice].retain(|person| !sure.contains(&Some(*person)));
            changed |= can[voice].len() != before;
        }
        if !changed {
            let named = |people_of: &[usize]| -> Vec<String> {
                people_of
                    .iter()
                    .map(|&person| people[person].to_owned())
                    .collect()
            };
            return (0..voices)
                .map(|voice| match (sure[voice], direct[voice]) {
                    (Some(person), Some(_)) => Who::Is(people[person].to_owned()),
                    (Some(person), None) => Who::Likely(people[person].to_owned()),
                    (None, _) if can[voice].is_empty() => Who::MayBe(named(&could[voice])),
                    (None, _) => Who::MayBe(named(&can[voice])),
                })
                .collect();
        }
    }
}

/// What a transcript says of the voices it leaves numbered, of those
/// `heard` in it: who each is likely to be, or who it may be; none when
/// that's nothing. A sure name needs no saying, since its voice's lines
/// are under it.
pub fn guesses(who: &[Who], heard: &[bool]) -> Option<String> {
    let said: Vec<String> = who
        .iter()
        .zip(heard)
        .enumerate()
        .filter(|(_, (_, heard))| **heard)
        .filter_map(|(voice, (who, _))| {
            let called = voice_called(voice);
            match who {
                Who::Is(_) => None,
                Who::Likely(name) => Some(format!("{called} is likely {name}.")),
                Who::MayBe(names) => either(names).map(|names| format!("{called} may be {names}.")),
            }
        })
        .collect();
    (!said.is_empty()).then(|| said.join(" "))
}

/// `names` as one of them, "Ada, Bo or Cy"; none for no names.
fn either(names: &[String]) -> Option<String> {
    match names {
        [] => None,
        [one] => Some(one.clone()),
        [rest @ .., last] => Some(format!("{} or {last}", rest.join(", "))),
    }
}

/// Whether anyone named `name` was in the Meet through `take`, give or
/// take `DRIFT`: one who shows twice, as from a second device, is one name.
fn there(name: &str, present: &[Present], take: &Spoke) -> bool {
    present
        .iter()
        .filter(|person| person.name == name)
        .any(|person| {
            person.joined <= take.from.saturating_add(DRIFT)
                && person
                    .left
                    .is_none_or(|left| take.to <= left.saturating_add(DRIFT))
        })
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

/// The names of those `present`, each once, in the order they're present.
fn names(present: &[Present]) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();
    for person in present {
        if !names.contains(&person.name.as_str()) {
            names.push(&person.name);
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

    fn likely(name: &str) -> Who {
        Who::Likely(name.to_owned())
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
    fn one_who_joined_later_is_likely_once_the_one_already_named_is_set_aside() {
        let present = [present("Ada", 0, None), present("Bo", 10, None)];
        let spoken = [spoke(0, 2), spoke(1, 12), spoke(0, 15)];
        assert_eq!(
            who(&present, &spoken, 2),
            [is("Ada"), likely("Bo")],
            "Bo's voice wasn't taken as likely hers once Ada's was sure, or was taken as sure"
        );
    }

    #[test]
    fn a_voice_left_with_nobody_once_others_are_set_aside_may_be_any_it_was() {
        let present = [present("Ada", 0, Some(15)), present("Bo", 5, None)];
        let spoken = [spoke(0, 2), spoke(1, 20), spoke(2, 8)];
        assert_eq!(
            who(&present, &spoken, 3),
            [is("Ada"), is("Bo"), may_be(&["Ada", "Bo"])],
            "a third voice heard while Ada and Bo were in was given nobody, or one of them"
        );
    }

    #[test]
    fn a_voice_heard_after_one_left_is_not_them() {
        let present = [present("Ada", 0, None), present("Bo", 0, Some(10))];
        let named = who(&present, &[spoke(0, 2), spoke(0, 12)], 1);
        assert_eq!(
            named,
            [is("Ada")],
            "a voice heard two minutes after Bo left was still taken as maybe Bo"
        );
    }

    #[test]
    fn a_take_just_after_a_leave_counts_within_the_drift() {
        let present = [present("Ada", 0, Some(10))];
        let late = Spoke {
            voice: 0,
            from: 10 * MINUTE,
            to: 10 * MINUTE + DRIFT / 2,
        };
        assert_eq!(
            who(&present, &[late], 1),
            [is("Ada")],
            "a take ending a little after Ada left by Meet's clock kept her from her voice"
        );
    }

    #[test]
    fn one_who_shows_twice_in_the_meet_is_one_name() {
        let present = [
            present("Ada", 0, None),
            present("Bo", 0, Some(5)),
            present("Bo", 10, None),
        ];
        let named = who(&present, &[spoke(0, 2), spoke(0, 12)], 1);
        assert_eq!(
            named,
            [may_be(&["Ada", "Bo"])],
            "a voice heard in both of Bo's times in the Meet was named Ada"
        );
    }

    #[test]
    fn a_voice_with_no_words_is_not_set_aside_from_anyone() {
        let present = [present("Ada", 0, None), present("Bo", 10, None)];
        assert_eq!(
            who(&present, &[spoke(0, 2)], 2),
            [is("Ada"), may_be(&["Ada", "Bo"])],
            "a voice with no worded take had Ada set aside from it"
        );
    }

    #[test]
    fn a_voice_with_no_words_is_never_sure_of_anyone() {
        let present = [present("Ada", 0, None)];
        assert_eq!(
            who(&present, &[spoke(0, 2)], 2),
            [is("Ada"), may_be(&["Ada"])],
            "a voice with no worded take was sure of Ada, keeping her from the voice that spoke"
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
        let present = [present("Bo", 10, None)];
        let early = Spoke {
            voice: 0,
            from: 10 * MINUTE - DRIFT / 2,
            to: 10 * MINUTE,
        };
        assert_eq!(
            who(&present, &[early], 1),
            [is("Bo")],
            "a take a little before Bo joined by Meet's clock kept Bo from her voice"
        );
    }

    #[test]
    fn a_take_further_before_a_join_than_the_drift_is_not_them() {
        let present = [present("Bo", 10, None)];
        let early = Spoke {
            voice: 0,
            from: 10 * MINUTE - DRIFT - 5_000,
            to: 10 * MINUTE,
        };
        assert_eq!(
            who(&present, &[early], 1),
            [may_be(&[])],
            "a take begun well before Bo joined was taken as maybe hers"
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
    fn two_people_of_one_name_are_that_name() {
        let present = [present("Ada", 0, None), present("Ada", 0, None)];
        assert_eq!(
            who(&present, &[spoke(0, 2), spoke(1, 3)], 2),
            [may_be(&["Ada"]), may_be(&["Ada"])],
            "two voices heard while only Adas were in were named, or Ada was said twice"
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

    #[test]
    fn the_voices_left_numbered_are_said_likely_or_maybe_and_only_those_heard() {
        let who = [
            is("Ada"),
            likely("Bo"),
            may_be(&["Cy", "Di", "Ed"]),
            may_be(&["Fi"]),
            may_be(&[]),
            may_be(&["Gus"]),
        ];
        assert_eq!(
            guesses(&who, &[true, true, true, true, true, false]).as_deref(),
            Some("Voice 2 is likely Bo. Voice 3 may be Cy, Di or Ed. Voice 4 may be Fi."),
            "the guesses didn't say who each numbered voice heard may be"
        );
        assert_eq!(
            guesses(&[is("Ada"), may_be(&[])], &[true, true]),
            None,
            "a sure name or a voice that may be nobody was said as a guess"
        );
    }
}
