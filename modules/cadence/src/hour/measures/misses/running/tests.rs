use super::*;

const OLD: &str = "xi5w06adwa04v6ifnajk1l37fp9hn79c";

const CODE: &str = "modules/cadence/src/main.rs";

#[test]
fn a_switch_names_what_runs_while_its_system_still_does() {
    let switch: Value = serde_json::from_str(
        "{\"generation\":2,\"revs\":{\"core\":\"bbbbbbb\",\"world\":\"0000000\"}}",
    )
    .expect("a switch line");
    assert_eq!(
        switched(&switch),
        BTreeMap::from([
            ("core".to_owned(), "bbbbbbb".to_owned()),
            ("world".to_owned(), "0000000".to_owned()),
        ]),
    );
    let odd: Value =
        serde_json::from_str("{\"revs\":{\"core\":\"--output=/x\",\"world\":\"abc\"}}")
            .expect("a switch line");
    assert!(
        switched(&odd).is_empty(),
        "what isn't a commit's name reached git"
    );
    assert!(still_runs(Some(2), &(Some(2), true)), "the switch runs");
    assert!(
        !still_runs(Some(2), &(Some(1), true)),
        "a switch back to generation 1 left the record's generation 2 read as running"
    );
    assert!(
        !still_runs(Some(2), &(Some(2), false)),
        "a boot into another generation left the profile's read as running"
    );
    assert!(
        !still_runs(None, &(None, true)),
        "a switch naming no generation counted as running"
    );
}

#[test]
fn what_cadence_cant_read_counts_as_not_running() {
    let all = |_: &str, _: &str| Some(true);
    let none = |_: &str, _: &str| Some(false);
    let gone = |_: &str, _: &str| None;
    let files = |_: &str| Some(CODE.to_owned());
    let never = |_: &str, _: Reaches| -> Option<&'static str> { panic!("a host was read") };
    let at = Some("a1b2c3d");
    assert_eq!(
        judge("core", at, Some("f00d"), none, files, never),
        Some("the last switch doesn't carry it"),
        "the 21:10 case: merged, but not switched in"
    );
    assert_eq!(judge("core", at, Some("f00d"), all, files, never), None);
    assert_eq!(
        judge("core", None, Some("f00d"), all, files, never),
        Some("cadence can't see what runs here"),
        "no switch read, yet the change counted as running"
    );
    assert_eq!(
        judge("core", at, None, all, files, never),
        Some("cadence can't find its merge")
    );
    assert_eq!(
        judge("core", at, Some("f00d"), gone, files, never),
        Some("cadence can't find the revision running here"),
        "a revision not fetched yet was put to the owner as not carried"
    );
    assert_eq!(
        judge("core", at, Some("f00d"), all, |_| None, never),
        Some("cadence can't read what it changed"),
        "an unread diff skipped the hosts"
    );
    assert_eq!(
        judge("harness", at, Some("f00d"), none, files, never),
        Some("the harness installed doesn't carry it")
    );
    assert_eq!(
        judge("mycelium", None, Some("f00d"), all, files, never),
        Some("cadence can't see what runs here"),
        "mycelium, which runs as core pins it, closed on its merge"
    );
    assert_eq!(
        judge("elsewhere", None, None, none, files, never),
        None,
        "a repository deployed apart is followed to its merge"
    );
}

