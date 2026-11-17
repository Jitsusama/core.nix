//! Keeping a base: asked for alone, on a builder or beside a gate, what
//! its VM tests read, carried past a record or evaluated whole, and what a
//! gate judges from it.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::{
    ALONE, BASE, GATING, KEEPING, MOVED, READS, RECORDED, StandIn, TRACE, WORKING, alone_record,
    evaluated, kept_behind, text, traced_alone,
};

#[test]
fn asked_to_keep_the_base_it_keeps_it_and_evaluates_nothing_else() {
    let stand_in = StandIn::new("keeps");
    // The working tree has no checks to give, so evaluating it would fail.
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--keep-base", "--here"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    assert!(
        text(&ran.stderr).contains("kept x86_64-linux's checks at 0123456789ab"),
        "the keep didn't say which checks it kept: {}",
        text(&ran.stderr)
    );
    let kept = Path::new("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(
        stand_in.folder.join(kept).exists(),
        "the kept checks weren't written to {}",
        kept.display()
    );
    assert!(
        stand_in.path(READS).exists(),
        "a keep records what the VM tests read as it evaluates them, so every kept base has \
         its record"
    );
}

#[test]
fn a_keep_on_a_builder_traces_the_vm_tests_beside_the_other_checks() {
    // One half after the other, a keep on 12 November took 679 s, each
    // half waiting on its slowest chunk.
    let stand_in = StandIn::new("keeps-beside");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    // Each evaluation waits for another under way, so none goes alone.
    fs::write(stand_in.path("meet"), "").expect("written");

    // As nix-offload says it on a builder made for the run.
    let ran = stand_in.run_told(&KEEPING, &[("CHANGED_CHECKS_HERE", "1")]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(stand_in.path(READS).exists(), "the keep lost its record");
    let runs = stand_in.runs();
    assert_eq!(
        evaluated(runs.last().expect("a keep's line")),
        ["base", "reads", "alone"],
        "a keep's two halves are noted in order, whichever ends first"
    );
}

#[test]
fn a_keep_on_a_builder_traces_while_a_gate_holds_the_base_then_checks_it_against_the_gates() {
    // On 12 November three keeps waited 119 to 704 s on the base's lock
    // while a gate evaluated the same base, and only then began tracing.
    let stand_in = StandIn::new("keeps-while-locked");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    let cache = stand_in.path("cache/changed-checks");
    fs::create_dir_all(&cache).expect("made");
    let lock = fs::File::create(cache.join("0123456789abcdef0123-x86_64-linux.lock"))
        .expect("the base's lock");
    lock.lock().expect("held as a gate holds it");

    let keep = stand_in.started(&KEEPING, &[("CHANGED_CHECKS_HERE", "1")]);
    let began = (0..200).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        stand_in.path("traced").exists()
    });
    fs::write(cache.join("0123456789abcdef0123-x86_64-linux.json"), BASE).expect("written");
    drop(lock);
    let ran = keep.wait_with_output().expect("changed-checks runs");

    assert!(began, "the trace waited for the base's lock");
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        stand_in.path(READS).exists(),
        "a trace that matched the gate's derivations was dropped: {}",
        text(&ran.stderr)
    );
    let runs = stand_in.runs();
    assert_eq!(
        evaluated(runs.last().expect("a keep's line")),
        ["reads", "alone"],
        "the gate's derivations were evaluated again"
    );
}

#[test]
fn a_keep_clears_what_a_keep_stopped_while_tracing_left_behind() {
    // On 12 November a switch stopped a keep mid-trace, and its traces, 15
    // MB each, and its cache stayed on the builder.
    let stand_in = StandIn::new("reads-swept");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    let cache = stand_in.path("cache/changed-checks");
    fs::create_dir_all(cache.join("old-x86_64-linux.reads.cache/fetcher")).expect("made");
    let long_ago = std::time::SystemTime::now() - std::time::Duration::from_hours(3);
    let left = |name: &str| {
        let path = cache.join(name);
        fs::write(&path, "").expect("written");
        path
    };
    let (old, now) = (
        left("old-x86_64-linux.reads.trace3"),
        left("now-x86_64-linux.reads.trace0"),
    );
    // A copy from the other builder that a killed ssh cut short.
    let copying = left("old-x86_64-linux.reads.json.copying.4242");
    for path in [&old, &copying, &cache.join("old-x86_64-linux.reads.cache")] {
        fs::File::open(path)
            .and_then(|file| file.set_modified(long_ago))
            .expect("dated");
    }

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(!old.exists(), "a trace left three hours ago stayed");
    assert!(!copying.exists(), "a copy left three hours ago stayed");
    assert!(
        !cache.join("old-x86_64-linux.reads.cache").exists(),
        "a cache left three hours ago stayed"
    );
    assert!(
        now.exists(),
        "another keep's trace, still being written, went"
    );
}

