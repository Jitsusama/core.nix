//! Why a machine's last update didn't all go ahead, as its node says it,
//! made one plain line a view can say beside the machine's other ties. Its
//! own module because it changes when update-to-main words how an update
//! ended, which none of a machine's other ties does.

use serde_json::Value;

/// The most characters of an update's why said, so a long error stays
/// one clause of a machine's line.
const WHY_AT_MOST: usize = 240;

/// Why a machine's last update didn't all go ahead, as its node says it:
/// a refusal's reason; a failure's first error, else its reason, since a
/// failure's reason names only the step that failed and the error is what
/// says why (cadence reads it so); or why a done update's system isn't the
/// one it boots. None for one that went ahead or is still under way, so a
/// reason an earlier attempt left can't stand for this one.
pub fn update_why(updated: &Value) -> Value {
    let reason = first_line(updated, "why");
    let reason = reason.as_deref();
    let why = match updated.get("step").and_then(Value::as_str) {
        Some("refused") => reason.and_then(refused_why),
        Some("failed") => failed_why(updated, reason),
        Some("done") => first_line(updated, "boots").as_deref().and_then(plain),
        _ => None,
    };
    why.map_or(Value::Null, Value::from)
}

/// The first line of what `updated` says as `field`, each check it names
/// said by its name: only the first, since the why is said as one line,
/// and a clause split off a later line would stand in for a first that
/// says something else.
fn first_line(updated: &Value, field: &str) -> Option<String> {
    let said = updated.get(field).and_then(Value::as_str)?;
    Some(uncommanded(said.lines().next().unwrap_or_default()))
}

/// How update-to-main's refusal for a hold turns from what it waits on to
/// what a person does about it (update.rs `held`).
const HOLD: &str = ", which waits for a hold: ";

/// Why an update was refused: its reason, with a hold's lever, what a
/// person does to let it through, kept whole beside a long reason's cut.
/// Only a hold's: what follows another refusal's colon is its reason, cut
/// from the end as the rest is.
fn refused_why(reason: &str) -> Option<String> {
    match reason.rsplit_once(HOLD) {
        Some((refusal, lever)) => beside(refusal, HOLD, lever),
        None => plain(reason),
    }
}

/// Why an update failed: its first error with words in it, else its
/// reason. When going back failed too, or there was nothing to go back to,
/// the reason's own failure is said, with that beside it kept whole however
/// long the failure, since only then is the machine left off the commit it
/// ran and waiting on a person. When going back failed, its lines follow
/// the switch's own with nothing between to tell them apart
/// (update-to-main's update.rs `going_back`), so a switch whose own lines
/// hold no error would take going back's as its cause: the reason names the
/// step that failed. With nothing to go back to, none ran, so every line is
/// the switch's own and its first error says why.
fn failed_why(updated: &Value, reason: Option<&str>) -> Option<String> {
    let stranded = reason.and_then(|reason| {
        STRANDED
            .iter()
            .find_map(|left| reason.find(left).map(|at| reason.split_at(at)))
    });
    match stranded {
        Some((failure, left)) => {
            let own = left
                .starts_with(NOTHING_BACK)
                .then(|| first_error(updated))
                .flatten();
            beside(own.as_deref().unwrap_or(failure), "; ", left)
        }
        None => first_error(updated)
            .as_deref()
            .map_or_else(|| reason.and_then(plain), plain),
    }
}

/// How update-to-main says a failed switch left the machine on no system
/// it ran, which a reason carries after the failure itself.
const STRANDED: [&str; 2] = ["going back failed too", NOTHING_BACK];

/// How update-to-main says a failed switch had no system to go back to, so
/// tried none.
const NOTHING_BACK: &str = "there was no system to go back to";

