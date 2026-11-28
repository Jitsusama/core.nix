//! How typing reads niri and kitty, and says a type to the journal.

use super::*;
use serde_json::json;

#[test]
fn the_launcher_holding_the_keys_is_read_from_its_layer() {
    let holding = json!({"Ok": {"Layers": [
        {"namespace": "launcher", "keyboard_interactivity": "Exclusive"}]}});
    let shown = json!({"Ok": {"Layers": [
        {"namespace": "launcher", "keyboard_interactivity": "None"},
        {"namespace": "margin", "keyboard_interactivity": "Exclusive"}]}});
    assert_eq!(let_go(&holding.to_string()), Some(false));
    assert_eq!(let_go(&shown.to_string()), Some(true));
    assert_eq!(let_go("{\"Ok\": {\"Windows\": []}}"), None);
}

#[test]
fn the_focused_window_is_read_with_its_process_and_app() {
    let one = json!({"Ok": {"FocusedWindow": {"id": 7, "pid": 42, "app_id": "firefox"}}});
    assert_eq!(
        focused(&one.to_string()),
        Some(Some(Window {
            id: 7,
            pid: Some(42),
            app_id: "firefox".to_owned()
        }))
    );
    assert_eq!(focused("{\"Ok\": {\"FocusedWindow\": null}}"), Some(None));
    assert_eq!(focused("not niri"), None);
}

#[test]
fn kitty_is_asked_where_it_was_told_to_listen_else_by_its_pid() {
    let words = |line: &str| line.split(' ').map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(
        listens_on(
            &words("kitty --listen-on unix:/run/user/1000/k.sock --class x"),
            9
        ),
        "unix:/run/user/1000/k.sock"
    );
    assert_eq!(
        listens_on(&words("kitty --listen-on=unix:@agentic-1"), 9),
        "unix:@agentic-1"
    );
    assert_eq!(listens_on(&words("kitty"), 9), "unix:@kitty-9");
}

#[test]
fn only_a_kitty_listening_at_an_abstract_address_answers_alone() {
    assert!(kittys_alone("unix:@kitty-9"), "kitty.conf's own address");
    for address in [
        "unix:/run/user/1000/pi-sessions/kitty.sock",
        "tcp:localhost:5000",
        "unix:",
        "",
    ] {
        assert!(
            !kittys_alone(address),
            "{address:?} could be taken by another program"
        );
    }
}

#[test]
fn only_the_process_listening_at_kittys_address_holds_it() {
    // A squatter binds kitty's foretold name first, then a second name
    // whose tail, written byte for byte into /proc/net/unix, forges a row
    // naming kitty's address and a socket kitty does hold, put ahead of
    // the squatter's own row (findings 26, 28 and 31 on #6643). Here the
    // test is the squatter, and a shell holding one end of a socket pair
    // is the kitty.
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::{SocketAddr, UnixListener};
    let bound = |name: &str| {
        SocketAddr::from_abstract_name(name).and_then(|at| UnixListener::bind_addr(&at))
    };
    let name = format!("squat-{}", std::process::id());
    let address = format!("unix:@{name}");
    let _squatter = bound(&name).expect("the test binds kitty's address first");
    let (held, kept) = UnixStream::pair().unwrap();
    let socket = std::fs::read_link(format!("/proc/self/fd/{}", held.as_raw_fd())).unwrap();
    let inode = socket
        .to_string_lossy()
        .trim_start_matches("socket:[")
        .trim_end_matches(']')
        .to_owned();
    // The shell reads its end until the test's goes, as the test ends.
    let mut kitty = Command::new("/bin/sh")
        .args(["-c", "read held"])
        .stdin(Stdio::from(OwnedFd::from(held)))
        .spawn()
        .expect("the kitty starts");
    let fake = format!("0: 2 0 10000 1 01 {inode} @{name}");
    let _forging = (0..256)
        .find_map(|nth| {
            let forging = bound(&format!("f{nth}\n{fake}")).ok()?;
            let unix = std::fs::read_to_string("/proc/net/unix").ok()?;
            let rows: Vec<&str> = unix.lines().collect();
            let at_forged = rows.iter().position(|at| *at == fake)?;
            let at_squatter = rows
                .iter()
                .position(|at| *at != fake && at.ends_with(&format!(" @{name}")))?;
            (at_forged < at_squatter).then_some(forging)
        })
        .expect("a forged row comes ahead of the squatter's");
    let kittys = kitty_holds(kitty.id(), &address);
    let squatters = kitty_holds(std::process::id(), &address);
    drop(kept);
    kitty.wait().expect("the kitty ends with its socket");
    assert!(
        !kittys,
        "a forged row of the socket table was taken for kitty's listener"
    );
    assert!(
        squatters,
        "the process listening at the address wasn't found to hold it"
    );
    assert!(
        !kitty_holds(std::process::id(), &format!("unix:@{name}-free")),
        "an address nothing listens at was taken to be held"
    );
    assert!(
        !kitty_holds(std::process::id(), "unix:/run/user/1000/kitty.sock"),
        "a socket's file was taken to be held"
    );
}

