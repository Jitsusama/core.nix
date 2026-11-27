//! What turn decides, case by case.

use super::*;

const NOW: u64 = 10_000;

fn gate(pid: u32, folder: &str, started: u64) -> Gate {
    Gate {
        pid,
        ticks: 1,
        commit: format!("{pid:040}"),
        folder: PathBuf::from(folder),
        started,
    }
}

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn everyone(_: &Gate) -> bool {
    true
}

/// A side changing `changed` and renaming or deleting `gone`.
fn side<'a>(changed: &'a [String], gone: &'a [String]) -> Side<'a> {
    Side {
        changed,
        gone,
        text: &nothing,
    }
}

/// No file the two give apart.
fn none_apart(_: &str) -> bool {
    false
}

fn touching(files: &[&str]) -> impl Fn(&Gate) -> Option<Vec<String>> {
    let files = owned(files);
    move |_| Some(files.clone())
}

/// Each gate counted as started when it did, as with no places kept.
fn own(gate: &Gate) -> u64 {
    gate.started
}

fn asking<'a>(started: u64, here: &'a str, ours: &'a [String]) -> Asking<'a> {
    Asking {
        started,
        waits_from: NOW,
        here: Path::new(here),
        ours,
    }
}

#[test]
fn a_change_to_a_crate_reading_an_earlier_gates_crate_waits_behind_it() {
    // #4513 changed terminal-kit, which cadence and others read. From
    // 23:44 on 14 November six of its passes went stale, each as a
    // change to a crate reading the kit merged during its run, sharing
    // no file with it, so nothing held that change back.
    let crates = Crates::of(
        "c:modules/terminal-kit/Cargo.toml:1:[package]\n\
         c:modules/cadence/Cargo.toml:1:[package]\n\
         c:modules/cadence/Cargo.toml:14:terminal-kit = { path = \"../terminal-kit\" }\n\
         c:modules/sun/Cargo.toml:1:[package]\n",
    );
    let theirs = owned(&["modules/terminal-kit/src/lib.rs"]);
    let ours = owned(&["modules/cadence/src/hour.rs", "modules/sun/src/main.rs"]);
    let touch = |_: &Gate| {
        Some(shares(
            &side(&theirs, &[]),
            &none_apart,
            &side(&ours, &[]),
            &crates,
            &[],
        ))
    };
    let gates = [gate(1, "/kit", 100)];
    let held = ahead(
        &gates,
        &asking(200, "/cadence", &ours),
        NOW,
        &own,
        &everyone,
        &touch,
    );
    assert_eq!(
        held.map(|held| held.shared),
        Some(owned(&["modules/cadence/src/hour.rs"])),
        "a change to a crate reading the kit merged ahead of the kit's earlier gate"
    );
}

#[test]
fn a_pass_waits_on_an_earlier_gate_only_so_long() {
    let gates = [gate(1, "/a", 100)];
    let ours = owned(&["y.rs"]);
    let touch = touching(&["y.rs"]);
    let mut waiting = asking(200, "/b", &ours);
    waiting.waits_from = NOW - PATIENCE + 1;
    assert!(
        ahead(&gates, &waiting, NOW, &own, &everyone, &touch).is_some(),
        "a pass short of its patience went ahead of an earlier gate"
    );
    waiting.waits_from = NOW - PATIENCE;
    assert_eq!(
        ahead(&gates, &waiting, NOW, &own, &everyone, &touch),
        None,
        "a pass that waited its patience was still held back"
    );
}