/// The first error in a failed update's lines with words in it, passing
/// over each that is only a piece waiting on one that failed: the lines
/// start at the first error that isn't (update-to-main's run.rs `tail`),
/// but when every error is one they are the last lines, and none says why.
fn first_error(updated: &Value) -> Option<String> {
    let lines: Vec<&str> = updated
        .get("lines")
        .and_then(Value::as_array)
        .map(|lines| lines.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let under = |line: &str| line.starts_with(char::is_whitespace);
    // Whether the error the line at hand belongs to is a waiting piece's:
    // its own line, or one of those indented under it, says so.
    let mut waits = false;
    for (at, &line) in lines.iter().enumerate() {
        let block = || lines[at + 1..].iter().take_while(|next| under(next));
        if !under(line) {
            waits = is_waiting(line) || block().any(|next| is_waiting(next));
        }
        if let Some(error) = line.trim().strip_prefix("error:")
            && !waits
            && plain(error).is_some()
        {
            return Some(cannot_build(error, block()).unwrap_or_else(|| error.to_owned()));
        }
    }
    None
}

/// The length of a store path's hash, before the dash and its name.
const STORE_HASH: usize = 32;

/// A build that failed in the words of Nix 2.29, "Cannot build
/// '/nix/store/<hash>-name.drv'.", with its reason on a line of its own
/// indented under it, said as "Cannot build name: <reason>": the path
/// would spend the why before it reached the reason, and the reason is
/// what says why. None for any other error.
fn cannot_build<'a>(error: &str, mut block: impl Iterator<Item = &'a &'a str>) -> Option<String> {
    let path = error
        .trim()
        .strip_prefix("Cannot build '")?
        .split_once('\'')?
        .0;
    let (hash, rest) = path
        .strip_prefix("/nix/store/")?
        .split_at_checked(STORE_HASH)?;
    let name = rest.strip_prefix('-')?.strip_suffix(".drv")?;
    if !hash.chars().all(|each| each.is_ascii_alphanumeric()) || name.is_empty() {
        return None;
    }
    let reason = block.find_map(|line| line.trim().strip_prefix("Reason:"))?;
    Some(format!("Cannot build {name}: {}", unended(reason)))
}

/// Whether a line of a Nix error says its piece failed only because one
/// it needs did, in the words of Nix before 2.29 and after, as
/// update-to-main's run.rs `is_waiting` tells it.
fn is_waiting(line: &str) -> bool {
    let line = line.trim();
    (line.starts_with("Reason:") && line.contains(" failed") && line.contains("dependenc"))
        || (line.starts_with("error:") && line.contains("dependencies of derivation"))
}

/// How update-to-main names a check by the command it ran (system.rs
/// `boot_once_checked` and `healthy`): a store path, which would spend the
/// why before it reached what the check said.
const CHECKS: [&str; 2] = ["the boot check `", "the health check `"];

/// `why` with each check it names said by its name alone, not its command.
fn uncommanded(why: &str) -> String {
    let mut said = why.to_owned();
    for check in CHECKS {
        let named = check.trim_end_matches([' ', '`']);
        if let Some(at) = said.find(check)
            && let Some(command) = said[at + check.len()..].find('`')
        {
            let after = said[at + check.len() + command + 1..].trim_start();
            let between = if after.is_empty() || after.starts_with([':', ',', ';', '.']) {
                ""
            } else {
                " "
            };
            said = format!("{}{named}{between}{after}", &said[..at]);
        }
    }
    said
}

/// `head` then `tail` as one plain line, since the tail says what to do or
/// where the machine was left: the tail whole where it takes no more than
/// half the why, else cut to that half, and the head cut to the room left.
fn beside(head: &str, between: &str, tail: &str) -> Option<String> {
    let Some(tail) = cut_to(tail, WHY_AT_MOST / 2) else {
        return plain(head);
    };
    // Its clauses are joined as plain joins the rest's.
    let between = between.replace("; ", ", ");
    let room = WHY_AT_MOST - tail.chars().count() - between.chars().count();
    match cut_to(head, room) {
        Some(head) => Some(format!("{head}{between}{tail}")),
        None => Some(tail),
    }
}

/// `why` as one plain line of at most `WHY_AT_MOST` characters, or none
/// when nothing is left of it (`cut_to`).
fn plain(why: &str) -> Option<String> {
    cut_to(why, WHY_AT_MOST)
}

