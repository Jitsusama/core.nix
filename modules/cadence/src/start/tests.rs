use super::*;

#[test]
fn a_host_is_known_by_either_entry() {
    for host in [
        b"node\0--disable-sigusr1\0/h/agentic-harness.pi/scripts/pi-host.mjs\0attended\0"
            .as_slice(),
        b"node\0/h/agentic-harness.pi/scripts/pi-host.ts\0background\0",
    ] {
        assert!(
            hosts(host),
            "a host not counted as one: {}",
            String::from_utf8_lossy(host)
        );
    }
    for other in [
        b"node\0/h/agentic-harness.pi/scripts/host-join.ts\0--session\0s\0".as_slice(),
        b"node\0/h/agentic-harness.pi/scripts/not-pi-host.mjs\0",
        b"vim\0scripts/pi-host.mjs\0",
    ] {
        assert!(
            !hosts(other),
            "not a host, yet counted as one: {}",
            String::from_utf8_lossy(other)
        );
    }
}

#[test]
fn a_hosted_sessions_viewer_counts_as_showing_it() {
    for viewer in [
        b"node\0/h/agentic-harness.pi/scripts/host-join.ts\0--session\0s\0".as_slice(),
        b"/nix/store/x-mycelium/bin/mycelium\0attach\0s\0",
        b"/nix/store/x-mycelium/bin/mycelium\0holds\0s\0attach\0",
        // As the launcher titles every viewer's window first.
        b"sh\0-c\0printf t; exec \"$@\"\0Reef\0mycelium\0--realm\0h\0attach\0--session\0s\0",
    ] {
        assert!(
            views(viewer),
            "a viewer counted as dead: {}",
            String::from_utf8_lossy(viewer)
        );
    }
    for other in [
        b"node\0/h/agentic-harness.pi/scripts/pi-host.ts\0".as_slice(),
        b"zsh\0-c\0mycelium attach s\0",
        b"/bin/mycelium\0held\0",
        b"/bin/mycelium\0holds\0",
    ] {
        assert!(
            !views(other),
            "not a viewer, yet counted as one: {}",
            String::from_utf8_lossy(other)
        );
    }
}

#[test]
fn a_revived_window_and_its_pi_say_the_same_numbered_class() {
    let command = terminal(
        "/home/j",
        "exec pi --session s",
        "pi-spawned.42",
        &Door::Instance,
    );
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let class = args.iter().position(|arg| arg == "--class");
    assert_eq!(
        class.and_then(|at| args.get(at + 1)).map(String::as_str),
        Some("pi-spawned.42"),
        "the window isn't opened with its own class: {args:?}"
    );
    let env = args.iter().position(|arg| arg == "env");
    let kitty = args.iter().position(|arg| arg == "kitty");
    assert!(
        env.is_some_and(|at| args.get(at + 1).map(String::as_str)
            == Some("MYCELIUM_APP_ID=pi-spawned.42")
            && kitty == Some(at + 2)),
        "its kitty isn't told the class in MYCELIUM_APP_ID: {args:?}"
    );
}

#[test]
fn a_revived_window_is_asked_of_the_sessions_kittys_socket_with_only_its_class() {
    let socket = PathBuf::from("/run/user/1000/pi-sessions/kitty.sock");
    let command = terminal(
        "/home/j",
        "exec pi --session s",
        "pi-spawned.42",
        &Door::Socket(socket),
    );
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        (command.get_program().to_string_lossy().into_owned(), args),
        (
            "kitten".to_owned(),
            [
                "@",
                "--to",
                "unix:/run/user/1000/pi-sessions/kitty.sock",
                "launch",
                "--type=os-window",
                "--os-window-class",
                "pi-spawned.42",
                "--cwd",
                "/home/j",
                "--env",
                "MYCELIUM_APP_ID=pi-spawned.42",
                "zsh",
                "-lic",
                "exec pi --session s",
            ]
            .map(String::from)
            .to_vec()
        ),
        "the window isn't asked of the socket with its class alone"
    );
}

#[test]
fn the_sessions_kitty_is_reached_by_its_socket_then_its_running_unit_and_never_otherwise() {
    let runtime = tempfile::tempdir().expect("a runtime folder to stand in");
    let proc = runtime.path().join("proc");
    let processes = runtime.path().join("cgroup.procs");
    fs::write(&processes, "").expect("an empty cgroup to stand in");
    assert_eq!(
        door(runtime.path(), &processes, &proc),
        None,
        "with no socket and its unit stopped, whoever holds the instance could be anyone"
    );
    assert_eq!(
        door(runtime.path(), &runtime.path().join("missing"), &proc),
        None,
        "with no unit at all, there is no sessions kitty"
    );
    fs::create_dir_all(proc.join("3273")).expect("a starting kitty's folder");
    fs::write(
        proc.join("3273/cmdline"),
        b"kitty\0--listen-on\0unix:/run/user/1000/pi-sessions/kitty.sock\0",
    )
    .expect("a starting kitty to stand in");
    fs::write(&processes, "3273\n").expect("a unit starting again to stand in");
    assert_eq!(
        door(runtime.path(), &processes, &proc),
        None,
        "a kitty starting before its socket exists holds no instance to be reached by"
    );
    fs::create_dir_all(proc.join("3272")).expect("an old kitty's folder");
    fs::write(
        proc.join("3272/cmdline"),
        b"kitty\0-1\0--instance-group\0pi-sessions\0--class\0pi-sessions-kept\0",
    )
    .expect("an old kitty to stand in");
    fs::write(&processes, "3272\n").expect("a running unit to stand in");
    assert_eq!(
        door(runtime.path(), &processes, &proc),
        Some(Door::Instance),
        "a unit that runs from before it listened is reached by its instance"
    );
    let socket = runtime.path().join("pi-sessions/kitty.sock");
    fs::create_dir_all(socket.parent().expect("its folder")).expect("its folder made");
    let _listening = std::os::unix::net::UnixListener::bind(&socket).expect("a socket");
    assert_eq!(
        door(runtime.path(), &processes, &proc),
        Some(Door::Socket(socket)),
        "a kitty that listens is reached by its socket"
    );
}

#[test]
fn the_sessions_kitty_units_cgroup_is_found_by_the_logins_uid() {
    assert_eq!(
        unit_processes(Path::new("/run/user/1000")),
        PathBuf::from(
            "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/agents.slice/sessions-kitty.service/cgroup.procs"
        ),
        "the cgroup isn't the sessions kitty unit's"
    );
}