#[test]
fn a_change_whose_gate_passed_keeps_its_turn_while_its_author_merges() {
    // On 14 November the tree at /a passed at 22:50:57, from a gate
    // started at 22:38:15, and /b's change, placed after it, asked 32
    // seconds later and merged first, leaving /a's pass stale again.
    let started = 1_794_713_895;
    let kept = 1_794_714_657;
    let asked = kept + 32;
    let trees = [(PathBuf::from("/a"), "9d0f6cb20".to_owned())];
    let passes = |commit: &str| (commit == "9d0f6cb20").then_some((started, kept));
    let unmerged = |_: &str| Some(false);
    let ours = owned(&["modules/quickshell/Launcher.qml"]);
    let touch = touching(&["modules/quickshell/Launcher.qml"]);
    let mut later = asking(started + 60, "/b", &ours);
    later.waits_from = asked - 60;
    let gates = finished(&trees, asked, &passes, &unmerged);
    let held = ahead(&gates, &later, asked, &own, &everyone, &touch);
    assert_eq!(
        held.map(|held| held.gate.folder.clone()),
        Some(PathBuf::from("/a")),
        "a change placed later merged ahead of one whose gate had just passed"
    );
    assert!(
        finished(&trees, kept + GRACE, &passes, &unmerged).is_empty(),
        "a pass left unmerged past its grace still held others back"
    );
    let merged = |_: &str| Some(true);
    assert!(
        finished(&trees, asked, &passes, &merged).is_empty(),
        "a pass main already holds still held others back"
    );
    let untold = |_: &str| None;
    assert!(
        finished(&trees, asked, &passes, &untold).is_empty(),
        "a pass main can't be told to hold or not held others back"
    );
    let never = |_: &str| None;
    assert!(
        finished(&trees, asked, &never, &unmerged).is_empty(),
        "a head that never passed held others back"
    );
}

#[test]
fn a_head_whose_change_main_took_rebased_counts_as_merged() {
    // On 15 November PN2VZC's tree moved on to new work from its
    // change's head before the rebase that merged as #4738, and that
    // head held #4763 back as a pass still to merge.
    use crate::tests::git_in;
    let repo = std::env::temp_dir().join(format!("merge-net-cherry-{}", std::process::id()));
    fs::create_dir_all(&repo).unwrap();
    git_in(&repo, &["init", "-q", "-b", "main"]);
    let write = |words: &str, message: &str| {
        fs::write(repo.join("w"), words).unwrap();
        git_in(&repo, &["add", "w"]);
        git_in(&repo, &["commit", "-qm", message]);
        git_in(&repo, &["rev-parse", "HEAD"])
    };
    let base = write("a\nb\nc\nd\ne\nf\ng\nh\n", "base");
    let change = write("a\nb\nc\nd\ne\nf\nG\nh\n", "change");
    git_in(&repo, &["checkout", "-q", &base]);
    write("a\nB\nc\nd\ne\nf\ng\nh\n", "moved");
    git_in(&repo, &["cherry-pick", &change]);
    let main = git_in(&repo, &["rev-parse", "HEAD"]);
    git_in(&repo, &["update-ref", "refs/remotes/origin/main", &main]);
    git_in(&repo, &["checkout", "-q", &base]);
    let other = write("a\nb\nc\nD\ne\nf\ng\nh\n", "other");
    let said = (on_main_in(&repo, &change), on_main_in(&repo, &other));
    fs::remove_dir_all(&repo).unwrap();
    assert_eq!(
        said,
        (Some(true), Some(false)),
        "a head main took rebased, or one it never took, was misjudged as merged or not"
    );
}

#[test]
fn a_change_in_an_earlier_gates_fate_waits_behind_it() {
    // Two changes in different files the map's check is built from each
    // fit its budget's room alone. Were the later to merge while the
    // earlier's gate ran, that pass would go stale on the map all the same.
    let fates = fates::parse("the map: modules/mycelium/site/ modules/local-sites/")
        .expect("the map's fate reads");
    let theirs = owned(&["modules/mycelium/site/view.js"]);
    let ours = owned(&["modules/local-sites/kit/head.css", "docs/testing.md"]);
    let crates = Crates::of("");
    let touch = |_: &Gate| {
        let (gate, merging) = (side(&theirs, &[]), side(&ours, &[]));
        Some(shares(&gate, &none_apart, &merging, &crates, &fates))
    };
    let gates = [gate(1, "/map", 100)];
    let held = ahead(
        &gates,
        &asking(200, "/kit", &ours),
        NOW,
        &own,
        &everyone,
        &touch,
    );
    assert_eq!(
        held.map(|held| held.shared),
        Some(owned(&["modules/local-sites/kit/head.css"])),
        "a change in the map's fate merged ahead of the map's earlier gate"
    );
    let apart = owned(&["modules/net/site/panel.ts"]);
    let touch_apart = |_: &Gate| {
        let (gate, merging) = (side(&theirs, &[]), side(&apart, &[]));
        Some(shares(&gate, &none_apart, &merging, &crates, &fates))
    };
    assert_eq!(
        ahead(
            &gates,
            &asking(200, "/net", &apart),
            NOW,
            &own,
            &everyone,
            &touch_apart,
        ),
        None,
        "a change outside the map's fate waited behind its gate"
    );
}

