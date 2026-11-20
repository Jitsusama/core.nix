//! What waits on `joel` over the views' own record (views/tests.rs), at
//! every width: which changes lead, in what order, under which reason, and
//! the record a refresh keeps, so what's drawn is what `repos` reads.

use terminal_kit::Pen;
use terminal_kit::layout::{self, Room};

use repos::github::{self, Seen};
use repos::model::{ChecksState, Record, Review};

use crate::views::repositories;
use crate::views::tests::{HOME, NOW, ROOMS, change, clock, home, record, refreshed};

/// The refreshed record with changes waiting on `joel`: one asking his
/// review in core.nix, a newer one in walgit-rs, his own failing, his own
/// with changes asked, one of his own that waits on nobody, and someone
/// else's failing, which waits on him not at all. Only what a refresh
/// keeps is set, so no change holds the logins it asked.
fn waiting_on_joel() -> Record {
    let mut record = refreshed();
    let mut asked = change(
        4795,
        "someone/asks",
        ChecksState::Passing,
        Review::Waiting,
        5,
    );
    asked.asks_your_review = true;
    let mut failing = change(
        4793,
        "joel/fails",
        ChecksState::Failing,
        Review::ChangesAsked,
        60,
    );
    failing.author = Some("JOEL".to_owned());
    let mut asked_to_change = change(
        4792,
        "joel/changes",
        ChecksState::Passing,
        Review::ChangesAsked,
        30,
    );
    asked_to_change.author = Some("joel".to_owned());
    let mut fine = change(4796, "joel/fine", ChecksState::Passing, Review::Approved, 1);
    fine.author = Some("joel".to_owned());
    record.repositories[0]
        .changes
        .splice(0..0, [fine, asked, failing, asked_to_change]);
    let mut asked_elsewhere = change(
        4794,
        "someone/asks-too",
        ChecksState::Pending,
        Review::Waiting,
        2,
    );
    asked_elsewhere.asks_your_review = true;
    if let Some(github) = &mut asked_elsewhere.github {
        github.url = "https://github.com/Jitsusama/walgit-rs/pull/4794".to_owned();
    }
    record.repositories[1].changes = vec![asked_elsewhere];
    record
}

/// The rows of the table the collection leads with, each by its change.
fn for_you(text: &str) -> Vec<&str> {
    text.lines()
        .skip_while(|line| !(line.starts_with("CHANGE ") && line.contains("FOR YOU")))
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect()
}

/// The collection as `joel` reads it.
fn drawn(record: &Record, room: Room) -> String {
    repositories(
        record,
        &clock(),
        Some(home()),
        Some("joel"),
        &Pen::plain(),
        room,
    )
}

const LEADS: [&str; 4] = [
    "Jitsusama/walgit-rs#4794",
    "core.nix#4795",
    "core.nix#4792",
    "core.nix#4793",
];

#[test]
fn what_waits_on_the_login_leads_the_collection() {
    let record = waiting_on_joel();
    for room in ROOMS {
        let text = drawn(&record, room);
        assert_eq!(layout::overrun(&text, room), None, "{text}");
        assert_eq!(
            for_you(&text),
            LEADS,
            "reviews asked of him, then his own failing or asked to change, each newest first \
             across every repository: {text}"
        );
        let leads = text.find("FOR YOU").expect("it leads");
        assert!(
            leads < text.find("REPOSITORY").expect("the collection follows"),
            "{text}"
        );
        let why = |change: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(change))
                .map(str::trim_start)
                .unwrap_or_else(|| panic!("{change} has no row: {text}"))
        };
        assert!(why("core.nix#4795").starts_with("your review "), "{text}");
        assert!(why("core.nix#4792").starts_with("changes asked "), "{text}");
        assert!(
            why("core.nix#4793").starts_with("failing · changes asked "),
            "{text}"
        );
    }
    let text = drawn(&record, Room::Unbounded);
    assert!(
        text.contains("Change number 4795, which says what it does at some length"),
        "into a pipe a title is whole: {text}"
    );
}