#[test]
fn keeping_a_base_a_gate_kept_first_traces_only_its_vm_tests() {
    let stand_in = StandIn::new("reads-gated");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    fs::create_dir_all(stand_in.path("cache/changed-checks")).expect("made");
    let kept = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.json";
    fs::write(stand_in.path(kept), BASE).expect("written");

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        stand_in.path(READS).exists(),
        "a base kept from git alone got no record"
    );
    let runs = stand_in.runs();
    assert_eq!(evaluated(&runs[0]), ["reads", "alone"], "{}", runs[0]);
}

#[test]
fn a_keep_whose_trace_fails_keeps_the_base_its_vm_tests_evaluated_from_git() {
    let stand_in = StandIn::new("reads-fallback");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), "").expect("written");

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let kept = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.json";
    let kept: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(stand_in.path(kept)).expect("kept"))
            .expect("JSON");
    assert_eq!(kept["vm-x"], "v.drv", "the VM test went unkept: {kept}");
    let runs = stand_in.runs();
    assert_eq!(
        evaluated(&runs[0]),
        ["base", "base", "alone"],
        "the ordinary checks, then the VM tests from git: {}",
        runs[0]
    );
}

/// A stand-in whose base has been kept, with what its VM test read, and
/// whose working tree changes `diff`'s files.
fn kept_with_reads(test: &str, diff: &str) -> StandIn {
    let stand_in = StandIn::new(test);
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("diff"), diff).expect("written");
    stand_in
}

#[test]
fn keeping_a_base_keeps_what_its_vm_tests_read_and_how_long_reading_took() {
    let stand_in = kept_with_reads("reads-kept", "");

    let kept: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(stand_in.path(READS)).expect("kept"))
            .expect("JSON");
    assert_eq!(
        kept,
        serde_json::json!({
            "tree": "basetree",
            "read": ["flake.nix", "listed", "tests", "tests/x.nix"],
            "missing": [],
            "listed": ["listed"],
        })
    );
    let runs = stand_in.runs();
    assert_eq!(
        evaluated(&runs[0]),
        ["base", "reads", "alone"],
        "{}",
        runs[0]
    );
}

#[test]
fn a_gate_whose_change_reaches_nothing_the_vm_tests_read_keeps_their_base_derivations() {
    // On 12 November 95 of 152 gates that named anything named no VM test,
    // and each spent half its evaluating on them.
    // A listing gives names alone, so a file changed in a folder listed
    // moves nothing, nor one added in a folder that was there already.
    // Nor does a file removed from a folder looked up that stays.
    let stand_in = kept_with_reads(
        "reads-unreached",
        "M\ta\nA\tnew\nD\tgone\nM\tlisted/old.nix\nA\tlisted/kept/new.nix\nD\ttests/y.nix\n",
    );
    fs::create_dir_all(stand_in.path("repo/listed/kept")).expect("made");
    fs::create_dir_all(stand_in.path("repo/tests")).expect("made");
    fs::write(stand_in.path("based"), "listed/kept\ntests\n").expect("written");
    // The other checks are evaluated, so only the VM tests' record counts.
    fs::remove_file(stand_in.path(ALONE)).expect("each check's record removed");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nadded    new\nchanged  a\nremoved  gone\n",
        "the VM test is the base's, so unchanged"
    );
    assert!(
        text(&ran.stderr).contains(
            "the change reaches nothing x86_64-linux's 1 VM tests read at 0123456789ab, \
             so each keeps the base's derivation unevaluated"
        ),
        "{}",
        text(&ran.stderr)
    );
    let runs = stand_in.runs();
    let tree = &runs[1]["evaluated"][0];
    assert_eq!(tree["of"], "tree", "{}", runs[1]);
    assert_eq!(
        tree["chunks"].as_array().map(Vec::len),
        Some(1),
        "only ordinary: {tree}"
    );
}

