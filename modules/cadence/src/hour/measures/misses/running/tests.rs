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
    let lack = |_: &str, reaches: Reaches| Some(if reaches.unit { "unit" } else { "runtime" });
    let at = Some("a1b2c3d");
    let patch = |_: &str| Some("check-descriptions.nix\nmodules/pi/host-patches/x.patch".into());
    assert_eq!(
        judge("core", at, Some("f00d"), all, patch, lack),
        Some("runtime"),
        "switched in, but a host patch didn't wait on the hosts"
    );
    let build = |_: &str| Some("modules/pi/hosted-pi.nix".to_owned());
    assert_eq!(
        judge("core", at, Some("f00d"), all, build, lack),
        Some("runtime")
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
fn a_runtime_is_pinned_where_it_was_first_named() {
    let touched = "dropped\nrenamed\nadded\n";
    let names = |commit: &str| commit != "dropped";
    assert_eq!(
        first_naming(touched, names),
        Some("added"),
        "the commit dropping the runtime, or a later one, was read as its pin"
    );
    assert_eq!(
        first_naming("", |_| true),
        None,
        "a runtime pinned nowhere was read against the working lock"
    );
    assert_eq!(
        first_naming("dropped\n", |_| false),
        None,
        "a commit that only dropped the runtime counted as its pin"
    );
}

#[test]
fn a_host_is_read_by_its_units_main_process() {
    let environ =
        format!("HOME=/x\0PI_PACKAGE_DIR=/nix/store/{OLD}-pi-host-runtime-1.5.0/lib/pi\0");
    assert_eq!(runtime(environ.as_bytes()), Some(OLD.to_owned()));
    let other = format!("PI_PACKAGE_DIR=/nix/store/{OLD}-pi-1.5.0/lib\0");
    assert_eq!(runtime(other.as_bytes()), None, "a pi not a host's runtime");
    assert_eq!(
        runtime(b"PI_PACKAGE_DIR=/home/x/pi\0"),
        None,
        "a pi outside the store"
    );
    assert_eq!(runtime(b"HOME=/home/x\0"), None, "no pi named");
    assert!(
        host_unit("pi-host-work-background.service"),
        "a host's unit wasn't a host"
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

#[test]
fn a_pin_names_a_checkout_and_a_folder() {
    assert_eq!(
        Pin::parse("there=a/b/"),
        Some(Pin {
            checkout: "there",
            folder: "a/b",
        })
    );
    assert_eq!(Pin::parse("there="), None, "no folder");
    assert_eq!(Pin::parse("=a"), None, "no checkout");
    assert_eq!(Pin::parse("there"), None, "no folder at all");
}

fn host(core: Option<&str>, started: Option<i64>) -> Host {
    Host {
        core: core.map(str::to_owned),
        started,
    }
}

#[test]
fn every_host_must_load_a_runtime_pinned_with_the_change() {
    let pin = Pin::parse("there=a");
    let has = |rev: &str| rev == "new";
    let runtime = Reaches {
        runtime: true,
        unit: false,
    };
    let lack = |hosts: Option<&[Host]>, pin| hosts_lack(hosts, pin, runtime, None, has);
    assert_eq!(lack(Some(&[host(Some("new"), None)]), pin), None);
    assert_eq!(
        lack(
            Some(&[host(Some("new"), None), host(Some("old"), None)]),
            pin
        ),
        Some("the pi hosts don't load it yet"),
        "one host on the old runtime left it unrun"
    );
    assert_eq!(
        lack(Some(&[host(Some("new"), None), host(None, None)]), pin),
        Some("cadence can't tell what a pi host loads"),
        "a runtime pinned nowhere cadence reads counted as carrying it"
    );
    assert_eq!(
        lack(Some(&[host(Some("old"), None), host(None, None)]), pin),
        Some("the pi hosts don't load it yet"),
        "a host known to lack it was said as cadence's blindness, so its owner wasn't asked"
    );
    assert_eq!(lack(Some(&[]), pin), Some("no pi host runs to load it"));
    assert_eq!(
        lack(None, pin),
        Some("cadence can't list the pi hosts"),
        "hosts cadence couldn't list counted as none"
    );
    assert_eq!(
        lack(Some(&[host(Some("new"), None)]), None),
        Some("cadence can't see what the pi hosts load"),
        "with no pin named, a host's runtime counted as followed"
    );
}

#[test]
fn every_host_must_have_started_since_its_unit_switched_in() {
    let unit = Reaches {
        runtime: false,
        unit: true,
    };
    let lack = |hosts: &[Host], at| hosts_lack(Some(hosts), None, unit, at, |_| true);
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
        unrun(&merged, &repos, "", None, &folder).is_empty(),
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
    let held = unrun(&BTreeSet::from([core.clone()]), &repos, "", None, &folder);
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
        unrun(&BTreeSet::from([core]), &repos, "", None, &folder).is_empty(),
        "a change kept as running was judged again, and could reopen its miss"
    );
    std::fs::remove_dir_all(&folder).expect("the scratch folder removed");
}
