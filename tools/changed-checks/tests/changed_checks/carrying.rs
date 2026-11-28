//! Carrying a base past a record: a keep that evaluates only what main
//! reached since, counts the carry, and evaluates whole once its bound is
//! met, naming what a carry would have got wrong.

use std::fs;

use super::keeping::chunks_of;
use super::{
    ALONE, BASE, KEEPING, READS, RECORDED, StandIn, alone_record, kept_behind, text, traced_alone,
};

/// Where the base's derivations are kept.
const KEPT: &str = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.json";

/// A stand-in whose base came after a recorded head, main having changed
/// only a.nix between them, the head's keep having carried from a record
/// `carried` keeps in a row, and its record made `age` ago.
fn past_a_carry(test: &str, carried: u64, age: std::time::Duration) -> StandIn {
    let stand_in = kept_behind(test, RECORDED, "M\ta.nix\n", "");
    let head = format!("cache/changed-checks/{RECORDED}-x86_64-linux.alone.reads.json");
    let mut record = alone_record(&stand_in, &head);
    record["carried"] = serde_json::json!(carried);
    fs::write(stand_in.path(&head), record.to_string()).expect("written");
    fs::File::options()
        .write(true)
        .open(stand_in.path(&head))
        .expect("opened")
        .set_modified(std::time::SystemTime::now() - age)
        .expect("dated");
    stand_in
}

/// Keep the stand-in's base, giving the keep's run and what it said.
fn kept_now(stand_in: &StandIn) -> (serde_json::Value, String) {
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let run = stand_in.runs().pop().expect("the keep's run");
    (run, text(&ran.stderr))
}

/// What the keep kept as the base's derivations.
fn kept_derivations(stand_in: &StandIn) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(stand_in.path(KEPT)).expect("kept")).expect("JSON")
}

#[test]
fn a_keep_past_a_record_evaluates_only_the_checks_main_reached_and_carries_the_rest() {
    // On 13 November each keep evaluated its head whole, about 800
    // checks in 88 to 198 s and the VM tests in 149 to 495 s more, so
    // records trailed main by 50 to 160 files.
    let stand_in = past_a_carry("carry", 0, std::time::Duration::ZERO);

    let (run, said) = kept_now(&stand_in);

    assert_eq!(
        chunks_of(&run, "base").map(|chunks| chunks.iter().sum::<u64>()),
        Some(1),
        "a keep evaluated checks main reached nothing of since the record: {run}"
    );
    assert_eq!(
        chunks_of(&run, "reads"),
        None,
        "a keep traced the VM tests though main reached nothing they read: {run}"
    );
    assert_eq!(
        kept_derivations(&stand_in),
        serde_json::from_str::<serde_json::Value>(BASE).expect("JSON"),
        "what was carried isn't what evaluating whole gives"
    );
    assert!(
        stand_in.path(READS).exists(),
        "what the VM tests read wasn't carried, though main reached none of it"
    );
    assert_eq!(
        alone_record(&stand_in, ALONE)["carried"],
        1,
        "the record didn't count the carry"
    );
    assert!(
        said.contains("carries from 9876543210fe, the first keep in a row to"),
        "the keep didn't say it carried: {said}"
    );
    assert!(
        !stand_in.path(BASE_TREE).exists(),
        "a keep that carried kept its derivations by their tree as a correction"
    );
}

/// Where the stand-in's base keeps its derivations by its tree, which git
/// gives as `tree-of-<commit>`.
const BASE_TREE: &str = concat!(
    "cache/changed-checks/trees/",
    "tree-of-0123456789abcdef0123-x86_64-linux.json"
);

/// What a gate's carry kept: b at another derivation than the base's.
const GATE_KEPT: &str = r#"{"a": "a.drv", "b": "b7.drv", "gone": "g.drv", "vm-x": "v.drv"}"#;

#[test]
fn a_keep_that_finds_its_base_kept_by_its_tree_counts_a_carry_and_leaves_the_tree() {
    // What a tree kept may have been carried, by a gate's run past its
    // record, so a keep taking it evaluated nothing and corrects nothing.
    let stand_in = traced_alone("keep-seeded");
    fs::create_dir_all(stand_in.path("cache/changed-checks/trees")).expect("made");
    fs::write(stand_in.path(BASE_TREE), GATE_KEPT).expect("kept as a gate's tree would");

    let (run, _) = kept_now(&stand_in);

    assert_eq!(
        chunks_of(&run, "base"),
        None,
        "a keep evaluated a base its tree kept: {run}"
    );
    assert_eq!(
        alone_record(&stand_in, ALONE)["carried"],
        1,
        "a keep counted what its tree kept as evaluated whole"
    );
    assert_eq!(
        fs::read_to_string(stand_in.path(BASE_TREE)).expect("kept"),
        GATE_KEPT,
        "a keep that evaluated nothing rewrote what its tree kept"
    );
}