#[test]
fn a_change_reaching_the_hosts_waits_on_them() {
    let all = |_: &str, _: &str| Some(true);
    let lack = |_: &str, reaches: Reaches| {
        Some(if reaches.unit {
            "unit"
        } else if reaches.patches == ["modules/pi/host-patches/x.patch"] {
            "runtime"
        } else {
            "patches misread"
        })
    };
    let at = Some("a1b2c3d");
    let patch = |_: &str| {
        Some(
            [
                "check-descriptions.nix",
                "modules/pi/host-patches/x.patch",
                "modules/pi/host-patches/README.md",
            ]
            .join("\n"),
        )
    };
    assert_eq!(
        judge("core", at, Some("f00d"), all, patch, lack),
        Some("runtime"),
        "switched in, but a host patch didn't wait on the hosts"
    );
    // `changed` names core's runtime build only where it changed more than
    // its list of patches, which a runtime built elsewhere may lack beside
    // the patch.
    let beside =
        |_: &str| Some("modules/pi/host-patches/x.patch\nmodules/pi/hosted-pi.nix".to_owned());
    assert_eq!(
        judge("core", at, Some("f00d"), all, beside, lack),
        Some("cadence can't tell what a pi host loads"),
        "a patch beside a change to core's runtime build was judged by the patch alone"
    );
    // A fix to how core builds its runtime, with no patch, can't be seen in
    // a runtime built elsewhere, while a note beside the patches reaches no
    // host at all.
    let build =
        |_: &str| Some("modules/pi/hosted-pi.nix\nmodules/pi/host-patches/README.md".into());
    assert_eq!(
        judge("core", at, Some("f00d"), all, build, lack),
        Some("cadence can't tell what a pi host loads"),
        "a fix to core's runtime build alone was told fixed"
    );
    let note = |_: &str| Some("modules/pi/host-patches/README.md".to_owned());
    assert_eq!(
        judge("core", at, Some("f00d"), all, note, lack),
        None,
        "a change reaching no host's patches waited on the hosts"
    );
    let unit = |_: &str| Some("modules/pi/host.nix".to_owned());
    assert_eq!(
        judge("core", at, Some("f00d"), all, unit, lack),
        Some("unit"),
        "the hosts' unit, which a switch leaves a running host keeping, didn't wait on them"
    );
    let home = |_: &str| Some(format!("modules/pi/home.nix\n{CODE}"));
    assert_eq!(
        judge("core", at, Some("f00d"), all, home, lack),
        None,
        "a switch alone didn't carry the rest"
    );
}

#[test]
fn a_runtime_build_that_only_lists_the_patches_changed_is_left_to_them() {
    let files = [
        "modules/pi/host-patches/x.patch",
        "modules/pi/hosted-pi.nix",
    ];
    let listed = "--- a/modules/pi/hosted-pi.nix\n+++ b/modules/pi/hosted-pi.nix\n\
                  @@ -24 +24,2 @@\n+    ./host-patches/x.patch\n";
    assert!(
        only_lists(listed, &files),
        "a new patch's line in the list was taken for a change to the build"
    );
    let built =
        format!("{listed}@@ -30 +31 @@\n-  patchFlags = [ ];\n+  patchFlags = [ \"-p1\" ];\n");
    assert!(
        !only_lists(&built, &files),
        "a change to a step of the build was taken for the list alone"
    );
    let other = "@@ -24 +24 @@\n+    ./host-patches/y.patch\n";
    assert!(
        !only_lists(other, &files),
        "a line listing a patch the merge didn't change was taken for its own"
    );
}

#[test]
fn a_merge_is_found_by_its_subject_not_a_later_body() {
    let found = concat!(
        "1111111 fix(pi): quote the crash (#5800)\n",
        "2222222 Merge pull request #5800 from me/fix\n",
    );
    assert_eq!(subject_merging(found, 5800), Some("1111111".to_owned()));
    let body = "3333333 Merge pull request #5801 from me/next\n";
    assert_eq!(
        subject_merging(body, 5800),
        None,
        "a later merge whose body named the change was taken as its merge"
    );
}

