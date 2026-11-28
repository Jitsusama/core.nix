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

/// /proc/net/unix's head, as Linux writes it above its rows.
const UNIX_HEAD: &str = "Num       RefCount Protocol Flags    Type St Inode Path";

/// A row of /proc/net/unix: a socket's flags, state, inode and path. Linux
/// pads the inode to five, but rows are read by their whitespace alone, so
/// it's written bare, as commands-agree asks of every column in a command.
fn unix_row(flags: &str, state: &str, inode: u64, path: &str) -> String {
    format!("0000000000000000: 00000002 00000000 {flags} 0001 {state} {inode} {path}")
}

#[test]
fn a_socket_listening_at_an_abstract_name_is_found_by_it() {
    let unix = [
        UNIX_HEAD.to_owned(),
        unix_row("00010000", "01", 4100, "@kitty-90"),
        unix_row("00010000", "01", 41001, "@kitty-9"),
        unix_row("00000000", "01", 41002, ""),
    ]
    .join("\n");
    assert_eq!(
        listening_at(&unix, "@kitty-9"),
        Some(41001),
        "kitty-9's listening socket wasn't found by its name in {unix}"
    );
}

#[test]
fn a_connected_socket_at_the_name_isnt_taken_for_the_listening_one() {
    // A connection kitty accepted shows its listener's name too.
    let unix = [
        UNIX_HEAD.to_owned(),
        unix_row("00000000", "03", 41002, "@kitty-9"),
        unix_row("00010000", "03", 41003, "@kitty-9"),
    ]
    .join("\n");
    assert_eq!(
        listening_at(&unix, "@kitty-9"),
        None,
        "a connected socket was taken for one listening in {unix}"
    );
}

#[test]
fn a_socket_file_is_never_found_listening() {
    // A socket's file can be removed and bound again by any program.
    let unix = [
        UNIX_HEAD.to_owned(),
        unix_row("00010000", "01", 41004, "/run/user/1000/kitty-9"),
    ]
    .join("\n");
    for name in ["/run/user/1000/kitty-9", "@kitty-9", "kitty-9"] {
        assert_eq!(
            listening_at(&unix, name),
            None,
            "the socket file was found listening at {name:?}"
        );
    }
}

#[test]
fn a_torn_row_of_the_socket_table_is_passed_over() {
    let unix = [
        UNIX_HEAD.to_owned(),
        "0000000000000000: 00000002 00000000 00010000 0001 01".to_owned(),
        "0000000000000000: 00000002 00000000 0001".to_owned(),
        "0000000000000000: 00000002 00000000 zz010000 0001 01 41005 @kitty-9".to_owned(),
        unix_row("00010000", "01", 41006, "@kitty-9"),
    ]
    .join("\n");
    assert_eq!(
        listening_at(&unix, "@kitty-9"),
        Some(41006),
        "a torn row wasn't passed over for the whole one in {unix}"
    );
    assert_eq!(
        listening_at(
            "0000000000000000: 00000002 00000000 00010000 0001 01 410",
            "@kitty-9"
        ),
        None,
        "a row torn before its path was found"
    );
}

#[test]
fn only_a_socket_among_kittys_own_files_is_held_by_it() {
    let links = [
        "/dev/null",
        "pipe:[7]",
        "socket:[41001]",
        "anon_inode:[eventfd]",
    ]
    .map(str::to_owned);
    assert!(
        holds_socket(&links, 41001),
        "kitty's own socket wasn't found among its files"
    );
    for inode in [4100, 410_011, 7] {
        assert!(
            !holds_socket(&links, inode),
            "socket {inode} was taken for one among kitty's files"
        );
    }
    assert!(
        !holds_socket(&[], 41001),
        "a kitty whose files couldn't be read was taken to hold its socket"
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
