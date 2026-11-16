//! What each ordinary check and VM test read alone, kept with the base, and
//! how a gate judges each check by it: added, removed, or past a head.

use std::fs;
use std::process::Command;

use super::{
    ALONE, GATING, KEEPING, MOVED, READS, RECORDED, StandIn, TRACE, alone_record, evaluated,
    filed_as, kept_alone, kept_behind, text, traced_alone,
};

#[test]
fn keeping_a_base_keeps_what_each_of_its_checks_read_alone() {
    let stand_in = kept_alone("alone-kept", MOVED, "");

    let kept: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(stand_in.path(ALONE)).expect("kept"))
            .expect("JSON");
    assert_eq!(
        kept["names"],
        serde_json::json!(["a", "b", "gone", "vm-x"]),
        "the record names every check the base has"
    );
    let paths = kept["paths"].as_array().expect("the paths read");
    let read = |name: &str| -> Vec<&str> {
        kept["checks"][name]["read"]
            .as_array()
            .expect("what it read")
            .iter()
            .map(|at| {
                paths[usize::try_from(at.as_u64().expect("a path's number")).expect("small")]
                    .as_str()
                    .expect("a path")
            })
            .collect()
    };
    assert_eq!(
        read("b"),
        ["b.nix", "flake.nix"],
        "b read its own file: {kept}"
    );
    assert!(
        read("vm-x").contains(&"tests/x.nix"),
        "the VM test's reads alone were kept beside the others': {kept}"
    );
    let runs = stand_in.runs();
    assert_eq!(
        evaluated(&runs[0]),
        ["base", "reads", "alone"],
        "{}",
        runs[0]
    );
    let traced: Vec<&serde_json::Value> = runs[0]["evaluated"][2]["chunks"]
        .as_array()
        .expect("what was traced alone")
        .iter()
        .map(|chunk| &chunk["checks"])
        .collect();
    assert_eq!(
        traced,
        [3, 1],
        "the other checks weren't traced before the VM test: {}",
        runs[0]
    );
}

#[test]
fn a_keep_beside_a_gate_traces_its_vm_tests_alone_one_at_a_time() {
    // On 13 November every live keep began its VM tests as a gate ran, so
    // traced none, and copying a record forward carried none on.
    let stand_in = traced_alone("vm-alone-gating");
    let gate = gate_held(&stand_in);

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let kept: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(stand_in.path(ALONE)).expect("kept"))
            .expect("JSON");
    assert!(!kept["checks"]["a"].is_null(), "{kept}");
    assert!(
        !kept["checks"]["vm-x"].is_null(),
        "a gate running beside the keep left the VM test untraced: {kept}"
    );
    drop(gate);
}

#[test]
fn a_gate_gives_each_check_its_change_reaches_nothing_of_the_bases_derivation() {
    // On 12 November a gate naming about 20 checks still spent 300 to 900
    // s evaluating all of the tree's.
    let stand_in = kept_alone("alone-untouched", MOVED, "M\tdocs/notes.md\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "",
        "a check that read nothing the change touched was evaluated again"
    );
    assert!(
        text(&ran.stderr).contains(
            "evaluates 0 of x86_64-linux's 3 other checks, since the change reaches nothing \
             the other 3 read at 0123456789ab, which keep their derivations there"
        ),
        "{}",
        text(&ran.stderr)
    );
}

#[test]
fn a_gate_evaluates_each_check_whose_read_file_its_change_moved() {
    let stand_in = kept_alone("alone-touched", MOVED, "M\ta.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  a\n",
        "only a read a.nix, so only a was evaluated again"
    );
    assert!(
        text(&ran.stderr).contains("evaluates 1 of x86_64-linux's 3 other checks"),
        "the gate didn't say how many it evaluated: {}",
        text(&ran.stderr)
    );
    let run = stand_in.runs().pop().expect("the gate's run");
    assert_eq!(
        run["recorded"],
        serde_json::json!({ "head": "0123456789abcdef0123", "main": 0, "tree": 1 }),
        "the run didn't say the record it judged from and what moved since: {run}"
    );
}