#[test]
fn a_host_is_read_by_its_units_main_process() {
    let environ =
        format!("HOME=/x\0PI_PACKAGE_DIR=/nix/store/{OLD}-pi-host-runtime-1.5.0/lib/pi\0");
    assert_eq!(
        runtime(environ.as_bytes()),
        Some(format!("/nix/store/{OLD}-pi-host-runtime-1.5.0").into()),
        "a host's runtime wasn't read as the store path its pi lies in"
    );
    let other = format!("PI_PACKAGE_DIR=/nix/store/{OLD}-pi-coding-agent-1.5.0/lib\0");
    assert_eq!(
        runtime(other.as_bytes()),
        Some(format!("/nix/store/{OLD}-pi-coding-agent-1.5.0").into()),
        "a runtime named otherwise, as core's own build is, wasn't read"
    );
    let stopped = format!(
        "MainPID=0\nEnvironment=HOME=/x \
         \"PI_PACKAGE_DIR=/nix/store/{OLD}-pi-host-runtime-1.5.0/lib/pi\" A=b\n"
    );
    assert_eq!(
        unit_runtime(&stopped),
        Some(format!("/nix/store/{OLD}-pi-host-runtime-1.5.0").into()),
        "a host with no process wasn't read by the runtime its unit starts it on"
    );
    assert_eq!(unit_runtime("MainPID=0\nEnvironment=HOME=/x\n"), None);
    assert_eq!(
        runtime(b"PI_PACKAGE_DIR=/home/x/pi\0"),
        None,
        "a pi outside the store"
    );
    assert_eq!(runtime(b"HOME=/home/x\0"), None, "no pi named");
    let climbs = format!("PI_PACKAGE_DIR=/nix/store/{OLD}-a/../{OLD}-b/lib\0");
    assert_eq!(
        runtime(climbs.as_bytes()),
        None,
        "a path climbing out of one store path was judged by it"
    );
    assert!(
        host_unit("pi-host-work-background.service"),
        "a host's unit wasn't a host"
    );
    let listed = [
        "pi-host-work-attended.service loaded active running pi",
        "pi-host-work-background.service loaded inactive dead pi",
        "pi-host-home-attended.service not-found active running pi",
        "pi-host-gone-attended.service not-found inactive dead pi",
        "pi-host-down@x.service loaded inactive dead down",
        "pi-host-work-failing.service loaded failed failed pi",
    ];
    assert_eq!(
        listed
            .iter()
            .filter_map(|line| listed_host(line))
            .collect::<Vec<_>>(),
        [
            ("pi-host-work-attended.service", false),
            ("pi-host-work-background.service", true),
            ("pi-host-home-attended.service", false),
            ("pi-host-work-failing.service", false),
        ],
        "a stopped host, a failed one or one still running after its unit's file went was \
         left out, or a failed one was read as stopped, by the runtime it can't start on"
    );
    assert!(
        !host_unit("pi-host-down@pi-host-work-attended.service"),
        "the unit saying a host is down was a host"
    );
    assert!(!host_unit("pi-hostess.service"), "another unit was a host");
    assert_eq!(
        shown_host("MainPID=3342173\nActiveEnterTimestamp=@1795138373\n"),
        (Some(3_342_173), Some(1_795_138_373))
    );
    assert_eq!(
        shown_host("MainPID=0\nActiveEnterTimestamp=\n"),
        (None, None),
        "a unit with no process or start read as one"
    );
}

#[test]
fn a_host_resumes_while_it_activates_and_from_when_it_became_active() {
    let now = 1_795_200_000;
    let shown = |state: &str, began: &str, entered: &str| {
        format!(
            "ActiveState={state}\nExecMainStartTimestamp={began}\n\
             ActiveEnterTimestamp={entered}\n"
        )
    };
    assert_eq!(
        resumed(&shown("activating", "@1795199500", "@1795100000"), now),
        Some(now),
        "a host still resuming its conversations was read by its last activation"
    );
    assert_eq!(
        resumed(&shown("active", "@1795199000", "@1795199500"), now),
        Some(1_795_199_500),
        "an active host wasn't read from when it became so"
    );
    assert_eq!(
        resumed(&shown("active", "@1795199700", "@1795199500"), now),
        Some(1_795_199_700),
        "a process begun since the host became active was left out"
    );
    assert_eq!(
        resumed(&shown("active", "@1795199700", ""), now),
        None,
        "an active host with no time it became so was read as just started"
    );
    for state in ["failed", "deactivating", "inactive"] {
        assert_eq!(
            resumed(&shown(state, "@1795199900", "@1795199900"), now),
            None,
            "a host {state} was read as resuming"
        );
    }
    assert_eq!(resumed("", now), None, "nothing shown was read as a host");
}

#[test]
fn a_lock_pins_each_input_by_the_roots_name_for_it() {
    let lock = r#"{"nodes": {
        "root": {"inputs": {"core": "core_2", "mycelium": "mycelium"}},
        "core": {"locked": {"rev": "0123456"}},
        "core_2": {"locked": {"rev": "abcdef0"}},
        "mycelium": {"locked": {"rev": "fedcba9"}}}}"#;
    assert_eq!(input_pin(lock, "core"), Some("abcdef0".to_owned()));
    assert_eq!(input_pin(lock, "mycelium"), Some("fedcba9".to_owned()));
    assert_eq!(
        input_pin(r#"{"nodes": {"root": {"inputs": {}}}}"#, "core"),
        None,
        "no core input"
    );
    assert_eq!(input_pin("not json", "core"), None, "a torn lock");
}