/// Assert a keep past `stand_in`'s record evaluated the base whole and
/// started the count of carries again, saying `why`.
fn assert_kept_whole(stand_in: &StandIn, why: &str) {
    let (run, said) = kept_now(stand_in);

    assert_eq!(
        chunks_of(&run, "base").map(|chunks| chunks.iter().sum::<u64>()),
        Some(3),
        "a keep past its bound carried derivations: {run}"
    );
    assert!(
        chunks_of(&run, "reads").is_some(),
        "a keep past its bound didn't trace the VM tests: {run}"
    );
    assert_eq!(
        alone_record(stand_in, ALONE)["carried"],
        0,
        "a whole keep's record counted a carry"
    );
    assert!(
        said.contains(why),
        "the keep didn't say why it went whole: {said}"
    );
}

#[test]
fn a_keep_evaluates_whole_once_six_keeps_in_a_row_carried() {
    // Each carry passes on a derivation never evaluated again, so a
    // wrong one would stand until a keep evaluates whole.
    let stand_in = past_a_carry("carry-six", 6, std::time::Duration::ZERO);

    assert_kept_whole(
        &stand_in,
        "evaluates whole, since 6 keeps in a row carried up to 9876543210fe",
    );
}

#[test]
fn a_keep_evaluates_whole_once_six_keeps_in_a_row_carried_though_a_gate_kept_its_base() {
    // On 14 November keeps at 06:02 and 06:18 said they evaluated whole,
    // but a gate had kept each base first, so they evaluated nothing and
    // counted a seventh and an eighth carry.
    let stand_in = past_a_carry("carry-six-gated", 6, std::time::Duration::ZERO);
    fs::create_dir_all(stand_in.path("cache/changed-checks/trees")).expect("made");
    fs::write(stand_in.path(KEPT), GATE_KEPT).expect("kept as a gate would");
    fs::write(stand_in.path(BASE_TREE), GATE_KEPT).expect("kept by its tree as a gate would");

    assert_kept_whole(
        &stand_in,
        "evaluated whole at 0123456789ab, x86_64-linux's b came to other derivations than \
         were kept before",
    );
    let base = serde_json::from_str::<serde_json::Value>(BASE).expect("JSON");
    assert_eq!(
        kept_derivations(&stand_in),
        base,
        "a whole keep left what a gate kept in place of what it evaluated"
    );
    let by_tree: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(stand_in.path(BASE_TREE)).expect("kept"))
            .expect("JSON");
    assert_eq!(
        by_tree, base,
        "a whole keep left what a gate kept by the tree in place of what it evaluated"
    );
}

#[test]
fn a_keep_past_its_bound_whose_base_a_gate_kept_as_it_began_counts_one_more_carry() {
    // A keep past its bound sets aside what a gate kept, then waits on the
    // base's lock; a gate holding it may keep the base again first, so
    // the keep evaluates nothing and mustn't end the bound.
    let stand_in = past_a_carry("carry-six-raced", 6, std::time::Duration::ZERO);
    fs::write(stand_in.path(KEPT), GATE_KEPT).expect("kept as a gate would");
    let lock =
        fs::File::create(stand_in.path(&KEPT.replace(".json", ".lock"))).expect("the base's lock");
    lock.lock().expect("held as a gate holds it");
    let aside = stand_in.path(&KEPT.replace(".json", ".gated"));

    let keep = stand_in.started(&KEEPING, &[]);
    let began = (0..200).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        aside.exists() && !stand_in.path(KEPT).exists()
    });
    fs::write(stand_in.path(KEPT), GATE_KEPT).expect("kept again as the gate would");
    drop(lock);
    let ran = keep.wait_with_output().expect("changed-checks runs");

    assert!(began, "the keep didn't set aside what the gate kept");
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let run = stand_in.runs().pop().expect("the keep's run");
    assert_eq!(
        chunks_of(&run, "base"),
        None,
        "a keep evaluated a base the gate kept: {run}"
    );
    assert_eq!(
        alone_record(&stand_in, ALONE)["carried"],
        7,
        "a keep that took the gate's derivations ended the bound on carries"
    );
    assert!(
        text(&ran.stderr).contains("a gate kept 0123456789ab while this keep began"),
        "the keep didn't say a gate kept its base: {}",
        text(&ran.stderr)
    );
    assert!(!aside.exists(), "what was set aside stayed");
}

#[test]
fn a_keep_evaluates_whole_once_the_record_it_would_carry_from_is_two_hours_old() {
    let stand_in = past_a_carry("carry-old", 1, std::time::Duration::from_hours(3));

    assert_kept_whole(
        &stand_in,
        "evaluates whole, since the record at 9876543210fe is over 2 hours old",
    );
}

#[test]
fn a_whole_keep_names_each_check_a_carry_would_have_given_another_derivation() {
    // A carry that went wrong shows only when a whole keep differs.
    let stand_in = past_a_carry("carry-drift", 6, std::time::Duration::ZERO);
    stand_in.checks(
        "base",
        r#"{"a": "a.drv", "b": "b9.drv", "gone": "g.drv", "vm-x": "v.drv"}"#,
    );

    let (_, said) = kept_now(&stand_in);

    assert!(
        said.contains(
            "evaluated whole at 0123456789ab, x86_64-linux's b came to other derivations \
             than carrying from 9876543210fe gave"
        ),
        "the keep didn't name the check a carry got wrong: {said}"
    );
}