#[test]
fn a_gate_on_a_base_with_no_record_of_each_checks_reads_evaluates_them_all() {
    let stand_in = kept_alone("alone-unkept", MOVED, "M\tdocs/notes.md\n");
    let _ = fs::remove_file(stand_in.path(ALONE));

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  a\nchanged  b\nchanged  gone\n",
        "with no record, a check kept the base's derivation"
    );
    let run = stand_in.runs().pop().expect("the gate's run");
    assert_eq!(run["recorded"], serde_json::Value::Null, "{run}");
}

#[test]
fn a_gate_on_a_base_past_a_recorded_head_evaluates_there_only_what_main_reached() {
    let stand_in = kept_behind("alone-behind", RECORDED, "M\ta.nix\n", "M\ta.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  a\n",
        "a check neither main nor the change reached was evaluated again"
    );
    assert!(
        text(&ran.stderr).contains(
            "evaluates 1 of x86_64-linux's 3 other checks at the base 0123456789ab, since main \
             from 9876543210fe reaches nothing the other 2 read"
        ),
        "the gate didn't say what it evaluated at the base: {}",
        text(&ran.stderr)
    );
    // On 13 November gates evaluated 425 to 890 checks, and their lines
    // said neither the record they judged from nor how far main had moved.
    let run = stand_in.runs().pop().expect("the gate's run");
    assert_eq!(
        run["recorded"],
        serde_json::json!({ "head": RECORDED, "main": 1, "tree": 1 }),
        "the run didn't say the record it judged from and what moved since: {run}"
    );
}

/// What the base's VM test read alone: its flake and its own file, where
/// the VM tests together read `TRACE`'s files besides.
const VM_ALONE: &str = concat!(
    "openat(AT_FDCWD</tmp>, \"SOURCE/flake.nix\", O_RDONLY) = 7<SOURCE/flake.nix>\n",
    "openat(AT_FDCWD</tmp>, \"SOURCE/vm-x.nix\", O_RDONLY) = 8<SOURCE/vm-x.nix>\n",
);

/// What the base's other VM test read alone: its flake and tests/x.nix.
const VM_Y_ALONE: &str = concat!(
    "openat(AT_FDCWD</tmp>, \"SOURCE/flake.nix\", O_RDONLY) = 7<SOURCE/flake.nix>\n",
    "openat(AT_FDCWD</tmp>, \"SOURCE/tests/x.nix\", O_RDONLY) = 8<SOURCE/tests/x.nix>\n",
);

/// The base's checks with two VM tests.
const VM_BASE: &str =
    r#"{"a": "a.drv", "b": "b.drv", "gone": "g.drv", "vm-x": "v.drv", "vm-y": "w.drv"}"#;

/// The base's checks as the tree has them, only the VM tests moved.
const VM_MOVED: &str =
    r#"{"a": "a.drv", "b": "b.drv", "gone": "g.drv", "vm-x": "v2.drv", "vm-y": "w2.drv"}"#;

/// A stand-in whose base is kept with what each check read alone, vm-x
/// reading its own file, vm-y tests/x.nix, and the two together `TRACE`'s
/// too; whose tree moves only the VM tests, and whose change is `diff`.
fn vm_alone(test: &str, diff: &str) -> StandIn {
    let stand_in = vm_traced_alone(test);
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    stand_in.checks("working", VM_MOVED);
    fs::write(stand_in.path("diff"), diff).expect("written");
    stand_in
}

/// A stand-in whose base has two VM tests, each traced alone as
/// `vm_alone` says.
fn vm_traced_alone(test: &str) -> StandIn {
    let stand_in = traced_alone(test);
    stand_in.checks("base", VM_BASE);
    fs::write(stand_in.path("trace"), format!("{TRACE}{VM_ALONE}")).expect("written");
    fs::write(stand_in.path("trace.vm-x"), VM_ALONE).expect("written");
    fs::write(stand_in.path("trace.vm-y"), VM_Y_ALONE).expect("written");
    stand_in
}

/// Hold the stand-in's gating lock, shared, as a running gate holds it.
fn gate_held(stand_in: &StandIn) -> fs::File {
    let lock = stand_in.path("cache/changed-checks/gating.lock");
    fs::create_dir_all(lock.parent().expect("its folder")).expect("made");
    let gate = fs::File::create(&lock).expect("the gate's lock");
    gate.lock_shared().expect("held as a gate holds it");
    gate
}

