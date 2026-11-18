//! The GitHub receptor over refreshes made up here: whose review counts,
//! which owners are the realm's, and that nothing a change says reaches
//! mycelium.

use std::collections::BTreeMap;

use super::*;
use crate::github::SCHEMA;
use crate::model::{Change, Checks, ChecksState, GitHubChange, Review};

const NOW: u64 = 1_800_000_000;

/// The machine's declared owners: kestrel-co for work, jsama for the
/// personal realm, and shared listed for both, as one word, since systemd
/// splits an unquoted `Environment=` at whitespace.
const DECLARED: &str = "work=Kestrel-Co,shared;personal=jsama,SHARED";

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

/// `change`, numbered `number` as GitHub numbers it, in the repository
/// GitHub spells `repo`.
fn numbered(repo: &str, number: u64, change: Change) -> Change {
    Change {
        github: Some(GitHubChange {
            number,
            url: format!("https://github.com/{repo}/pull/{number}"),
        }),
        ..change
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
fn owners_with_whitespace_among_them_count_nowhere() {
    // What systemd hands over of a value it split is part of it, so a
    // realm's owners may have gone missing; none is trusted.
    for split in [
        "work=Kestrel-Co,shared personal=jsama,SHARED",
        "personal=jsama,SHARED ",
        "personal=jsama,\tSHARED",
    ] {
        let personal = Owners::of(split, "personal");
        assert!(
            !personal.hold("github:jsama/tool") && !personal.hold("github:shared/tool"),
            "owners declared with whitespace counted: {split:?}"
        );
    }
    assert!(
        Owners::of(DECLARED, "personal").hold("github:jsama/tool"),
        "owners declared as one word didn't count"
    );
}

#[test]
fn the_open_changes_read_are_counted_and_those_asking_the_login_however_long_ago() {
    let seen = seen(
        &[
            (
                "github:kestrel-co/tool",
                vec![
                    change("within the hour", NOW - HOUR + 1, &["jsama"]),
                    change("an hour ago", NOW - HOUR, &["osprey", "JSAMA"]),
                    change("last week", NOW - 7 * 24 * HOUR, &["jsama"]),
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
    let told = told(&seen, &Owners::of(DECLARED, "work"), &login(), NOW);
    // Mycelium refuses a record whose hour or addressed passes its today,
    // so every review asked of the login is one of the changes read.
    assert_eq!(
        (
            told.source,
            told.state,
            told.hour,
            told.today,
            told.addressed
        ),
        (Source::GitHub, State::Reading, 3, 5, 3),
        "the realm's open changes, those updated within the hour, or those \
         asking the login were miscounted"
    );
}

#[test]
fn it_reads_quiet_at_none_and_failing_while_a_realms_repository_went_unread() {
    let owners = Owners::of(DECLARED, "work");
    let quiet = seen(&[("github:kestrel-co/tool", Vec::new())], &[]);
    let others_unread = seen(&[], &["github:osprey/elsewhere: not readable here"]);
    let own_unread = seen(&[], &["github:kestrel-co/tool: not readable here"]);
    let states =
        [quiet, others_unread, own_unread].map(|seen| told(&seen, &owners, &login(), NOW).state);
    assert_eq!(
        states,
        [State::Quiet, State::Quiet, State::Failing],
        "a realm's repository unread, or another realm's, was judged wrong"
    );
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
        (
            program(),
            Some("work".into()),
            Some("jsama".into()),
            Some("personal=jsama".into()),
        ),
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
        [true, false, false, false, false, false, false],
        "a receptor was taken with part of it undeclared, or no owner for its realm"
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
            "--owner",
            "Kestrel-Co",
        ],
        "mycelium was given more than fixed words, counts and the realm's owners"
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
fn only_the_realms_changes_asking_the_login_are_named_by_number_alone() {
    let planted = "Fix the sign-in";
    let seen = seen(
        &[
            (
                "github:KESTREL-CO/tool",
                vec![
                    numbered("Kestrel-Co/tool", 12, change(planted, NOW - 60, &["JSAMA"])),
                    numbered("Kestrel-Co/tool", 13, change("else", NOW - 60, &["osprey"])),
                    numbered(
                        "Kestrel-Co/tool",
                        0,
                        change("no number", NOW - 60, &["jsama"]),
                    ),
                    change("never numbered", NOW - 60, &["jsama"]),
                    numbered(
                        "Kestrel-Co/tool",
                        9,
                        change("older", NOW - 7 * 24 * HOUR, &["jsama"]),
                    ),
                ],
            ),
            (
                // A second clone of the same repository, its remote typed in
                // another case, its copy last updated before #9, so the two
                // copies never sit side by side when sorted by time.
                "github:kestrel-co/tool",
                vec![numbered(
                    "Kestrel-Co/tool",
                    12,
                    change(planted, NOW - 8 * 24 * HOUR, &["jsama"]),
                )],
            ),
            (
                "github:osprey/elsewhere",
                vec![numbered(
                    "osprey/elsewhere",
                    4,
                    change("realm's?", NOW - 60, &["jsama"]),
                )],
            ),
            (
                "github:shared/tool",
                vec![numbered(
                    "shared/tool",
                    5,
                    change("both realms'", NOW - 60, &["jsama"]),
                )],
            ),
        ],
        &[],
    );
    assert_eq!(
        asked(&seen, &Owners::of(DECLARED, "work"), &login()),
        ["Kestrel-Co/tool#12", "Kestrel-Co/tool#9"],
        "a change was named twice, not as GitHub spells it, or not the realm's, asking the \
         login or numbered"
    );
    let said: Vec<String> = receptor("/nix/store/x-mycelium/bin/mycelium")
        .arguments(&seen, NOW, 4242)
        .into_iter()
        .map(|word| word.into_string().expect("words"))
        .collect();
    let after = said
        .iter()
        .position(|word| word == "4242")
        .expect("the counts are said first");
    assert_eq!(
        said[after + 1..],
        [
            "--owner",
            "Kestrel-Co",
            "--item",
            "Kestrel-Co/tool#12",
            "--item",
            "Kestrel-Co/tool#9",
        ],
        "the changes weren't named after the realm's owners, as GitHub spells them"
    );
    for word in [planted, "osprey", "else", "jsama"] {
        assert!(
            !said.iter().any(|said| said.contains(word)),
            "{word} reached mycelium: {said:?}"
        );
    }
}

#[test]
fn a_read_that_failed_names_no_changes_so_mycelium_keeps_those_it_had() {
    let asking = numbered(
        "kestrel-co/tool",
        12,
        change("asking", NOW - 60, &["jsama"]),
    );
    let said = |unread: &str| -> Vec<String> {
        let seen = seen(
            &[("github:kestrel-co/tool", vec![asking.clone()])],
            &[unread],
        );
        receptor("/nix/store/x-mycelium/bin/mycelium")
            .arguments(&seen, NOW, 4242)
            .into_iter()
            .map(|word| word.into_string().expect("words"))
            .collect()
    };
    let failed = said("github:kestrel-co/other: not readable here");
    assert!(
        failed.contains(&"failing".to_owned()),
        "it was told failing: {failed:?}"
    );
    assert!(
        !failed
            .iter()
            .any(|word| word == "--owner" || word == "--item"),
        "a failed read named changes, clearing those mycelium kept: {failed:?}"
    );
    let elsewhere = said("github:osprey/other: not readable here");
    assert!(
        elsewhere.contains(&"kestrel-co/tool#12".to_owned()),
        "another realm's unread repository stopped this realm's naming: {elsewhere:?}"
    );
    // The owner is declared Kestrel-Co, but GitHub spells it kestrel-co,
    // and mycelium takes an item only of an owner it was told as spelled.
    assert!(
        elsewhere
            .windows(2)
            .any(|pair| pair == ["--owner", "kestrel-co"]),
        "the owner the item spells wasn't said, so mycelium would refuse it: {elsewhere:?}"
    );
}

#[test]
fn a_declared_owner_mycelium_wouldnt_take_is_never_said() {
    let receptor = Receptor::named(
        Some("/nix/store/x-mycelium/bin/mycelium".into()),
        Some("work".into()),
        Some("jsama".into()),
        Some("work=Kestrel-Co,b@d".into()),
    )
    .expect("the declared receptor");
    let said: Vec<String> = receptor
        .arguments(&seen(&[], &[]), NOW, 4242)
        .into_iter()
        .map(|word| word.into_string().expect("words"))
        .collect();
    assert!(said.contains(&"Kestrel-Co".to_owned()), "{said:?}");
    assert!(
        !said.contains(&"b@d".to_owned()),
        "an owner mycelium refuses was said, costing the counts: {said:?}"
    );
}

#[test]
fn a_change_mycelium_would_refuse_is_left_out_and_no_more_are_named_than_it_keeps() {
    // The refused ones are the newest, so only their refusal keeps them out.
    let refused = [
        ("github:kestrel-co/a tool", "kestrel-co/a tool", 3),
        ("github:kestrel-co/moved", "osprey/moved", 7),
        ("github:kestrel-co/other", "kestrel-co/other", 0),
        ("github:kestrel-co/-tool", "kestrel-co/-tool", 4),
        ("github:kestrel-co/..", "kestrel-co/..", 5),
    ];
    let mut changes: Vec<(&str, Vec<Change>)> = refused
        .iter()
        .map(|&(key, repo, number)| {
            (
                key,
                vec![numbered(repo, number, change("refused", NOW, &["jsama"]))],
            )
        })
        .collect();
    let mut issue = numbered("kestrel-co/other", 8, change("an issue", NOW, &["jsama"]));
    if let Some(github) = issue.github.as_mut() {
        github.url = "https://github.com/kestrel-co/other/issues/8".to_owned();
    }
    changes[2].1.push(issue);
    // Links that aren't this pull request's own: another number, and a
    // page beneath it.
    for (number, url) in [
        (6, "https://github.com/kestrel-co/other/pull/60"),
        (2, "https://github.com/kestrel-co/other/pull/2/files"),
    ] {
        let mut elsewhere = numbered("kestrel-co/other", number, change("else", NOW, &["jsama"]));
        if let Some(github) = elsewhere.github.as_mut() {
            github.url = url.to_owned();
        }
        changes[2].1.push(elsewhere);
    }
    changes.push((
        "github:kestrel-co/tool",
        (1..=250)
            .map(|number| {
                numbered(
                    "kestrel-co/tool",
                    number,
                    change("many", NOW - number, &["jsama"]),
                )
            })
            .collect(),
    ));
    let seen = seen(&changes, &[]);
    let asked = asked(&seen, &Owners::of(DECLARED, "work"), &login());
    assert_eq!(
        asked.len(),
        MOST_ITEMS,
        "more were named than mycelium keeps"
    );
    assert_eq!(
        (
            asked.first().map(String::as_str),
            asked.last().map(String::as_str)
        ),
        (Some("kestrel-co/tool#1"), Some("kestrel-co/tool#200")),
        "the newest weren't the ones named"
    );
}
