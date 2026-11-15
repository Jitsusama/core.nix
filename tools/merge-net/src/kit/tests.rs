//! What the kit's queue decides, case by case.

use super::*;

const KIT_CHANGE: Role = Role::Kit;
const READER: Role = Role::Reader { owed: false };
const OWED: Role = Role::Reader { owed: true };

fn gate(pid: u32, started: u64) -> Gate {
    Gate {
        pid,
        ticks: 1,
        commit: format!("{pid:040}"),
        folder: PathBuf::from(format!("/tree-{pid}")),
        started,
    }
}

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// Gate 1 changes the kit, gate 2 is a reader, gate 3 an owed reader, and
/// gate 4 is apart.
fn roles(gate: &Gate) -> Role {
    match gate.pid {
        1 => KIT_CHANGE,
        2 => READER,
        3 => OWED,
        _ => Role::Apart,
    }
}

fn crates() -> Crates {
    Crates::of(
        "c:modules/terminal-kit/Cargo.toml:1:[package]\n\
         c:modules/cadence/Cargo.toml:1:[package]\n\
         c:modules/cadence/Cargo.toml:14:terminal-kit = { path = \"../terminal-kit\" }\n\
         c:modules/sun/Cargo.toml:1:[package]\n",
    )
}

#[test]
fn a_change_is_to_the_kit_a_reader_or_apart_by_its_files() {
    let crates = crates();
    let kit = owned(&[
        "modules/terminal-kit/src/lib.rs",
        "modules/cadence/src/hour.rs",
    ]);
    assert_eq!(
        role(&kit, &crates, false),
        KIT_CHANGE,
        "a change to the kit wasn't one"
    );
    let reader = owned(&["modules/cadence/src/hour.rs", "docs/commands.md"]);
    assert_eq!(role(&reader, &crates, false), READER, "a reader wasn't one");
    assert_eq!(
        role(&reader, &crates, true),
        OWED,
        "an owed reader wasn't owed"
    );
    let apart = owned(&["modules/sun/src/main.rs", "modules/terminal-kit.md"]);
    assert_eq!(
        role(&apart, &crates, false),
        Role::Apart,
        "a change apart was in the queue"
    );
}

#[test]
fn a_reader_waits_for_a_kit_gate_started_after_its_own() {
    // #4749 read the kit and merged at 12:22 on 15 November while #4796,
    // a change to the kit, ran again, sending it round a third time.
    let going = [gate(4, 50), gate(2, 100), gate(1, 300)];
    assert_eq!(
        behind(READER, &going, &roles).map(|held| held.pid),
        Some(1),
        "a reader went ahead of a change to the kit that started after it"
    );
    assert!(
        !by_start(READER, KIT_CHANGE) && !by_start(KIT_CHANGE, READER),
        "the order gates started in still ordered the kit and a reader"
    );
}

#[test]
fn changes_to_the_kit_go_first_come_first_served_among_themselves() {
    let going = [gate(1, 100), gate(2, 50)];
    assert_eq!(
        behind(KIT_CHANGE, &going, &roles),
        None,
        "a change to the kit waited for another or a reader not owed"
    );
    assert!(
        by_start(KIT_CHANGE, KIT_CHANGE),
        "changes to the kit weren't ordered by when they started"
    );
    assert!(
        by_start(Role::Apart, KIT_CHANGE) && by_start(READER, READER),
        "changes outside the queue weren't ordered by when they started"
    );
}

#[test]
fn a_reader_waits_for_at_most_one_change_to_the_kit() {
    // With six changes to the kit queued back to back, a reader waiting
    // for each would wait hours, so once the first it waited for merges,
    // the next waits for it instead.
    let mut owed = Owed::default();
    let first = "k1";
    let none_merged = |_: &str| false;
    let first_merged = |kit: &str| kit == first;
    owed.waits("reader", first, 100, &none_merged);
    assert!(
        !owed.owes("reader", 200, &none_merged),
        "a reader was owed its turn before the change it waited for merged"
    );
    assert!(
        owed.owes("reader", 200, &first_merged),
        "a reader wasn't owed its turn once the change it waited for merged"
    );
    owed.waits("reader", "k2", 300, &first_merged);
    assert!(
        owed.owes("reader", 400, &first_merged),
        "a reader owed its turn was made to wait for a second change to the kit"
    );
    let going = [gate(1, 50), gate(3, 100)];
    assert_eq!(
        behind(OWED, &going, &roles),
        None,
        "an owed reader waited for the next change to the kit"
    );
    assert_eq!(
        behind(KIT_CHANGE, &going, &roles).map(|held| held.pid),
        Some(3),
        "the next change to the kit went ahead of a reader owed its turn"
    );
}

#[test]
fn a_wait_whose_change_has_not_merged_moves_to_the_next_and_lapses() {
    let mut owed = Owed::default();
    let none_merged = |_: &str| false;
    owed.waits("reader", "k1", 100, &none_merged);
    owed.waits("reader", "k2", 200, &none_merged);
    let second_merged = |kit: &str| kit == "k2";
    assert!(
        owed.owes("reader", 300, &second_merged),
        "a reader's wait didn't move to the change it waits for now"
    );
    assert!(
        !owed.owes("reader", 200 + KEPT, &second_merged),
        "a wait was kept past its keeping"
    );
    owed.tidy(200 + KEPT);
    assert_eq!(owed, Owed::default(), "a lapsed wait wasn't given up");
}