#[test]
fn a_gate_whose_change_reaches_what_the_vm_tests_read_evaluates_them() {
    for diff in [
        "M\ttests/x.nix\n",
        "A\tlisted/new.nix\n",
        "A\tlisted/came/new.nix\n",
        "D\ttests/y.nix\n",
        "M\tflake.lock\n",
    ] {
        let stand_in = kept_with_reads("reads-reached", diff);
        fs::create_dir_all(stand_in.path("repo/listed/came")).expect("made");
        // tests/ went with tests/y.nix, its last file.
        fs::write(stand_in.path("based"), "tests\n").expect("written");

        let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert_eq!(
            text(&ran.stdout),
            "KIND     CHECK\nadded    new\nchanged  a\nchanged  vm-x\nremoved  gone\n",
            "{diff:?} reaches the VM test, so it was evaluated"
        );
    }
}

#[test]
fn a_record_made_from_another_tree_or_a_change_git_cant_name_evaluates_them() {
    for (test, odd) in [("reads-tree", "tree"), ("reads-rename", "diff")] {
        let stand_in = kept_with_reads(test, "M\ta\n");
        let said = if odd == "tree" {
            "othertree\n"
        } else {
            "R100\ta\tb\n"
        };
        fs::write(stand_in.path(odd), said).expect("written");

        let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert!(
            text(&ran.stdout).contains("changed  vm-x\n"),
            "{odd} was odd, yet the record was used"
        );
    }
}

#[test]
fn a_base_whose_trace_cant_be_read_or_whose_inputs_are_local_keeps_no_record() {
    let unread = "4242 chdir(\"SOURCE/tests\") = 0\n";
    let lock = r#"{"nodes": {"x": {"locked": {"type": "path", "path": "/x"}}}}"#;
    for (test, trace, lock) in [
        ("reads-unread", unread, None),
        ("reads-empty", "", None),
        ("reads-local", TRACE, Some(lock)),
    ] {
        let stand_in = StandIn::new(test);
        stand_in.checks("base", BASE);
        fs::write(stand_in.path("trace"), trace).expect("written");
        if let Some(lock) = lock {
            fs::create_dir_all(stand_in.path("source")).expect("made");
            fs::write(stand_in.path("source/flake.lock"), lock).expect("written");
        }

        let ran = stand_in.run(&KEEPING);

        assert!(ran.status.success(), "{test}: {}", text(&ran.stderr));
        assert!(
            text(&ran.stderr)
                .contains("couldn't record what x86_64-linux's VM tests read at 0123456789ab"),
            "{test}: {}",
            text(&ran.stderr)
        );
        assert!(!stand_in.path(READS).exists(), "{test} kept a record");
    }
}

/// Where another base's record of what each of its checks read is kept.
const ANOTHERS: &str = "cache/changed-checks/fedcba9876543210fedc-x86_64-linux.alone.reads.json";

/// Keep the stand-in's base with a record of what another base's checks
/// each read already kept, written `age` ago, and give the keep's run.
fn kept_after_another(test: &str, age: std::time::Duration) -> (StandIn, serde_json::Value) {
    let stand_in = traced_alone(test);
    let other = stand_in.path(ANOTHERS);
    fs::create_dir_all(other.parent().expect("its folder")).expect("made");
    fs::write(&other, "{}").expect("written");
    fs::File::options()
        .write(true)
        .open(&other)
        .expect("opened")
        .set_modified(std::time::SystemTime::now() - age)
        .expect("dated");
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let run = stand_in.runs().remove(0);
    (stand_in, run)
}

#[test]
fn a_keep_traces_each_check_alone_beside_a_fresh_record_it_cant_judge_from() {
    // On 13 November keeps skipped every head for 20 minutes after a
    // record, leaving gates to judge from one 25 merges behind.
    let (stand_in, run) = kept_after_another("alone-fresh", std::time::Duration::from_mins(5));

    assert!(
        stand_in.path(ALONE).exists(),
        "a keep traced no check because another base's record was five minutes old"
    );
    assert_eq!(evaluated(&run), ["base", "reads", "alone"], "{run}");
    assert_eq!(
        traced_alone_in(&run),
        [3, 1],
        "a record of a base off this one's main served the keep: {run}"
    );
}

