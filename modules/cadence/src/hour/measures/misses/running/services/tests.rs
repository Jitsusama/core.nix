use super::*;

const NODE: &str = "/nix/store/p1m6i9bhgngwvizf0nsxn5gmb7qmcg1i-mycelium-0.1.0";
const PINNED: &str = "/nix/store/20lhvy6h000000000000000000000000-mycelium-0.1.0";
const COREUTILS: &str = "/nix/store/a50ml7za6h61m4mv4cdbvkl33c6sx5yn-coreutils-9.12";
const BASH: &str = "/nix/store/22222222222222222222222222222222-bash-5.3";

fn service(unit: &str, runs: Runs) -> Service {
    Service {
        unit: unit.to_owned(),
        runs,
    }
}

fn owned(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

/// A block of `systemctl show` for `unit`, as `read` asks for it.
fn shown(unit: &str, pid: u32, exec: &str, fragment: &str) -> String {
    format!(
        "Id={unit}\nMainPID={pid}\nExecStart={{ path={exec} ; argv[]={exec} node ; \
         ignore_errors=no }}\nFragmentPath={fragment}"
    )
}

#[test]
fn a_unit_names_its_path_or_what_a_starter_runs() {
    let direct = format!(
        "{{ path={NODE}/bin/mycelium ; argv[]={NODE}/bin/mycelium --realm work node ; \
         ignore_errors=no ; start_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }}"
    );
    assert_eq!(
        named(&direct),
        Some(format!("{NODE}/bin/mycelium")),
        "a unit's own path wasn't read as what it names"
    );
    let started = format!(
        "{{ path={COREUTILS}/bin/env ; argv[]={COREUTILS}/bin/env -u INVOCATION_ID \
         {NODE}/bin/mycelium node ; ignore_errors=no }}"
    );
    assert_eq!(
        named(&started),
        Some(format!("{NODE}/bin/mycelium")),
        "env, which only starts the program, was read as what the unit names"
    );
    let set = format!(
        "{{ path={COREUTILS}/bin/env ; argv[]={COREUTILS}/bin/env \
         WORDS=/nix/store/ym0pgbab76dr2zxqnfnlcr97fzhh3ybs-words.json \
         {NODE}/bin/mycelium ; ignore_errors=no }}"
    );
    assert_eq!(
        named(&set),
        Some(format!("{NODE}/bin/mycelium")),
        "a setting naming a file in the store was read as the program env starts"
    );
    assert_eq!(
        named("{ argv[]=x ; ignore_errors=no }"),
        None,
        "a unit naming no path was read as naming one"
    );
}

#[test]
fn a_unit_file_names_its_first_exec_start_s_program() {
    let unit = format!(
        "[Unit]\nDescription=x\n\n[Service]\nExecStartPre=/bin/true\n\
         ExecStart=-\"{NODE}/bin/mycelium\" --realm work node\nExecStart={PINNED}/bin/mycelium\n"
    );
    assert_eq!(
        fragment_named(&unit),
        Some(format!("{NODE}/bin/mycelium")),
        "a unit file's first ExecStart, past its prefix and quotes, wasn't what it names"
    );
    let started = format!("[Service]\nExecStart={COREUTILS}/bin/env A=1 {NODE}/bin/mycelium\n");
    assert_eq!(
        fragment_named(&started),
        Some(format!("{NODE}/bin/mycelium")),
        "env in a unit file was read as what it names"
    );
    assert_eq!(
        fragment_named("[Service]\nType=oneshot\n"),
        None,
        "a unit file with no ExecStart was read as naming a program"
    );
}

#[test]
fn a_process_may_run_its_stored_first_two_arguments_or_its_executable() {
    let python = "/nix/store/00000000000000000000000000000000-python3-3.12";
    let words = "/nix/store/ym0pgbab76dr2zxqnfnlcr97fzhh3ybs-words.json";
    assert_eq!(
        programs(
            &owned(&[&format!("{PINNED}/bin/mycelium"), "node", "--words", words]),
            None,
        ),
        owned(&[PINNED]),
        "the program a process was started as wasn't read, or a data file was"
    );
    assert_eq!(
        programs(
            &owned(&[
                &format!("{python}/bin/python3.12"),
                &format!("{NODE}/bin/script")
            ]),
            Some(&format!("{python}/bin/python3.12")),
        ),
        owned(&[NODE]),
        "an interpreter, which a new script's closure holds too, was read beside its script"
    );
    assert_eq!(
        programs(
            &owned(&["kitty"]),
            Some(&format!("{NODE}/bin/.kitty-wrapped"))
        ),
        owned(&[NODE]),
        "a process started by a bare name wasn't read by its executable"
    );
    assert!(
        programs(
            &owned(&["/run/current-system/sw/bin/mycelium", "node"]),
            None
        )
        .is_empty(),
        "a link the process was started through was read as today's build"
    );
    assert!(
        programs(&owned(&["nix-daemon", "--daemon"]), None).is_empty(),
        "root's process, read neither way, was given a build"
    );
    assert!(
        programs(
            &owned(&[&format!("{BASH}/bin/bash"), "-c", "a; b"]),
            Some(&format!("{BASH}/bin/bash")),
        )
        .is_empty(),
        "a shell running a command line was given its interpreter's build"
    );
}

#[test]
fn a_service_runs_its_unit_s_build_or_one_in_its_closure() {
    let never = |_: &str| -> Option<BTreeSet<String>> { panic!("the closure was read") };
    assert_eq!(
        runs(&owned(&[NODE]), &format!("{NODE}/bin/mycelium"), never),
        Some(Runs::Named),
        "a service running its unit's own build was read as stale"
    );
    let wrapped = "/nix/store/11111111111111111111111111111111-mycelium-unwrapped";
    let held = |_: &str| Some(BTreeSet::from([NODE.to_owned(), wrapped.to_owned()]));
    assert_eq!(
        runs(&owned(&[wrapped]), NODE, held),
        Some(Runs::Named),
        "a wrapper's target, in the closure its unit names, was read as stale"
    );
    assert_eq!(
        runs(&owned(&[PINNED]), NODE, held),
        Some(Runs::Another),
        "the node a drop-in pinned to 23 November's build read as running its unit's"
    );
    assert_eq!(
        runs(&owned(&[PINNED]), NODE, |_| None),
        Some(Runs::Unread),
        "a closure that couldn't be read let the service go unjudged"
    );
    assert_eq!(
        runs(&[], NODE, never),
        None,
        "a process whose build couldn't be read was judged"
    );
}

#[test]
fn a_drop_in_s_pinned_build_reads_against_the_unit_file_the_switch_wrote() {
    // As on 23 November: the drop-in's ExecStart, which `systemctl show`
    // gives, names the pinned build the process runs; the unit file names
    // the switch's.
    let blocks = [
        shown(
            "mycelium.service",
            41,
            &format!("{PINNED}/bin/mycelium"),
            "/u/mycelium.service",
        ),
        shown("x.service", 42, &format!("{NODE}/bin/x"), "/u/x.service"),
        "Id=gone.service\nMainPID=0\nExecStart=\nFragmentPath=".to_owned(),
    ]
    .join("\n\n");
    let fragment = |path: &str| match path {
        "/u/mycelium.service" => Some(format!("[Service]\nExecStart={NODE}/bin/mycelium node\n")),
        _ => None,
    };
    let process = |pid: u32| match pid {
        41 => owned(&[PINNED]),
        42 => owned(&[NODE]),
        _ => Vec::new(),
    };
    let closure = |root: &str| Some(BTreeSet::from([root.to_owned()]));
    assert_eq!(
        judged(&blocks, fragment, process, closure),
        vec![
            service("mycelium.service", Runs::Another),
            service("x.service", Runs::Named),
        ],
        "a drop-in that copied ExecStart hid the pinned build, or a unit file that \
         couldn't be read didn't fall back to what systemd shows"
    );
}

#[test]
fn a_service_running_another_program_by_design_is_left_out() {
    let blocks = [
        shown("merge-net.service", 7, &format!("{NODE}/bin/merge-net"), ""),
        shown(
            "sessions-kitty.service",
            8,
            &format!("{COREUTILS}/bin/sleep"),
            "",
        ),
    ]
    .join("\n\n");
    assert!(
        judged(
            &blocks,
            |_| None,
            |_| owned(&[PINNED]),
            |_| Some(BTreeSet::new())
        )
        .is_empty(),
        "a service that runs another build by design was judged"
    );
    assert!(
        RUN_ANOTHER
            .iter()
            .all(|(unit, why)| unit.ends_with(".service") && !why.is_empty()),
        "a service left out of the read doesn't say why"
    );
}

#[test]
fn systemd_s_listing_gives_each_service_s_name() {
    let listed = "mycelium.service loaded active running Mycelium\n\
                  a.socket loaded active running A\n\
                  pi-a@1.service loaded active running Pi\n";
    assert_eq!(
        listed_units(listed),
        vec!["mycelium.service", "pi-a@1.service"],
        "a listed service was missed, or a socket was read as one"
    );
}

#[test]
fn a_change_reaches_the_services_its_folders_name() {
    assert_eq!(
        reached("mycelium", None),
        BTreeSet::from(["mycelium".to_owned()]),
        "a mycelium change didn't reach mycelium's services"
    );
    assert_eq!(
        reached(
            "core",
            Some("modules/model-relay/src/a.rs\nflake.lock\nmodules/x/y.nix")
        ),
        BTreeSet::from(["model-relay".to_owned(), "x".to_owned()]),
        "a core change's folders under modules/ weren't what it reaches"
    );
    assert!(
        reached("world", Some("modules/x/y.nix")).is_empty(),
        "a change to a repository whose services cadence doesn't map reached some"
    );
    assert!(
        reaches("model-relay", "model-relay-agents.service"),
        "a unit named for its folder and a part wasn't reached"
    );
    assert!(
        reaches("mycelium", "mycelium.service"),
        "a unit named for its folder alone wasn't reached"
    );
    assert!(
        reaches("pi", "pi-conversation-a@1.service"),
        "a templated unit named for its folder wasn't reached"
    );
    assert!(
        !reaches("model", "modelling.service"),
        "a unit whose name only begins with the folder's letters was reached"
    );
}

#[test]
fn a_change_waits_on_a_stale_or_unread_service_it_reaches() {
    let mycelium = reached("mycelium", None);
    let pinned = [
        service("mycelium.service", Runs::Another),
        service("x.service", Runs::Named),
    ];
    assert_eq!(
        lack(&mycelium, Some(&pinned)),
        Some(STALE),
        "a mycelium merge read as running while its node ran a pinned older build"
    );
    let unread = [service("mycelium.service", Runs::Unread)];
    assert_eq!(
        lack(&mycelium, Some(&unread)),
        Some(UNREAD),
        "a service whose build couldn't be read let a change reaching it read as running"
    );
    let fine = [
        service("mycelium.service", Runs::Named),
        service("x.service", Runs::Another),
    ];
    assert_eq!(
        lack(&mycelium, Some(&fine)),
        None,
        "another folder's stale service held a change that doesn't reach it"
    );
    assert_eq!(
        lack(&mycelium, None),
        Some(UNLISTED),
        "a blind hour let a change reaching a service read as running"
    );
    assert_eq!(
        lack(&BTreeSet::new(), None),
        None,
        "a change reaching no service waited on the services"
    );
    assert!(
        UNLISTED.starts_with(super::super::UNSEEN)
            && UNREAD.starts_with(super::super::UNSEEN)
            && !STALE.starts_with(super::super::UNSEEN),
        "cadence's own blindness wasn't said as it says it elsewhere"
    );
}