#[test]
fn a_keep_beside_a_gate_keeps_the_record_after_each_vm_test_it_traces() {
    // A keep stopped part way, as a switch stops one, keeps what it traced.
    let stand_in = vm_traced_alone("vm-alone-each");
    let gate = gate_held(&stand_in);
    let holds = stand_in.path("holds.vm-y");
    let made = Command::new("mkfifo")
        .arg(&holds)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "no pipe to hold vm-y's trace on");

    // One at a time, so vm-x's trace is done before vm-y's begins.
    let keep = stand_in.started(&KEEPING, &[("CHANGED_CHECKS_AT_ONCE_BESIDE_A_GATE", "1")]);
    // Open once vm-y's trace waits on it, so vm-x's is done.
    let holding = fs::File::options()
        .write(true)
        .open(&holds)
        .expect("opened");
    let kept = alone_record(&stand_in, ALONE);
    drop(holding);
    let ran = keep.wait_with_output().expect("changed-checks runs");

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        !kept["checks"]["vm-x"].is_null(),
        "vm-x's reads weren't kept before vm-y's trace began: {kept}"
    );
    assert!(kept["checks"]["vm-y"].is_null(), "{kept}");
    assert!(kept["waiting"]["vm-y"].is_u64(), "{kept}");
    let kept = alone_record(&stand_in, ALONE);
    assert!(!kept["checks"]["vm-y"].is_null(), "{kept}");
    assert_eq!(kept["waiting"], serde_json::json!({}), "{kept}");
    drop(gate);
}

#[test]
fn a_keep_carrying_past_a_vm_test_main_reached_keeps_none_of_its_old_reads() {
    // Without the joint trace a carried VM test is judged by what it read
    // alone, so old reads kept for one main reached would let a gate skip it.
    let stand_in = vm_traced_alone("carry-vm-reached");
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    filed_as(&stand_in, RECORDED);
    fs::write(stand_in.path("diff.between"), "M\tvm-x.nix\n").expect("written");
    // Main's change has vm-x read a file it didn't.
    let reads_anew =
        "openat(AT_FDCWD</tmp>, \"SOURCE/vm-x-new.nix\", O_RDONLY) = 9<SOURCE/vm-x-new.nix>\n";
    fs::write(
        stand_in.path("trace.vm-x"),
        format!("{VM_ALONE}{reads_anew}"),
    )
    .expect("written");

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("carries from 9876543210fe"),
        "the keep didn't carry: {}",
        text(&ran.stderr)
    );
    let kept = alone_record(&stand_in, ALONE);
    let paths = kept["paths"].as_array().expect("the paths read");
    let anew = paths
        .iter()
        .position(|path| path == "vm-x-new.nix")
        .unwrap_or_else(|| panic!("vm-x kept what it read before main reached it: {kept}"));
    assert!(
        kept["checks"]["vm-x"]["read"]
            .as_array()
            .is_some_and(|read| read.contains(&serde_json::json!(anew))),
        "vm-x kept what it read before main reached it: {kept}"
    );
    assert!(
        !kept["checks"]["vm-y"].is_null(),
        "vm-y's reads, which main reached nothing of, weren't carried: {kept}"
    );
    assert!(
        !stand_in.path(READS).exists(),
        "what the VM tests read together was kept, though main reached it"
    );

    // A gate there judges the VM tests by what each read alone.
    stand_in.checks("working", VM_MOVED);
    fs::write(stand_in.path("diff"), "M\ttests/x.nix\n").expect("written");
    let ran = stand_in.run(&GATING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  vm-y\n",
        "a gate at a base without the joint trace evaluated a VM test it didn't reach"
    );
    assert!(
        text(&ran.stderr).contains(
            "evaluates 1 of x86_64-linux's 2 VM tests, since the change reaches nothing \
             the other 1 read at 0123456789ab"
        ),
        "{}",
        text(&ran.stderr)
    );
}