/// `why` as one plain line, or none when nothing is left of it: its first
/// line, made plain as every owner's words are (src/plain.rs), since it
/// reaches a terminal; its own clauses joined by commas, so it reads as
/// one clause of a machine's line, which joins its clauses with "; "; no
/// closing mark, which the page gives it once; and at most `at_most`
/// characters, cut at a word and marked so.
fn cut_to(why: &str, at_most: usize) -> Option<String> {
    let first = crate::plain::words(why.lines().next().unwrap_or_default()).replace("; ", ", ");
    let said = unended(&first);
    if said.chars().count() <= at_most {
        return (!said.is_empty()).then(|| said.to_owned());
    }
    let keep = at_most.saturating_sub(1);
    let cut: String = said.chars().take(keep).collect();
    // A cut that ends a word whole, before a space or a clause's mark with a
    // space after it, keeps it; one inside a word, even before its
    // apostrophe, dash or slash, or a mark inside a name such as a unit's
    // dot, goes back to the word before.
    let mut rest = said.chars().skip(keep);
    let ends = match (rest.next(), rest.next()) {
        (Some(next), _) if next.is_whitespace() => true,
        (Some(next), after) if CLAUSE_ENDS.contains(&next) => after.is_none_or(char::is_whitespace),
        _ => false,
    };
    let word = if ends {
        cut.as_str()
    } else {
        cut.rfind(' ').map_or(cut.as_str(), |at| &cut[..at])
    };
    Some(format!("{}{CUT}", unended(word)))
}

/// What marks a why cut short.
const CUT: char = '\u{2026}';

/// The marks a clause can end on, which end a word as a space does.
const CLAUSE_ENDS: [char; 4] = ['.', ';', ':', ','];