#[test]
fn only_the_focused_windows_foreground_is_read() {
    let listed = json!([
        {"is_focused": false, "tabs": [{"is_focused": true, "windows": [
            {"is_focused": true, "foreground_processes": [{"pid": 1, "cmdline": ["pi"]}]}]}]},
        {"is_focused": true, "tabs": [
            {"is_active": false, "windows": [
                {"is_active": true, "foreground_processes": [{"pid": 2, "cmdline": ["pi"]}]}]},
            {"is_active": true, "windows": [
                {"is_active": false, "foreground_processes": [{"pid": 3, "cmdline": ["pi"]}]},
                {"is_active": true, "foreground_processes": [{"pid": 4, "cmdline": ["-zsh"]}]}]}]}
    ]);
    assert_eq!(
        foreground(&listed.to_string()),
        Some(vec![(Some(4), vec!["-zsh".to_owned()])])
    );
    assert_eq!(foreground("[]"), None);
}

#[test]
fn a_kitty_window_stands_alone_only_in_a_group_of_its_own() {
    let listed = |groups: Value, window: Value| {
        json!([{"is_focused": true, "tabs": [{"is_active": true, "groups": groups,
            "windows": [window]}]}])
        .to_string()
    };
    let focused = json!({"id": 3, "is_active": true});
    assert_eq!(
        kitty_window(&listed(
            json!([{"windows": [1]}, {"windows": [3]}]),
            focused.clone()
        )),
        Some((3, true))
    );
    // An overlay shares its window's group, in front of it or behind.
    assert_eq!(
        kitty_window(&listed(json!([{"windows": [3, 4]}]), focused.clone())),
        Some((3, false))
    );
    assert_eq!(
        kitty_window(&listed(json!([{"windows": [4, 3]}]), focused.clone())),
        Some((3, false))
    );
    // Groups kitty doesn't name can't say it stands alone.
    assert_eq!(
        kitty_window(&listed(json!(null), focused)),
        Some((3, false))
    );
    assert_eq!(
        kitty_window(&listed(
            json!([{"windows": [3]}]),
            json!({"is_active": true})
        )),
        None
    );
}

#[test]
fn a_session_is_pi_or_mycelium_by_name_or_by_any_word() {
    let words = |line: &str| line.split(' ').map(str::to_owned).collect::<Vec<_>>();
    assert!(a_sessions("pi", &[]), "a process named pi wasn't a session");
    assert!(
        a_sessions("", &words("/nix/store/x-mycelium/bin/mycelium attach 1")),
        "mycelium as its command's first word wasn't a session"
    );
    assert!(
        a_sessions("ssh", &words("ssh -t far-machine mycelium attach 1")),
        "mycelium as a later word of ssh's command wasn't a session"
    );
    assert!(
        !a_sessions("zsh", &words("-zsh")),
        "a login zsh was taken for a session"
    );
    assert!(
        !a_sessions("nvim", &words("nvim pipeline.md")),
        "nvim on a file was taken for a session"
    );
}

#[test]
fn an_agents_process_is_known_by_its_name_its_runtime_or_its_slice() {
    let user = "0::/user.slice/user-1000.slice/user@1000.service";
    let words = |line: &str| line.split(' ').map(str::to_owned).collect::<Vec<_>>();
    let hosted = format!("{user}/agents.slice/pi-conversation-work-background@01a1.service");
    let command = format!("{user}/jobs.slice/jobs-work.slice/pi-spawner-3820109-dc5d.scope");
    let kittys = format!("{user}/agents.slice/kitty-1-1.scope");
    let app = format!("{user}/app.slice/app-gio-google-chrome-2349662.scope");
    assert!(
        agents("node", &words("/nix/store/x-nodejs/bin/node"), &hosted),
        "a hosted session's node in the agents' slice wasn't an agent"
    );
    assert!(
        agents(
            "node",
            &words("/nix/store/x-pi-host-runtime-1.5.0/libexec/pi-host/node"),
            ""
        ),
        "the hosted runtime, out of its slice, wasn't an agent"
    );
    assert!(
        agents("chromium", &words("chromium --headless"), &hosted),
        "a browser a hosted session opened wasn't an agent's"
    );
    assert!(
        agents("pi-spawner", &[], &app),
        "pi-spawner wasn't an agent by its name"
    );
    assert!(
        agents("bash", &words("bash -c make"), &command),
        "a command in the jobs' slice wasn't an agent's"
    );
    assert!(
        agents("zsh", &words("-zsh"), &kittys),
        "a shell in a scope named as kitty's window, in the agents' slice, wasn't an agent's"
    );
    assert!(
        !agents("chrome", &words("/nix/store/x-google-chrome/chrome"), &app),
        "Joel's own Chrome was taken for an agent"
    );
    assert!(
        agents_group(&format!("{user}/agents.slice/sessions-kitty.service")),
        "the sessions' kitty's own unit in the agents' slice wasn't an agent's"
    );
}