/// A host whose runtime applied the patch `x.patch` reading `kept`, or kept
/// no copies when `kept` is `None`.
fn host(kept: Option<&str>, started: Option<i64>) -> Host {
    Host {
        applied: kept.map(|kept| Applied::from([("x.patch".to_owned(), kept.to_owned())])),
        started,
        stopped: false,
    }
}

#[test]
fn every_host_must_load_a_runtime_that_applied_the_patch() {
    let merged = Wanted::from([("x.patch".to_owned(), vec![Some("new".to_owned())])]);
    let runtime = Reaches {
        patches: vec!["modules/pi/host-patches/x.patch".to_owned()],
        unit: false,
    };
    let lack = |hosts: Option<&[Host]>| {
        hosts_lack(hosts, &runtime, None, |applied| {
            Some(loads(applied, &merged))
        })
    };
    assert_eq!(lack(Some(&[host(Some("new"), None)])), None);
    assert_eq!(
        lack(Some(&[host(Some("new"), None), host(Some("old"), None)])),
        Some("the pi hosts don't load it yet"),
        "one host on a runtime with the old patch left it unrun"
    );
    assert_eq!(
        lack(Some(&[host(Some("new"), None), host(None, None)])),
        Some("cadence can't tell what a pi host loads"),
        "a runtime keeping no copies counted as carrying it"
    );
    assert_eq!(
        lack(Some(&[host(Some("old"), None), host(None, None)])),
        Some("the pi hosts don't load it yet"),
        "a host known to lack it was said as cadence's blindness, so its owner wasn't asked"
    );
    assert_eq!(lack(Some(&[])), Some("no pi host runs to load it"));
    assert_eq!(
        lack(None),
        Some("cadence can't list the pi hosts"),
        "hosts cadence couldn't list counted as none"
    );
    let unread = |hosts: &[Host]| hosts_lack(Some(hosts), &runtime, None, |_| None);
    assert_eq!(
        unread(&[host(Some("new"), None)]),
        Some("cadence can't tell what a pi host loads"),
        "a patch whose merged version went unread counted as loaded"
    );
}

/// The 19:05 miss of 19 November: #5800 added a patch that the work hosts'
/// runtime, built from its own fixed list, never applied.
#[test]
fn a_patch_runs_once_a_runtime_applied_it_as_merged_or_since() {
    // A change that rewrote a.patch, later rewritten again, and removed
    // gone.patch.
    let wanted = Wanted::from([
        (
            "a.patch".to_owned(),
            vec![Some("a1".to_owned()), Some("a2".to_owned())],
        ),
        ("gone.patch".to_owned(), vec![None]),
    ]);
    let applied = |patches: &[(&str, &str)]| -> Applied {
        patches
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect()
    };
    assert!(
        loads(&applied(&[("a.patch", "a1")]), &wanted),
        "the patch as merged didn't count as carrying the change"
    );
    assert!(
        loads(&applied(&[("a.patch", "a2")]), &wanted),
        "a later version of the patch didn't count as carrying the change"
    );
    assert!(
        !loads(&applied(&[("a.patch", "a0")]), &wanted),
        "a version the change doesn't name counted as carrying it"
    );
    assert!(
        !loads(&applied(&[]), &wanted),
        "a runtime that never applied the rewritten patch counted as loading it"
    );
    assert!(
        !loads(&applied(&[("a.patch", "a1"), ("gone.patch", "g")]), &wanted),
        "a runtime still applying the removed patch counted as loading its removal"
    );
}

/// Runs git in `repo` for a history of the test's own, apart from the
/// account's config, so it neither signs nor asks who commits.
fn git(repo: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "here")
        .env("GIT_AUTHOR_EMAIL", "here@example.org")
        .env("GIT_COMMITTER_NAME", "here")
        .env("GIT_COMMITTER_EMAIL", "here@example.org")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A scratch folder of its own, emptied of what a failed run left.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cadence-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    dir
}