/// `words` trimmed of space and of the marks a clause can end on.
fn unended(words: &str) -> &str {
    words.trim().trim_end_matches(['.', ';', ':', ',', ' '])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn why_of(updated: &Value) -> Value {
        update_why(updated)
    }

    #[test]
    fn an_update_carries_why_only_where_it_didnt_go_ahead() {
        let paused = "updates are paused for the follower, since uid 501 asked at \
                      1763755200; a person's ask still updates, and \
                      `update-to-main resume` lets the follower's run, and no commit is held";
        let joined = "updates are paused for the follower, since uid 501 asked at \
                      1763755200, a person's ask still updates, and \
                      `update-to-main resume` lets the follower's run, and no commit is held";
        assert_eq!(
            why_of(&json!({"step": "refused", "why": paused})),
            json!(joined),
            "a reason's own clauses are kept, joined by commas as one clause of the machine's line"
        );
        let full = "0123456789abcdef0123456789abcdef01234567";
        let held = format!(
            "0123456 is held, and main at 89abcde holds it; `update-to-main lift {full}` lets \
             it through"
        );
        let lever = why_of(&json!({"step": "refused", "why": held}));
        assert!(
            lever.as_str().is_some_and(|why| why.contains(full)),
            "a held commit's lever keeps the full id it needs: {lever}"
        );
        let path = "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-system-path.drv";
        let failed = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                            "lines": [format!("error: Cannot build '{path}'."),
                                      "       Reason: builder failed with exit code 1.",
                                      "error: Cannot build '/nix/store/y.drv'.",
                                      "       Reason: 1 dependency failed."]});
        assert_eq!(
            why_of(&failed),
            json!("Cannot build system-path: builder failed with exit code 1"),
            "a failure says its first error by the piece's name and its reason, not the step \
             that failed, and stops once"
        );
        let bare = json!({"step": "failed", "lines": ["switching"],
                          "why": "switching failed (exit status: 2); the machine went back to \
                                  what it ran"});
        assert_eq!(
            why_of(&bare),
            json!("switching failed (exit status: 2), the machine went back to what it ran"),
            "a failure with no error says its reason, and where it left the machine"
        );
        let traced = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                            "lines": ["error:", "       \u{2026} while evaluating 'x'",
                                      "       error: attribute 'x' missing"]});
        assert_eq!(
            why_of(&traced),
            json!("attribute 'x' missing"),
            "an error with nothing on its line gives way to the next that says something"
        );
        let nothing = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                             "lines": ["error:"]});
        assert_eq!(
            why_of(&nothing),
            json!("building the system failed (exit status: 1)"),
            "a failure whose errors say nothing says its reason"
        );
        let stranded = json!({"step": "failed",
                              "why": "switching failed (exit status: 4); going back failed too: \
                                      switching back failed (exit status: 4)",
                              "lines": ["error: unit nix-daemon.service failed",
                                        "error: unit nix-daemon.service failed again"]});
        assert_eq!(
            why_of(&stranded),
            json!(
                "switching failed (exit status: 4), going back failed too: switching back failed \
                 (exit status: 4)"
            ),
            "a machine left off what it ran says so beside the step that failed"
        );
        // Going back's lines follow the switch's with nothing between, so an
        // error among them is never taken for the switch's own.
        let backs = json!({"step": "failed",
                           "why": "switching failed (exit status: 4); going back failed too: \
                                   switching back failed (exit status: 4)",
                           "lines": ["warning: the following units failed: a.service",
                                     "error: b"]});
        assert_eq!(
            why_of(&backs),
            json!(
                "switching failed (exit status: 4), going back failed too: switching back failed \
                 (exit status: 4)"
            ),
            "going back's error isn't said as the switch's cause"
        );
        let old = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                         "lines": ["error: builder for '/nix/store/x.drv' failed with exit \
                                    code 1;"]});
        assert_eq!(
            why_of(&old),
            json!("builder for '/nix/store/x.drv' failed with exit code 1"),
            "an error's own closing mark is dropped, so the page's isn't doubled"
        );
    }

    #[test]
    fn a_switch_with_nothing_to_go_back_to_says_its_own_error() {
        let gone = json!({"step": "failed", "lines": ["error: the switch was cut off"],
                          "why": "switching failed (exit status: 4); there was no system to go \
                                  back to"});
        assert_eq!(
            why_of(&gone),
            json!("the switch was cut off, there was no system to go back to"),
            "a machine with nothing to go back to says the switch's own error beside it"
        );
        let mute = json!({"step": "failed", "lines": ["error:"],
                          "why": "switching failed (exit status: 4); there was no system to go \
                                  back to"});
        assert_eq!(
            why_of(&mute),
            json!("switching failed (exit status: 4), there was no system to go back to"),
            "and the step that failed when its lines say nothing"
        );
    }

    #[test]
    fn a_failed_update_keeps_where_it_left_the_machine_and_passes_over_waiting_pieces() {
        let waiting = json!({"step": "failed", "why": "building the system failed (exit status: 1)",
                             "lines": ["error: Cannot build '/nix/store/y.drv'.",
                                       "       Reason: 1 dependency failed.",
                                       "error: 1 dependencies of derivation '/nix/store/z.drv' \
                                        failed to build"]});
        assert_eq!(
            why_of(&waiting),
            json!("building the system failed (exit status: 1)"),
            "an error that is only a piece waiting on another, in either Nix's words, says no \
             why, so the reason is said"
        );
        let then = json!({"step": "failed", "why": "switching failed (exit status: 4)",
                          "lines": ["error: Cannot build '/nix/store/y.drv'.",
                                    "       Reason: 1 dependency failed.",
                                    "error: unit nix-daemon.service failed"]});
        assert_eq!(
            why_of(&then),
            json!("unit nix-daemon.service failed"),
            "an error after a waiting piece is said"
        );
        let long = "x ".repeat(115);
        let lost = json!({"step": "failed", "lines": [],
                          "why": format!("switching failed: {long}; there was no system to \
                                          go back to")});
        let lost = why_of(&lost);
        let lost = lost.as_str().expect("a long failure still says something");
        assert!(
            lost.chars().count() <= WHY_AT_MOST
                && lost.ends_with("x\u{2026}, there was no system to go back to"),
            "a long failure is cut before where the machine was left, which is kept: {lost}"
        );
        let unsaid = json!({"step": "failed", "lines": [],
                            "why": format!("switching failed: {long}; going back failed too: \
                                            switching back failed (exit status: 4)")});
        let unsaid = why_of(&unsaid);
        let unsaid = unsaid.as_str().expect("a long reason still says something");
        assert!(
            unsaid.chars().count() <= WHY_AT_MOST
                && unsaid.ends_with(
                    "x\u{2026}, going back failed too: switching back failed (exit status: 4)"
                ),
            "a long reason with no error keeps where the machine was left too: {unsaid}"
        );
        let checked = json!({"step": "failed", "lines": [],
                             "why": "the health check `/nix/store/0123-check/bin/check --now` \
                                     failed (exit status: 1); the machine went back to what it \
                                     ran"});
        assert_eq!(
            why_of(&checked),
            json!("the health check failed (exit status: 1), the machine went back to what it ran"),
            "a health check is said by its name, not the command it ran"
        );
    }

    #[test]
    fn a_long_ending_is_cut_to_half_and_still_said_after_the_cause() {
        let back = "y ".repeat(80);
        for (updated, what) in [
            (
                json!({"step": "failed", "lines": ["error: the switch was cut off"],
                       "why": format!("switching failed (exit status: 4); going back failed \
                                       too: {back}")}),
                "beside an error going back may have left",
            ),
            (
                json!({"step": "failed", "lines": [],
                       "why": format!("switching failed (exit status: 4); going back failed \
                                       too: {back}")}),
                "beside a reason",
            ),
        ] {
            let said = why_of(&updated);
            let said = said.as_str().expect("a long ending still says something");
            let (cause, ending) = said
                .split_once(", going back failed too")
                .expect("where the machine was left follows its cause");
            assert!(
                said.chars().count() <= WHY_AT_MOST
                    && !cause.is_empty()
                    && !cause.ends_with(',')
                    && "going back failed too".len() + ending.chars().count() <= WHY_AT_MOST / 2
                    && said.ends_with("y\u{2026}"),
                "{what}, a long ending is cut to half the why after its cause: {said}"
            );
        }
    }

    #[test]
    fn a_boot_check_says_its_cause_and_an_update_with_nothing_to_say_carries_none() {
        let boots = json!({"step": "done", "boots": "the boot check \
                           `/nix/store/0123456789abcdefghijklmnopqrstuv-check/bin/check --now` \
                           failed (exit status: 1), so the machine boots what it booted before: \
                           no loader; bad entry"});
        assert_eq!(
            why_of(&boots),
            json!(
                "the boot check failed (exit status: 1), so the machine boots what it booted \
                 before: no loader, bad entry"
            ),
            "a done update that boots its old system says why, without the check's command"
        );
        let quiet = json!({"step": "done", "boots": "the boot check `check` failed (exit \
                           status: 1), so the machine boots what it booted before: "});
        assert_eq!(
            why_of(&quiet),
            json!(
                "the boot check failed (exit status: 1), so the machine boots what it booted \
                 before"
            ),
            "a check that printed nothing ends its why without a colon"
        );
        let unrun = json!({"step": "done", "boots": "the boot check `x`: No such file or \
                           directory (os error 2), so the machine boots what it booted before: "});
        assert_eq!(
            why_of(&unrun),
            json!(
                "the boot check: No such file or directory (os error 2), so the machine boots \
                 what it booted before"
            ),
            "a check that couldn't run keeps its error beside its name"
        );
        let unbooted = json!({"step": "done", "boots": "making it what the machine boots failed \
                              (exit status: 1), so the machine boots what it booted before: \
                              no `loader` entry"});
        assert_eq!(
            why_of(&unbooted),
            json!(
                "making it what the machine boots failed (exit status: 1), so the machine boots \
                 what it booted before: no `loader` entry"
            ),
            "only a check's command is left out, never words in what it said"
        );
        for (updated, what) in [
            (json!({"step": "done"}), "an update that went ahead"),
            (
                json!({"step": "unchanged", "why": "left over"}),
                "an update with nothing to do",
            ),
            (
                json!({"step": "building", "why": "the Mac is over its room"}),
                "one under way",
            ),
            (
                json!({"step": "refused", "why": " . "}),
                "a reason with nothing in it",
            ),
            (
                json!({"step": "refused", "why": 4}),
                "a reason that isn't words",
            ),
        ] {
            assert_eq!(why_of(&updated), Value::Null, "{what} carries no why");
        }
    }

    #[test]
    fn an_update_why_is_one_plain_line_of_bounded_length() {
        let why = "busy, saying \u{1b}]8;;x\u{7}ok\rdone \u{202e}evil\u{2069}\nmore";
        assert_eq!(
            why_of(&json!({"step": "refused", "why": why})),
            json!("busy, saying ok done evil"),
            "its first line is said, a link's escape gone whole, a return as a space and what \
             would turn the words around gone"
        );
        assert_eq!(
            why_of(&json!({"step": "refused", "why": "busy;\tsaying so"})),
            json!("busy, saying so"),
            "a clause ended by a tab is joined as one ended by a space"
        );
        assert_eq!(
            why_of(&json!({"step": "refused", "why": "busy\nx, which waits for a hold: y"})),
            json!("busy"),
            "a later line's hold can't split off a lever of its own"
        );
        let path = "/run/update-to-main-hold/given";
        let units = ["a-rather-long-unit-name.service"; 10].join(", ");
        let hold = format!(
            "switching would change {units}, which waits for a hold: once the machine's work is \
             held, make {path} and ask again"
        );
        let cut = why_of(&json!({"step": "refused", "why": hold}));
        let cut = cut.as_str().expect("a long reason still says something");
        assert!(
            cut.chars().count() <= WHY_AT_MOST
                && cut.starts_with("switching would change ")
                && cut.contains(
                    "\u{2026}, which waits for a hold: once the machine's work is held, make \
                     /run/update-to-main-hold/given and ask again"
                ),
            "a long hold is cut at a word and marked before its lever, which is kept: {cut}"
        );
        let short = why_of(&json!({"step": "refused", "why": hold.replace(&units, "x.service")}));
        assert!(
            short.as_str().is_some_and(
                |why| why.ends_with("make /run/update-to-main-hold/given and ask again")
            ),
            "a hold's lever under the bound is said whole: {short}"
        );
        let colon = format!("{}: short", "word ".repeat(60));
        let other = why_of(&json!({"step": "refused", "why": colon}));
        assert!(
            other
                .as_str()
                .is_some_and(|why| why.ends_with("word\u{2026}") && !why.contains("short")),
            "another refusal's colon keeps nothing whole; it is cut from the end: {other}"
        );
        let told = format!(
            "0123456 is held, and whether main at 89abcde holds it couldn't be told: {}",
            "slow ".repeat(60)
        );
        let told = why_of(&json!({"step": "refused", "why": told}));
        assert!(
            told.as_str()
                .is_some_and(|why| why.chars().count() <= WHY_AT_MOST
                    && why.starts_with("0123456 is held, and whether main at 89abcde holds it")
                    && why.ends_with("slow\u{2026}")),
            "a hold that couldn't be told is cut from the end: {told}"
        );
    }

    #[test]
    fn a_cut_keeps_a_word_that_ends_where_the_cut_does() {
        let word = "a".repeat(WHY_AT_MOST - 3);
        let why = format!("b {word} rest");
        assert_eq!(
            why_of(&json!({"step": "refused", "why": why})),
            json!(format!("b {word}\u{2026}")),
            "a word ending just at the cut is kept whole"
        );
        assert_eq!(
            why_of(&json!({"step": "refused", "why": format!("b {word}, rest")})),
            json!(format!("b {word}\u{2026}")),
            "so is one a clause's mark ends"
        );
        for inside in ['\'', '-', '/', '.', ':', ','] {
            assert_eq!(
                why_of(&json!({"step": "refused", "why": format!("b {word}{inside}t rest")})),
                json!("b\u{2026}"),
                "a word cut at its {inside} goes back to the word before"
            );
        }
    }

    #[test]
    fn a_why_at_its_room_stands_whole_and_one_past_it_is_cut_to_it() {
        // Two bytes a character, so a room counted in bytes fails it.
        let whole = "é".repeat(WHY_AT_MOST);
        assert_eq!(
            why_of(&json!({"step": "refused", "why": whole})),
            json!(whole),
            "a why just as long as its room was cut"
        );
        // With no space to go back to, the cut falls inside the one word
        // rather than leave nothing of it.
        assert_eq!(
            why_of(&json!({"step": "refused", "why": "é".repeat(WHY_AT_MOST + 1)})),
            json!(format!("{}\u{2026}", "é".repeat(WHY_AT_MOST - 1))),
            "a why one past its room isn't cut to it"
        );
    }
}