#[test]
fn a_kitty_from_kittens_own_package_in_the_agents_slice_is_an_agents() {
    // An agent can run kitty's own program from its package, or start it
    // in the agents' slice, so a kitty there is an agent's like anything
    // else, and nothing is typed into its windows.
    let user = "0::/user.slice/user-1000.slice/user@1000.service";
    let sessions_kitty = format!("{user}/agents.slice/sessions-kitty.service");
    let hosted = format!("{user}/agents.slice/pi-conversation-work-background@01a1.service");
    let package = Path::new("/nix/store/k-kitty-0.50.0");
    let kitty = "/nix/store/k-kitty-0.50.0/bin/.kitty-wrapped";
    let node = "/nix/store/x-nodejs/bin/node";
    let words = |exe: &str| vec![exe.to_owned()];
    for group in [&sessions_kitty, &hosted] {
        assert!(
            kitty_believed(".kitty-wrapped", kitty, group, Some(package)),
            "a kitty from kitten's package in {group} wasn't the one kitten would ask"
        );
        assert!(
            agents(".kitty-wrapped", &words(kitty), group),
            "a kitty from kitten's package in {group} wasn't an agent's"
        );
    }
    assert!(
        agents("kitty", &words(node), &hosted),
        "an agent renamed kitty in the agents' slice was let through"
    );
}

#[test]
fn a_multiplexer_is_known_by_its_name_or_its_first_word() {
    let words = |line: &str| line.split(' ').map(str::to_owned).collect::<Vec<_>>();
    assert!(
        multiplexes("tmux: client", &[]),
        "tmux's client wasn't a multiplexer"
    );
    assert!(
        multiplexes("", &words("/nix/store/x-zellij/bin/zellij attach")),
        "zellij by its path wasn't a multiplexer"
    );
    assert!(
        multiplexes("screen", &words("screen -r")),
        "screen wasn't a multiplexer"
    );
    assert!(
        !multiplexes("nvim", &words("nvim tmux.conf")),
        "nvim editing tmux's config was taken for a multiplexer"
    );
}

#[test]
fn a_journal_line_names_the_entry_and_window_on_one_line_each() {
    let line = journal_line("Git\nHub", "fire\nfox", "didn't type", "no\nwhy");
    assert_eq!(
        line.lines().count(),
        6,
        "a newline in a value started a field: {line}"
    );
    assert!(line.contains("PASSWORDS_TYPED_TITLE=Git Hub\n"), "{line}");
    assert!(line.contains("PASSWORDS_TYPED_APP_ID=fire fox\n"), "{line}");
    assert!(
        line.contains("MESSAGE=didn't type \"Git Hub\" into fire fox: no why\n"),
        "{line}"
    );
}

#[test]
fn a_type_refused_before_a_window_was_read_names_none() {
    let line = journal_line("GitHub", "", "didn't type", "a type is already under way");
    assert!(
        line.contains("MESSAGE=didn't type \"GitHub\": a type is already under way\n"),
        "{line}"
    );
}

#[test]
fn an_x11_window_is_known_by_what_speaks_for_every_x_program() {
    for name in ["xwayland-satell", "xwayland-satellite", "Xwayland"] {
        assert!(x11s(name), "{name} wasn't taken for X11's");
    }
    for name in ["foot", "kitty", "xwayland"] {
        assert!(!x11s(name), "{name} was taken for X11's");
    }
}

#[test]
fn a_kitty_is_believed_only_from_kittens_package_and_outside_the_jobs_slice() {
    let kitten = "/nix/store/abc-kitty-0.50.0/bin/kitten";
    let from = kittys_package(kitten);
    assert_eq!(from, Some(Path::new("/nix/store/abc-kitty-0.50.0")));
    let exe = "/nix/store/abc-kitty-0.50.0/bin/.kitty-wrapped";
    let joels = "0::/agents.slice/sessions-kitty.service";
    assert!(
        kitty_believed(".kitty-wrapped", exe, joels, from),
        "Joel's kitty"
    );
    assert!(
        kitty_believed("kitty", exe, joels, from),
        "kitty by its own name"
    );
    let renamed = [
        (
            ".kitty-wrapped",
            exe,
            "0::/jobs.slice/jobs-work.slice/work-1.scope",
            "in jobs",
        ),
        ("kitty", "/tmp/kitty", joels, "from elsewhere"),
        (
            "kitty",
            "/nix/store/abc-kitty-0.50.0-evil/bin/kitty",
            joels,
            "a lookalike",
        ),
        ("foot", exe, joels, "another name"),
    ];
    for (name, exe, group, which) in renamed {
        assert!(
            !kitty_believed(name, exe, group, from),
            "{which} was believed"
        );
    }
    assert!(
        !kitty_believed("kitty", exe, joels, None),
        "a kitty was believed with no kitten named"
    );
    assert_eq!(kittys_package("kitten"), None, "a bare name has no package");
}