#[test]
fn a_patch_carries_a_change_as_merged_or_as_mains_own_line_left_it() {
    let repo = scratch("versions");
    std::fs::create_dir_all(&repo).expect("a scratch history");
    // The patches lie where core keeps them, and are named by their files.
    let [x, y, gone, later, backed] =
        ["x", "y", "gone", "later", "backed"].map(|name| format!("{PATCHES}{name}.patch"));
    let (x, y, gone, later) = (x.as_str(), y.as_str(), gone.as_str(), later.as_str());
    let backed = backed.as_str();
    // Writes each patch named with its text, or removes it, and commits.
    let commit = |patches: &[(&str, Option<&str>)], message: &str| {
        for (name, text) in patches {
            let file = repo.join(name);
            std::fs::create_dir_all(file.parent().expect("a folder")).expect("a folder made");
            match text {
                Some(text) => std::fs::write(file, text).expect("a patch written"),
                None => std::fs::remove_file(file).expect("a patch removed"),
            }
        }
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", message]);
    };
    git(&repo, &["init", "-q", "-b", "main"]);
    let first = [
        (x, Some("oldest")),
        (y, Some("y0")),
        (gone, Some("g")),
        (later, Some("l0")),
        (backed, Some("b0")),
    ];
    commit(&first, "before");
    commit(&[(x, Some("old"))], "a step");
    // A side branch cut before the fix, holding a patch without it.
    git(&repo, &["checkout", "-qb", "side"]);
    commit(&[(y, Some("side"))], "side");
    git(&repo, &["checkout", "-qb", "fix", "main"]);
    let fix = [
        (x, Some("new")),
        (y, Some("y1")),
        (gone, None),
        (later, Some("l1")),
        (backed, Some("b1")),
    ];
    commit(&fix, "fix");
    let head = |name: &str| {
        let out = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "rev-parse", name])
            .output()
            .expect("git runs");
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    let fixed = head("HEAD");
    git(&repo, &["checkout", "-q", "main"]);
    git(&repo, &["merge", "-q", "--no-ff", "-m", "the fix", "fix"]);
    let sha = head("HEAD");
    commit(&[(x, Some("newer")), (backed, Some("b2"))], "a later fix");
    // The side branch merged since, its text taking y back to one without
    // the fix.
    commit(&[(y, Some("y2"))], "a later fix to y");
    git(
        &repo,
        &[
            "merge", "-q", "--no-ff", "-X", "theirs", "-m", "side", "side",
        ],
    );
    // The fix's commit reverted after its patch changed again, which leaves
    // a text found nowhere before; a revert to a state from before the step
    // before the fix, a rewrite built on it, and a later removal of what
    // the fix rewrote.
    let undone = format!("Revert \"fix\"\n\nThis reverts commit {fixed}.");
    commit(&[(backed, Some("b2 without b1"))], &undone);
    commit(&[(x, Some("oldest"))], "a revert");
    commit(&[(x, Some("rewritten")), (y, Some("y3"))], "a rewrite");
    commit(&[(later, None)], "a removal");
    let path = repo.to_string_lossy();
    let patches = [x, y, gone, later, backed].map(str::to_owned);
    assert_eq!(
        versions(&path, &sha, &patches),
        Some(Wanted::from([
            (
                "x.patch".to_owned(),
                vec![Some("new".to_owned()), Some("newer".to_owned())]
            ),
            (
                "y.patch".to_owned(),
                vec![Some("y1".to_owned()), Some("y2".to_owned())]
            ),
            ("gone.patch".to_owned(), vec![None]),
            ("later.patch".to_owned(), vec![Some("l1".to_owned())]),
            (
                "backed.patch".to_owned(),
                vec![Some("b1".to_owned()), Some("b2".to_owned())]
            ),
        ])),
        "a revert, what was built on it, a later removal or a side branch's patch counted \
         as carrying the fix"
    );
    assert_eq!(
        versions(&path, "0000000000000000000000000000000000000000", &patches),
        None,
        "a merge cadence doesn't have read as one carrying nothing"
    );
    assert!(
        matches!(state(&path, &sha, gone), Ok(None)),
        "a patch the merge removed wasn't read as absent"
    );
    assert!(
        state(&path, "0000000000000000000000000000000000000000", x).is_err(),
        "a commit git can't read was taken for one lacking the patch"
    );
    std::fs::remove_dir_all(&repo).expect("the scratch history removed");
}