/// How many checks each of a keep's chunks traced alone.
fn traced_alone_in(run: &serde_json::Value) -> Vec<u64> {
    chunks_of(run, "alone").expect("what was traced alone")
}

/// How many checks each chunk of a run's evaluation `of` asked of; none
/// when it made no such evaluation.
fn chunks_of(run: &serde_json::Value, of: &str) -> Option<Vec<u64>> {
    let evaluation = run["evaluated"]
        .as_array()?
        .iter()
        .find(|evaluation| evaluation["of"] == of)?;
    Some(
        evaluation["chunks"]
            .as_array()?
            .iter()
            .map(|chunk| chunk["checks"].as_u64().expect("a count"))
            .collect(),
    )
}

/// Keep the base once a head it came after was recorded, main having
/// changed only a.nix between them, and give the keep's run.
fn kept_past_a_record(
    test: &str,
    whole: Option<std::time::Duration>,
) -> (StandIn, serde_json::Value) {
    let stand_in = kept_behind(test, RECORDED, "M\ta.nix\n", "");
    let head = format!("cache/changed-checks/{RECORDED}-x86_64-linux.alone.reads.json");
    if let Some(age) = whole {
        let mut record = alone_record(&stand_in, &head);
        let at = std::time::SystemTime::now() - age;
        record["whole"] = serde_json::json!(
            at.duration_since(std::time::UNIX_EPOCH)
                .expect("after 1970")
                .as_secs()
        );
        fs::write(stand_in.path(&head), record.to_string()).expect("written");
    }
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let run = stand_in.runs().pop().expect("the keep's run");
    (stand_in, run)
}

#[test]
fn a_keep_past_a_record_traces_only_the_checks_main_reached_and_copies_the_rest() {
    let (stand_in, run) = kept_past_a_record("alone-copied", None);

    assert_eq!(
        traced_alone_in(&run),
        [1],
        "a keep traced checks main reached nothing of since the record: {run}"
    );
    let base = alone_record(&stand_in, ALONE);
    let head = alone_record(
        &stand_in,
        &format!("cache/changed-checks/{RECORDED}-x86_64-linux.alone.reads.json"),
    );
    assert_eq!(
        base["names"], head["names"],
        "the record named other checks than the base has"
    );
    let read = |record: &serde_json::Value, name: &str| -> BTreeSet<String> {
        let paths = record["paths"].as_array().expect("the paths read");
        record["checks"][name]["read"]
            .as_array()
            .unwrap_or_else(|| panic!("{name} wasn't recorded: {record}"))
            .iter()
            .map(|at| {
                let at = usize::try_from(at.as_u64().expect("a path's number")).expect("small");
                paths[at].as_str().expect("a path").to_owned()
            })
            .collect()
    };
    for name in ["a", "b", "gone", "vm-x"] {
        assert_eq!(
            read(&base, name),
            read(&head, name),
            "{name}'s reads weren't what the head's record held"
        );
    }
}

#[test]
fn a_keep_traces_every_check_again_once_its_record_was_last_traced_whole_hours_ago() {
    let (stand_in, run) =
        kept_past_a_record("alone-stale", Some(std::time::Duration::from_hours(5)));

    assert_eq!(
        traced_alone_in(&run),
        [3, 1],
        "a record copied for five hours was copied again: {run}"
    );
    let whole = alone_record(&stand_in, ALONE)["whole"]
        .as_u64()
        .expect("when traced whole");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    assert!(
        now - whole < 600,
        "a record traced whole didn't say it was traced now"
    );
}

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
    fs::create_dir_all(stand_in.path("cache/changed-checks")).expect("made");
    fs::write(
        stand_in.path(KEPT),
        r#"{"a": "a.drv", "b": "b7.drv", "gone": "g.drv", "vm-x": "v.drv"}"#,
    )
    .expect("kept as a gate would");

    assert_kept_whole(
        &stand_in,
        "evaluated whole at 0123456789ab, x86_64-linux's b came to other derivations than a \
         gate kept",
    );
    assert_eq!(
        kept_derivations(&stand_in),
        serde_json::from_str::<serde_json::Value>(BASE).expect("JSON"),
        "a whole keep left what a gate kept in place of what it evaluated"
    );
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