#[test]
fn keeps_beside_a_gate_past_their_bound_trace_the_vm_tests_left_longest_first() {
    // On 13 November gates judged from records 50 to 160 files behind
    // main, so beside a gate a keep begins no VM trace past a bound but
    // its first; were what it left not first next time, a busy main would
    // spend each keep on the same few.
    let stand_in = vm_traced_alone("vm-alone-bounded");
    let gate = gate_held(&stand_in);
    let bounded = [("CHANGED_CHECKS_BESIDE_A_GATE", "0")];

    let first = stand_in.run_told(&KEEPING, &bounded);

    assert!(first.status.success(), "{}", text(&first.stderr));
    let kept = alone_record(&stand_in, ALONE);
    assert!(!kept["checks"]["vm-x"].is_null(), "{kept}");
    assert!(
        kept["checks"]["vm-y"].is_null(),
        "a keep past its bound began another VM test's trace: {kept}"
    );
    assert!(
        kept["waiting"]["vm-y"].is_u64(),
        "the record didn't say vm-y waits: {kept}"
    );
    assert!(
        text(&first.stderr).contains(
            "traced 1 of the 2 VM tests x86_64-linux had to trace alone at 0123456789ab, \
             leaving 1 for a later keep"
        ),
        "{}",
        text(&first.stderr)
    );

    // Main moved what vm-x read, so the next keep has both to trace.
    filed_as(&stand_in, RECORDED);
    fs::write(stand_in.path("diff.between"), "M\tvm-x.nix\n").expect("written");
    let second = stand_in.run_told(&KEEPING, &bounded);

    assert!(second.status.success(), "{}", text(&second.stderr));
    let kept = alone_record(&stand_in, ALONE);
    assert!(
        !kept["checks"]["vm-y"].is_null(),
        "the keep traced what main reached before what a keep left: {kept}"
    );
    assert!(kept["checks"]["vm-x"].is_null(), "{kept}");
    assert!(kept["waiting"]["vm-x"].is_u64(), "{kept}");
    drop(gate);
}

#[test]
fn a_gate_reaching_the_vm_tests_but_nothing_one_read_alone_keeps_its_derivation() {
    // On 13 November 46 of 49 gates evaluated all 101 VM tests, though of
    // 12 traced alone, 104 of the last 300 merges reached none.
    let stand_in = vm_alone("vm-alone-unreached", "M\ttests/x.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  vm-y\n",
        "only vm-y read what the change touched, yet vm-x was evaluated too"
    );
    assert!(
        text(&ran.stderr).contains(
            "evaluates 1 of x86_64-linux's 2 VM tests, since the change reaches nothing \
             the other 1 read at 0123456789ab"
        ),
        "{}",
        text(&ran.stderr)
    );
}

#[test]
fn asked_only_to_name_them_it_judges_each_check_by_what_it_read_as_a_gate_does() {
    // On 14 November each of the merge net's rounds, which only names what
    // moved, evaluated all 930 of the tree's checks in 275 to 390 s before
    // its first VM test, while gates beside them evaluated 100 to 600.
    let naming = ["--system", "x86_64-linux", "--name-only"];
    let ordinary = kept_alone("named-alone-untouched", MOVED, "M\tdocs/notes.md\n");
    let vm = vm_alone("named-vm-alone-unreached", "M\ttests/x.nix\n");

    let (ran_ordinary, ran_vm) = (ordinary.run(&naming), vm.run(&naming));

    assert!(
        ran_ordinary.status.success(),
        "{}",
        text(&ran_ordinary.stderr)
    );
    assert!(
        text(&ran_ordinary.stderr).contains(
            "evaluates 0 of x86_64-linux's 3 other checks, since the change reaches nothing \
             the other 3 read at 0123456789ab"
        ),
        "naming them evaluated the other checks the change reaches nothing of: {}",
        text(&ran_ordinary.stderr)
    );
    assert_eq!(
        text(&ran_vm.stdout),
        "KIND     CHECK\nchanged  vm-y\n",
        "only vm-y read what the change touched, yet naming them named more"
    );
    assert!(
        text(&ran_vm.stderr).contains("evaluates 1 of x86_64-linux's 2 VM tests"),
        "naming them evaluated the VM tests the change reaches nothing of: {}",
        text(&ran_vm.stderr)
    );
}