#[test]
fn nothing_leads_where_nothing_waits_or_no_login_is_declared() {
    // Whose review a change asks was settled by the refresh, against the
    // same declared login, so only whose changes they are can differ here.
    let mut joels_own = waiting_on_joel();
    for repository in &mut joels_own.repositories {
        for change in &mut repository.changes {
            change.asks_your_review = false;
        }
    }
    for (record, login) in [
        (waiting_on_joel(), None),
        (joels_own, Some("someone-else")),
        (refreshed(), Some("joel")),
    ] {
        let text = repositories(
            &record,
            &clock(),
            Some(home()),
            login,
            &Pen::plain(),
            Room::Cells(80),
        );
        assert!(!text.contains("FOR YOU"), "login {login:?}: {text}");
    }
}

#[test]
fn the_collection_reads_whole_at_forty_and_names_each_change_whole() {
    let mut record = waiting_on_joel();
    record.roots.push(format!("{HOME}/world/trees"));
    let room = Room::Cells(40);
    let text = drawn(&record, room);
    assert_eq!(layout::overrun(&text, room), None, "{text}");
    assert_eq!(for_you(&text), LEADS, "{text}");
    assert!(text.contains("under ~/src and ~/world/trees"), "{text}");
}

#[test]
fn a_head_that_fits_stays_one_line() {
    let text = drawn(&waiting_on_joel(), Room::Cells(40));
    assert_eq!(
        text.lines().next(),
        Some("repositories · 4 on laptop · under ~/src"),
        "a head of exactly 40 cells was broken at 40: {text}"
    );
}

#[test]
fn a_change_two_clones_hold_leads_once() {
    let mut record = waiting_on_joel();
    let mut twin = record.repositories[0].clone();
    twin.key = "github:jitsusama/core.nix".to_owned();
    twin.owner = Some("jitsusama".to_owned());
    record.repositories.push(twin);
    let text = drawn(&record, Room::Unbounded);
    let rows = for_you(&text);
    assert_eq!(
        rows.iter().filter(|row| row.ends_with("#4795")).count(),
        1,
        "a remote typed in another case named one change twice: {text}"
    );
    assert_eq!(rows.len(), 4, "{text}");
}

#[test]
fn a_review_asked_of_the_login_leads_once_a_refresh_has_kept_it() {
    let mut asked = change(
        4795,
        "someone/asks",
        ChecksState::Passing,
        Review::Waiting,
        5,
    );
    asked.review_asked_of = vec!["Joel".to_owned(), "osprey".to_owned()];
    let mut asked_of_another = change(
        4794,
        "someone/asks-another",
        ChecksState::Passing,
        Review::Waiting,
        6,
    );
    asked_of_another.review_asked_of = vec!["osprey".to_owned()];
    let mut seen = Seen::new(NOW - 120);
    seen.changes.insert(
        "github:Jitsusama/core.nix".to_owned(),
        vec![asked, asked_of_another],
    );
    seen.mark_asked("joel");
    let folder = std::env::temp_dir().join(format!("repos-{}-asked-and-kept", std::process::id()));
    let file = folder.join("github.json");
    github::keep(&file, &seen).expect("it keeps");
    let written = std::fs::read_to_string(&file).expect("it reads");
    let _ = std::fs::remove_dir_all(&folder);
    assert!(
        !written.contains("Joel") && !written.contains("osprey"),
        "a login whose review was asked was kept: {written}"
    );
    let mut record = record();
    github::merge(
        &mut record,
        &serde_json::from_str(&written).expect("what's kept reads back"),
    );
    let text = drawn(&record, Room::Cells(80));
    assert_eq!(for_you(&text), ["core.nix#4795"], "{text}");
}

#[test]
fn a_change_named_by_its_branch_leads_by_it() {
    let mut record = waiting_on_joel();
    record.repositories[0].changes[1].github = None;
    let text = drawn(&record, Room::Unbounded);
    assert!(
        text.lines()
            .any(|line| line.starts_with("core.nix someone/asks ")),
        "{text}"
    );
}
