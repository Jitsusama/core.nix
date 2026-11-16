//! The GitHub receptor over refreshes made up here: whose review counts,
//! which owners are the realm's, and that nothing a change says reaches
//! mycelium.

use std::collections::BTreeMap;

use super::*;
use crate::github::SCHEMA;
use crate::model::{Change, Checks, ChecksState, Review};

const NOW: u64 = 1_800_000_000;
const MIDNIGHT: u64 = NOW - 10 * HOUR;

/// The machine's declared owners: kestrel-co for work, jsama for the
/// personal realm, and shared listed for both.
const DECLARED: &str = "work=Kestrel-Co,shared personal=jsama,SHARED";

fn login() -> Declared {
    Declared::login(Some("JSama".into())).expect("a login")
}

fn change(title: &str, updated_at: u64, asked_of: &[&str]) -> Change {
    Change {
        title: title.to_owned(),
        author: Some("osprey".to_owned()),
        draft: false,
        branch: "osprey/branch".to_owned(),
        base: "main".to_owned(),
        commit: None,
        updated_at,
        checks: Checks {
            state: ChecksState::None,
            passed: 0,
            failed: Vec::new(),
            pending: Vec::new(),
        },
        review: Review::Waiting,
        github: None,
        review_asked_of: asked_of.iter().copied().map(str::to_owned).collect(),
    }
}

fn seen(changes: &[(&str, Vec<Change>)], unread: &[&str]) -> Seen {
    Seen {
        schema: SCHEMA.to_owned(),
        read_at: NOW,
        changes: changes
            .iter()
            .map(|(key, changes)| ((*key).to_owned(), changes.clone()))
            .collect::<BTreeMap<_, _>>(),
        unread: unread.iter().copied().map(str::to_owned).collect(),
        through: BTreeMap::new(),
    }
}

/// The work realm's receptor, run as `program`.
fn receptor(program: &str) -> Receptor {
    Receptor::named(
        Some(program.into()),
        Some("work".into()),
        Some("jsama".into()),
        Some(DECLARED.into()),
    )
    .expect("the declared receptor")
}

#[test]
fn only_the_realms_own_owners_count_matched_as_github_matches_them() {
    let work = Owners::of(DECLARED, "work");
    let held = [
        "github:kestrel-co/tool",
        "github:KESTREL-CO/tool",
        "github:shared/tool",
        "github:jsama/tool",
        "github:osprey/tool",
        "gitlab:kestrel-co/tool",
        "kestrel-co/tool",
    ]
    .map(|key| work.hold(key));
    assert_eq!(
        held,
        [true, true, false, false, false, false, false],
        "an owner outside the realm, or listed for two, was counted"
    );
    assert!(
        !Owners::of(DECLARED, "elsewhere").hold("github:kestrel-co/tool"),
        "a realm the machine didn't list held another's owner"
    );
}

#[test]
fn the_changes_asking_the_declared_login_are_counted_by_hour_and_day() {
    let seen = seen(
        &[
            (
                "github:kestrel-co/tool",
                vec![
                    change("within the hour", NOW - 60, &["jsama"]),
                    change("this morning", NOW - 2 * HOUR, &["osprey", "JSAMA"]),
                    change("yesterday", MIDNIGHT - HOUR, &["jsama"]),
                    change("someone else's", NOW - 60, &["osprey"]),
                    change("a team's", NOW - 60, &[]),
                ],
            ),
            (
                "github:osprey/elsewhere",
                vec![change("not the realm's", NOW - 60, &["jsama"])],
            ),
        ],
        &[],
    );
    let told = told(
        &seen,
        &Owners::of(DECLARED, "work"),
        &login(),
        NOW,
        MIDNIGHT,
    );
    assert_eq!(
        (
            told.source,
            told.state,
            told.hour,
            told.today,
            told.addressed
        ),
        (Source::GitHub, State::Reading, 1, 2, 3)
    );
}

#[test]
fn it_reads_quiet_at_none_and_failing_while_a_realms_repository_went_unread() {
    let owners = Owners::of(DECLARED, "work");
    let quiet = seen(&[("github:kestrel-co/tool", Vec::new())], &[]);
    let others_unread = seen(&[], &["github:osprey/elsewhere: not readable here"]);
    let own_unread = seen(&[], &["github:kestrel-co/tool: not readable here"]);
    let states = [quiet, others_unread, own_unread]
        .map(|seen| told(&seen, &owners, &login(), NOW, MIDNIGHT).state);
    assert_eq!(states, [State::Quiet, State::Quiet, State::Failing]);
}

#[test]
fn no_receptor_is_said_without_all_the_machine_declares() {
    let program = || Some(OsString::from("/nix/store/x-mycelium/bin/mycelium"));
    let declared = || Some(OsString::from(DECLARED));
    let taken = [
        (
            program(),
            Some("work".into()),
            Some("jsama".into()),
            declared(),
        ),
        (program(), Some("work".into()), None, declared()),
        (
            program(),
            Some("work".into()),
            Some("not a login".into()),
            declared(),
        ),
        (program(), Some("work".into()), Some("jsama".into()), None),
        (program(), None, Some("jsama".into()), declared()),
        (
            Some("mycelium".into()),
            Some("work".into()),
            Some("jsama".into()),
            declared(),
        ),
    ]
    .map(|(program, realm, login, owners)| {
        Receptor::named(program, realm, login, owners).is_some()
    });
    assert_eq!(
        taken,
        [true, false, false, false, false, false],
        "a receptor was taken with part of it undeclared"
    );
}

#[test]
fn mycelium_is_given_fixed_words_and_counts_never_what_a_change_says() {
    let planted = "--realm personal";
    let seen = seen(
        &[(
            "github:kestrel-co/a-repository-named-loudly",
            vec![change(planted, NOW - 60, &["jsama"])],
        )],
        &[],
    );
    let said: Vec<String> = receptor("/nix/store/x-mycelium/bin/mycelium")
        .arguments(&seen, NOW, 4242)
        .into_iter()
        .map(|word| word.into_string().expect("words"))
        .collect();
    assert_eq!(
        said,
        [
            "--realm",
            "work",
            "receptors",
            "say",
            "github",
            "--state",
            "reading",
            "--hour",
            "1",
            "--today",
            "1",
            "--addressed",
            "1",
            "--every",
            "300",
            "--pid",
            "4242",
        ],
        "mycelium was given more than fixed words and counts"
    );
    for word in [
        planted,
        "kestrel-co",
        "a-repository-named-loudly",
        "jsama",
        "osprey",
    ] {
        assert!(
            !said.iter().any(|said| said.contains(word)),
            "{word} reached mycelium: {said:?}"
        );
    }
}

#[test]
fn the_day_begins_at_a_minute_on_or_before_now_within_a_day() {
    for now in [NOW, NOW + 59, NOW + 12 * HOUR + 1] {
        let began = midnight(now);
        assert!(
            began <= now && now - began < 24 * HOUR && began.is_multiple_of(60),
            "the day of {now} began at {began}"
        );
    }
}