#[test]
fn a_gate_evaluates_each_vm_test_its_change_reaches_or_the_record_lacks() {
    for (test, diff, lacks, moved) in [
        (
            "vm-alone-reached",
            "M\tvm-x.nix\n",
            false,
            "KIND     CHECK\nchanged  vm-x\n",
        ),
        (
            "vm-alone-all-read",
            "M\tflake.nix\n",
            false,
            "KIND     CHECK\nchanged  vm-x\nchanged  vm-y\n",
        ),
        (
            "vm-alone-lacked",
            "M\ttests/x.nix\n",
            true,
            "KIND     CHECK\nchanged  vm-x\nchanged  vm-y\n",
        ),
    ] {
        let stand_in = vm_alone(test, diff);
        if lacks {
            let mut kept: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(stand_in.path(ALONE)).expect("kept"))
                    .expect("JSON");
            kept["checks"]
                .as_object_mut()
                .expect("each check's reads")
                .remove("vm-x");
            fs::write(stand_in.path(ALONE), kept.to_string()).expect("written");
        }

        let ran = stand_in.run(&GATING);

        assert!(ran.status.success(), "{test}: {}", text(&ran.stderr));
        assert_eq!(
            text(&ran.stdout),
            moved,
            "{test}: a VM test the change reached, or the record lacked, kept its derivation"
        );
    }
}

#[test]
fn a_gate_on_a_base_past_a_head_whose_vm_test_main_reached_evaluates_it_at_the_base() {
    let stand_in = vm_alone("vm-alone-behind", "M\tvm-x.nix\n");
    filed_as(&stand_in, RECORDED);
    fs::write(stand_in.path("diff.between"), "M\tvm-x.nix\n").expect("written");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "KIND     CHECK\nchanged  vm-x\n");
    assert!(
        text(&ran.stderr).contains(
            "evaluates 1 of x86_64-linux's 2 VM tests at the base 0123456789ab, since main \
             from 9876543210fe reaches nothing the other 1 read"
        ),
        "the gate didn't evaluate at the base the VM test main reached: {}",
        text(&ran.stderr)
    );
}

#[test]
fn a_gate_on_a_base_past_a_head_recorded_off_its_main_evaluates_them_all() {
    let stand_in = kept_behind("alone-off-main", "5555555555555555aaaa", "", "M\ta.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nchanged  a\nchanged  b\nchanged  gone\n",
        "a check took the derivation of a head the base didn't come from"
    );
}

#[test]
fn a_gate_whose_tree_adds_a_check_evaluates_it_and_judges_the_rest_by_their_reads() {
    // Main went from 882 checks to 886 in an hour on 12 November, and a
    // record refused for naming others served 1 of 22 gates.
    let named_anew =
        r#"{"a": "a2.drv", "b": "b2.drv", "gone": "g2.drv", "new": "n.drv", "vm-x": "v.drv"}"#;
    let stand_in = kept_alone("alone-renamed", named_anew, "A\tnew.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND   CHECK\nadded  new\n",
        "a check whose reads the change reaches none of was evaluated again"
    );
    assert!(
        text(&ran.stderr).contains("evaluates 1 of x86_64-linux's 4 other checks"),
        "the gate didn't evaluate only the check the record lacks: {}",
        text(&ran.stderr)
    );
}

#[test]
fn a_gate_whose_tree_removes_a_check_judges_the_rest_by_their_reads() {
    let named_fewer = r#"{"a": "a2.drv", "b": "b2.drv", "vm-x": "v.drv"}"#;
    let stand_in = kept_alone("alone-removed", named_fewer, "D\tgone.nix\n");

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "KIND     CHECK\nremoved  gone\n",
        "a check whose reads the change reaches none of was evaluated again"
    );
    assert!(
        text(&ran.stderr).contains("evaluates 0 of x86_64-linux's 2 other checks"),
        "the gate didn't judge the checks left by their reads: {}",
        text(&ran.stderr)
    );
}

#[test]
fn a_gate_on_a_base_where_main_added_a_check_evaluates_there_only_it() {
    let stand_in = kept_behind(
        "alone-behind-added",
        RECORDED,
        "A\tnew.nix\n",
        "A\tnew.nix\n",
    );
    let added = r#"{"a": "a.drv", "b": "b.drv", "gone": "g.drv", "new": "n.drv", "vm-x": "v.drv"}"#;
    stand_in.checks("base", added);

    let ran = stand_in.run(&GATING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains(
            "evaluates 1 of x86_64-linux's 4 other checks at the base 0123456789ab, since main \
             from 9876543210fe reaches nothing the other 3 read"
        ),
        "the base was evaluated whole for a check main added: {}",
        text(&ran.stderr)
    );
}