#[test]
fn a_runtime_is_read_by_the_copies_it_keeps() {
    let root = scratch("applied");
    assert_eq!(
        applied(&root),
        None,
        "a runtime keeping no copies read as one"
    );
    std::fs::create_dir_all(root.join(APPLIED)).expect("a scratch runtime");
    std::fs::write(root.join(APPLIED).join("x.patch"), "new").expect("a copy written");
    assert_eq!(
        applied(&root),
        Some(Applied::from([("x.patch".to_owned(), "new".to_owned())]))
    );
    std::fs::remove_dir_all(&root).expect("the scratch runtime removed");
}

#[test]
fn every_host_must_have_started_since_its_unit_switched_in() {
    let unit = Reaches {
        patches: Vec::new(),
        unit: true,
    };
    let lack = |hosts: &[Host], at| hosts_lack(Some(hosts), &unit, at, |_| Some(true));
    assert_eq!(lack(&[host(None, Some(200))], Some(100)), None);
    assert_eq!(
        lack(&[host(None, Some(200)), host(None, Some(50))], Some(100)),
        Some("a pi host keeps its unit from before it"),
        "a host started before the switch counted as running its new unit"
    );
    assert_eq!(
        lack(&[host(None, None)], Some(100)),
        Some("cadence can't tell when a pi host started")
    );
    // A host stopped since it last started keeps no unit, and starts on the
    // one switched in.
    let stopped = |started| Host {
        stopped: true,
        ..host(None, started)
    };
    assert_eq!(
        lack(
            &[host(None, Some(200)), stopped(Some(50)), stopped(None)],
            Some(100)
        ),
        None,
        "a stopped host's last start held its unit's miss open"
    );
    assert_eq!(
        lack(&[host(None, Some(200))], None),
        Some("cadence can't tell when a pi host started"),
        "no switch found carrying it, yet the hosts counted as started since"
    );
}

#[test]
fn a_unit_switched_in_when_a_switch_first_carried_it() {
    let switches = concat!(
        "{\"at\":\"2026-11-19T19:57:21-05:00\",\"revs\":{\"core\":\"aaaaaaa\"}}\n",
        "{\"at\":\"2026-11-19T20:20:38-05:00\",\"revs\":{\"core\":\"bbbbbbb\"}}\n",
        "{\"at\":\"2026-11-19T21:00:00-05:00\",\"revs\":{\"core\":\"ccccccc\"}}\n",
    );
    let at: jiff::Timestamp = "2026-11-19T20:20:38-05:00".parse().expect("a time");
    assert_eq!(
        first_switched(switches, |rev| rev != "aaaaaaa"),
        Some(at.as_second()),
        "a later switch, not the first carrying it, was read"
    );
    assert_eq!(first_switched(switches, |_| false), None);
}

#[test]
fn a_change_seen_running_is_kept_and_not_read_again() {
    let folder = std::env::temp_dir().join(format!("cadence-ran-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a scratch folder");
    let apart = Named {
        repo: "elsewhere".into(),
        number: 7,
    };
    let repos = Checkouts::from([("elsewhere".to_owned(), folder.clone())]);
    let merged = BTreeSet::from([apart.clone()]);
    assert!(
        unrun(&merged, &repos, "", &folder).is_empty(),
        "a change deployed apart was held"
    );
    let kept: BTreeSet<Named> =
        serde_json::from_str(&std::fs::read_to_string(folder.join("ran.json")).expect("kept"))
            .expect("a set");
    assert_eq!(kept, merged, "the change seen running wasn't kept");
    let core = Named {
        repo: "core".into(),
        number: 1,
    };
    let repos = Checkouts::from([("core".to_owned(), folder.clone())]);
    let held = unrun(&BTreeSet::from([core.clone()]), &repos, "", &folder);
    assert_eq!(
        held.get(&core),
        Some(&"cadence can't see what runs here"),
        "a core change with no switch read counted as running"
    );
    let ran = BTreeSet::from([apart, core.clone()]);
    std::fs::write(
        folder.join("ran.json"),
        serde_json::to_string(&ran).expect("a set"),
    )
    .expect("ran.json written");
    assert!(
        unrun(&BTreeSet::from([core]), &repos, "", &folder).is_empty(),
        "a change kept as running was judged again, and could reopen its miss"
    );
    std::fs::remove_dir_all(&folder).expect("the scratch folder removed");
}