#[test]
fn a_gate_whose_only_fated_file_stands_apart_still_holds_its_fate() {
    // A pytest file the two give cases apart in won't make the gate's
    // pass stale as a file, but the fate's one check still measures it
    // beside ours, so the gate holds a change in the fate all the same.
    let fates = fates::parse("the map: modules/mycelium/site/ modules/local-sites/")
        .expect("the map's fate reads");
    let theirs = owned(&["modules/mycelium/site/tests/map-reads-live-test.py"]);
    let ours = owned(&["modules/local-sites/kit/head.css"]);
    let crates = Crates::of("");
    let apart = |file: &str| file.ends_with("-test.py");
    let touch = |_: &Gate| {
        let (gate, merging) = (side(&theirs, &[]), side(&ours, &[]));
        Some(shares(&gate, &apart, &merging, &crates, &fates))
    };
    let gates = [gate(1, "/map", 100)];
    let held = ahead(
        &gates,
        &asking(200, "/kit", &ours),
        NOW,
        &own,
        &everyone,
        &touch,
    );
    assert_eq!(
        held.map(|held| held.shared),
        Some(ours.clone()),
        "a gate whose only file in the map's fate stands apart held nothing"
    );
}

#[test]
fn a_gate_renaming_a_file_out_of_a_fate_holds_a_change_in_it() {
    // The fate measured the file the gate moved away, so a change in the
    // fate merging first would leave that measure stale.
    let fates = fates::parse("the map: modules/mycelium/site/ modules/local-sites/")
        .expect("the map's fate reads");
    let theirs = owned(&["modules/elsewhere/old-view.js"]);
    let moved = owned(&["modules/mycelium/site/old-view.js"]);
    let ours = owned(&["modules/local-sites/kit/head.css"]);
    let crates = Crates::of("");
    let touch = |_: &Gate| {
        let (gate, merging) = (side(&theirs, &moved), side(&ours, &[]));
        Some(shares(&gate, &none_apart, &merging, &crates, &fates))
    };
    let gates = [gate(1, "/moving", 100)];
    let held = ahead(
        &gates,
        &asking(200, "/kit", &ours),
        NOW,
        &own,
        &everyone,
        &touch,
    );
    assert_eq!(
        held.map(|held| held.shared),
        Some(ours.clone()),
        "a gate renaming a file out of the map's fate held nothing"
    );
}

#[test]
fn an_earlier_gate_sharing_a_file_holds_a_later_change_back() {
    let gates = [gate(1, "/a", 100)];
    let ours = owned(&["y.rs", "z.rs"]);
    let touch = touching(&["x.rs", "y.rs"]);
    let held = ahead(
        &gates,
        &asking(200, "/b", &ours),
        NOW,
        &own,
        &everyone,
        &touch,
    );
    assert_eq!(
        held,
        Some(Ahead {
            gate: &gates[0],
            shared: owned(&["y.rs"]),
            started: 100,
        }),
        "an earlier gate sharing y.rs didn't hold the change back"
    );
}

#[test]
fn a_later_gate_holds_an_earlier_change_back_never() {
    let gates = [gate(1, "/a", 300)];
    let ours = owned(&["y.rs"]);
    let touch = touching(&["y.rs"]);
    let held = ahead(
        &gates,
        &asking(200, "/b", &ours),
        NOW,
        &own,
        &everyone,
        &touch,
    );
    assert_eq!(held, None, "a gate started later held the change back");
}

#[test]
fn a_rerun_keeps_its_trees_place_ahead_of_gates_started_since_its_first() {
    // The tree at /a passed a gate at 100, a merge left it stale, and
    // it runs again from 300; /b's gate started at 200, between them.
    let mut places = Places::default();
    places.keep(Path::new("/a"), 100);
    let place = |gate: &Gate| places.start(&gate.folder, gate.started, NOW);
    let rerun = [gate(1, "/a", 300)];
    let ours = owned(&["y.rs"]);
    let touch = touching(&["y.rs"]);
    let held = ahead(
        &rerun,
        &asking(200, "/b", &ours),
        NOW,
        &place,
        &everyone,
        &touch,
    );
    assert_eq!(
        held.map(|held| held.started),
        Some(100),
        "a rerun went behind a gate started after its tree's first"
    );
    let first = [gate(2, "/c", 50)];
    let held = ahead(
        &first,
        &asking(100, "/a", &ours),
        NOW,
        &place,
        &everyone,
        &touch,
    );
    assert_eq!(
        held.map(|held| held.gate.pid),
        Some(2),
        "a rerun in its tree's place went ahead of a gate started before it"
    );
}