/// A stand-in whose base's checks each read every folder's checks.nix, as
/// checks.nix gathers them: a from x/checks.nix, b from y/checks.nix, and
/// gone from checks.nix itself; and whose change is `diff`.
fn gathered_alone(test: &str, diff: &str) -> StandIn {
    use std::fmt::Write as _;
    let stand_in = StandIn::new(test);
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    let files = [
        ("flake.nix", "{ }"),
        ("checks.nix", "folders = [ ./x/checks.nix ./y/checks.nix ];"),
        ("x/checks.nix", "{ }: { a = 1; }"),
        ("y/checks.nix", "{ }: { b = 1; }"),
    ];
    let mut trace = String::new();
    for (file, text) in files {
        let path = stand_in.path(&format!("source/{file}"));
        fs::create_dir_all(path.parent().expect("its folder")).expect("made");
        fs::write(path, text).expect("written");
        writeln!(
            trace,
            "openat(AT_FDCWD</tmp>, \"SOURCE/{file}\", O_RDONLY) = 7<SOURCE/{file}>"
        )
        .expect("written");
    }
    for name in ["a", "b", "gone"] {
        fs::write(stand_in.path(&format!("trace.{name}")), &trace).expect("written");
    }
    fs::write(
        stand_in.path("files.json"),
        r#"{"a": "x/checks.nix", "b": "y/checks.nix", "gone": "checks.nix", "vm-x": "checks.nix"}"#,
    )
    .expect("written");
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    stand_in.checks("working", MOVED);
    fs::write(stand_in.path("diff"), diff).expect("written");
    stand_in
}

#[test]
fn a_change_to_another_folders_checks_reaches_none_it_doesnt_give() {
    let stand_in = gathered_alone("gathered-other", "M\ty/checks.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  b\nchanged  gone\n",
        "y/checks.nix gives only b, and checks.nix gives gone, yet others were evaluated again"
    );
}

#[test]
fn a_change_to_a_checks_own_file_reaches_it() {
    let stand_in = gathered_alone("gathered-own", "M\tx/checks.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  a\nchanged  gone\n",
        "x/checks.nix gives a, and checks.nix gives gone, so both are evaluated again"
    );
}

#[test]
fn a_check_whose_derivation_takes_the_whole_source_is_evaluated_by_every_gate() {
    let stand_in = traced_alone("alone-whole");
    fs::write(stand_in.path("referrers"), "SOURCE\nb.drv\n").expect("written");
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        stand_in.path(ALONE).exists(),
        "the other checks' reads weren't recorded: {}",
        text(&ran.stderr)
    );
    stand_in.checks("working", MOVED);
    fs::write(stand_in.path("diff"), "M\tdocs/notes.md\n").expect("written");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  b\n",
        "a check whose build may read any file kept the base's derivation"
    );
}

// `src = ../..` in a folder's checks.nix is the source's root, which Nix
// copies to a path of its own named after the source's, reading no file
// a trace sees: every-long-file-says-why's derivation took it so on 13
// November and kept the base's derivation through a change to a file.
#[test]
fn a_check_whose_derivation_takes_a_copy_of_the_whole_source_is_evaluated_by_every_gate() {
    let stand_in = traced_alone("alone-copy");
    fs::write(stand_in.path("copy"), "").expect("written");
    fs::write(stand_in.path("copy-referrers"), "b.drv\n").expect("written");
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    stand_in.checks("working", MOVED);
    fs::write(stand_in.path("diff"), "M\tdocs/notes.md\n").expect("written");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  b\n",
        "a check taking a copy of the source kept the base's derivation"
    );
}

#[test]
fn a_vm_test_whose_derivation_takes_the_whole_source_leaves_no_record() {
    let stand_in = StandIn::new("vm-whole");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    fs::write(stand_in.path("referrers"), "SOURCE\nv.drv\n").expect("written");

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        !stand_in.path(READS).exists(),
        "what the VM tests read was kept though one's build may read any file"
    );
}