#[test]
fn a_gate_gone_here_too_long_or_sharing_only_lists_holds_nothing_back() {
    let ours = owned(&["y.rs", "check-descriptions.nix"]);
    let gone = [gate(1, "/a", 100)];
    let nobody = |_: &Gate| false;
    let touch = touching(&["y.rs"]);
    let at = asking(200, "/b", &ours);
    assert_eq!(
        ahead(&gone, &at, NOW, &own, &nobody, &touch),
        None,
        "a gate whose process is gone held the change back"
    );
    let here = [gate(1, "/b", 100)];
    assert_eq!(
        ahead(&here, &at, NOW, &own, &everyone, &touch),
        None,
        "a gate in the asking tree held its own change back"
    );
    let old = [gate(1, "/a", 100)];
    let later = 100 + LONGEST;
    let placed_early = |_: &Gate| 10;
    assert_eq!(
        ahead(&old, &at, later, &placed_early, &everyone, &touch),
        None,
        "a gate past the longest wait held the change back"
    );
    let lists = touching(&["check-descriptions.nix"]);
    assert_eq!(
        ahead(&old, &at, NOW, &own, &everyone, &lists),
        None,
        "a gate sharing only a list held the change back"
    );
}

#[test]
fn a_rebased_head_takes_the_start_of_the_pass_it_merges_on() {
    let kept = |started: u64| format!(r#"{{"run":["x"],"started":{started}}}"#);
    let record = |commit: &str| match commit {
        "newer" => kept(300),
        "older" => kept(100),
        _ => String::new(),
    };
    assert_eq!(
        start_of(&["head", "older"], &record),
        Some(100),
        "a rebased head with no run of its own took no start"
    );
    assert_eq!(start_of(&["newer", "older"], &record), Some(300));
    assert_eq!(start_of(&["head"], &record), None);
}

#[test]
fn a_gates_start_is_its_earliest_kept_and_none_before_starts_were() {
    let record = "{\"dir\":null,\"run\":[\"--build\"],\"printed\":\"\",\"started\":300}\n\
                  {\"dir\":null,\"check\":\"checks.x.a\"}\n\
                  {\"dir\":null,\"run\":[\"--build\",\"--no-vm\"],\"started\":200}\n\
                  {\"dir\":\"sub\",\"run\":[\"--build\"],\"started\":100}\n";
    assert_eq!(started(record), Some(200), "the wrong start was read");
    let old = "{\"dir\":null,\"run\":[\"--build\"],\"printed\":\"\"}\n";
    assert_eq!(started(old), None, "a record with no start gave one");
}

#[test]
fn a_pass_is_kept_when_its_run_ended_not_when_checks_were_added_to_its_record() {
    // As nix-offload writes them: two runs that built, the newest ending
    // second, then a naming run, a run keeping main's, a `there --run`
    // and a run in a folder below, each ending later and none a pass.
    let record = |_: &str| {
        "{\"dir\":null,\"run\":[\"--system\",\"x86_64-linux\"],\"started\":100,\"ended\":300}\n\
         {\"dir\":null,\"check\":\"checks.x.a\"}\n\
         {\"dir\":null,\"run\":[\"--build\",\"--no-vm\",\"--system\",\"x86_64-linux\"],\
         \"started\":200,\"ended\":400}\n\
         {\"dir\":null,\"run\":[\"--base\",\"m\",\"--name-only\",\"--system\",\"x86_64-linux\"],\
         \"started\":500,\"ended\":700}\n\
         {\"dir\":null,\"run\":[\"--keep-base\",\"--system\",\"aarch64-darwin\"],\
         \"started\":500,\"ended\":800}\n\
         {\"dir\":null,\"ran\":[\"nix\",\"eval\"],\"copies\":[],\"ended\":850}\n\
         {\"dir\":\"sub\",\"run\":[\"--system\",\"x86_64-linux\"],\"started\":500,\"ended\":900}\n\
         {\"dir\":null,\"check\":\"checks.x.vm-b\"}\n"
            .to_owned()
    };
    assert_eq!(
        kept_at(&["head"], &record, &|_| Some(1000)),
        Some(400),
        "a pass was kept from something other than when its newest run that built ended"
    );
}

#[test]
fn a_pass_from_before_runs_kept_their_end_is_kept_from_when_its_record_was_written() {
    let record =
        |_: &str| "{\"dir\":null,\"run\":[\"--system\",\"x\"],\"started\":100}\n".to_owned();
    assert_eq!(
        kept_at(&["head"], &record, &|_| Some(1234)),
        Some(1234),
        "a record that kept no end wasn't read by when it was written"
    );
}

#[test]
fn a_pass_ending_after_its_record_was_written_is_kept_from_when_it_was_written() {
    let record = |_: &str| {
        "{\"dir\":null,\"run\":[\"--system\",\"x\"],\"started\":100,\"ended\":5000}\n".to_owned()
    };
    assert_eq!(
        kept_at(&["head"], &record, &|_| Some(1000)),
        Some(1000),
        "an end no record could have been written by held the turn past it"
    );
}

// A process's start is read from /proc, which only Linux has: on a Mac the
// development shell builds merge-net too, and this test failed it there.
#[cfg(target_os = "linux")]
#[test]
fn a_gate_is_alive_only_while_a_process_started_when_it_said_has_its_id() {
    let pid = std::process::id();
    let ticks = ticks(pid).expect("this process's start is read");
    let mut running = gate(pid, "/a", 100);
    running.ticks = ticks;
    assert!(
        alive(&running),
        "a gate whose process runs was taken as ended"
    );
    running.ticks = ticks + 1;
    assert!(
        !alive(&running),
        "a gate was taken as running by another process given its id"
    );
    assert!(
        !alive(&gate(u32::MAX, "/a", 100)),
        "a gate with no process ran"
    );
}

#[test]
fn the_gates_going_are_read_beside_what_passed() {
    assert!(
        folder().is_some_and(|folder| folder.ends_with("nix-offload/running")),
        "the gates going were looked for outside nix-offload's state"
    );
}

#[test]
fn a_hold_kept_while_another_waits_on_the_lock_is_kept_too() {
    let folder = std::env::temp_dir().join(format!("merge-net-held-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("made");
    let file = folder.join("waited.json");
    let (first, second) = (Path::new("/first"), Path::new("/second"));
    // The first `ready` holds the lock and keeps its hold while the second
    // waits on it: the second must read the file only once it has the lock.
    // It joins only once the lock is let go, or the second would wait out
    // its bounded wait and go round the lock.
    let waiting = crate::whole::changed(&file, || {
        let theirs = file.clone();
        let waiting = std::thread::spawn(move || {
            keep_held(Some("second"), Path::new("/second"), &theirs, 1000);
        });
        std::thread::sleep(std::time::Duration::from_millis(200));
        let mut waited = Waited::default();
        waited.held(Some("first"), first, 990);
        waited.write(&file).expect("written");
        waiting
    });
    waiting.join().expect("the waiting ready ended");
    let waited = Waited::read(&file);
    let _ = fs::remove_dir_all(&folder);
    assert_eq!(
        waited.since(Some("first"), first, None, 1000),
        990,
        "a hold kept while another ready waited on the lock was lost"
    );
    assert_eq!(
        waited.since(Some("second"), second, None, 1010),
        1000,
        "the ready that waited on the lock didn't keep its own hold"
    );
}

#[test]
fn gates_are_read_as_nix_offload_says_them() {
    let folder = std::env::temp_dir().join(format!("merge-net-turn-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    let said = r#"{"pid":7,"ticks":9,"commit":"abc","folder":"/t","started":5}"#;
    fs::write(folder.join("7.json"), said).unwrap();
    fs::write(folder.join("8.json.new"), said).unwrap();
    fs::write(folder.join("9.json"), "not yet whole").unwrap();
    let read = gates(&folder);
    let _ = fs::remove_dir_all(&folder);
    assert_eq!(
        read,
        [Gate {
            pid: 7,
            ticks: 9,
            commit: "abc".to_owned(),
            folder: PathBuf::from("/t"),
            started: 5
        }],
        "the gates going were misread"
    );
}
