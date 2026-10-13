use disk_guard::activity::{Process, Use, judge};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const ME: u32 = 1;
const TARGET: &str = "/home/u/src/app/target";
const CHECKOUT: &str = "/home/u/src/app";

fn process(pid: u32, exe: &str) -> Process {
    Process {
        pid,
        uid: 1000,
        name: Path::new(exe)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        exe: Some(PathBuf::from(exe)),
        cwd: Some(PathBuf::from("/home/u")),
        cmdline: vec![OsString::from(exe)],
        readable: true,
        ..Default::default()
    }
}

fn shell(pid: u32) -> Process {
    process(pid, "/nix/store/abc-zsh/bin/zsh")
}

fn cargo(pid: u32) -> Process {
    process(pid, "/nix/store/abc-cargo/bin/cargo")
}

fn judged(processes: &[Process]) -> Use {
    judge(processes, ME, Path::new(TARGET), Some(Path::new(CHECKOUT)))
}

fn sits(mut process: Process, cwd: &str) -> Process {
    process.cwd = Some(PathBuf::from(cwd));
    process
}

fn carries(mut process: Process, name: &str, value: &str) -> Process {
    process
        .env
        .push((OsString::from(name), OsString::from(value)));
    process
}

#[test]
fn a_tree_nothing_uses_is_free() {
    assert_eq!(judged(&[shell(10), cargo(11)]), Use::Free);
}

#[test]
fn any_process_sitting_in_the_target_makes_it_busy() {
    assert_eq!(
        judged(&[sits(shell(10), "/home/u/src/app/target/debug")]),
        Use::Busy(10)
    );
}

#[test]
fn a_file_open_under_the_target_makes_it_busy() {
    let mut editor = process(10, "/nix/store/abc-nvim/bin/nvim");
    editor
        .files
        .push(PathBuf::from("/home/u/src/app/target/doc/index.html"));
    assert_eq!(judged(&[editor]), Use::Busy(10));
}

#[test]
fn a_program_running_from_the_target_makes_it_busy() {
    let server = process(10, "/home/u/src/app/target/debug/app");
    assert_eq!(judged(&[server]), Use::Busy(10));
}

#[test]
fn a_map_under_the_target_makes_it_busy() {
    let mut test = process(10, "/usr/bin/python3");
    test.maps.push(PathBuf::from(
        "/home/u/src/app/target/debug/deps/libapp-0123456789abcdef.so",
    ));
    assert_eq!(judged(&[test]), Use::Busy(10));
}

#[test]
fn a_proc_macro_servers_map_only_occupies_it() {
    let macros = "/home/u/src/app/target/debug/deps/libderive-0123456789abcdef.so";
    let mut server = process(
        10,
        "/nix/store/abc-rust/libexec/rust-analyzer-proc-macro-srv",
    );
    server.maps.push(PathBuf::from(macros));
    assert_eq!(judged(&[server]), Use::Occupied(10));
    let mut older = process(
        11,
        "/nix/store/abc-rust-analyzer/bin/.rust-analyzer-wrapped",
    );
    older.cmdline.push(OsString::from("proc-macro"));
    older.maps.push(PathBuf::from(macros));
    assert_eq!(judged(&[older]), Use::Occupied(11));
}

#[test]
fn a_shell_in_the_checkout_occupies_it_and_a_build_there_is_busy() {
    assert_eq!(
        judged(&[sits(shell(10), "/home/u/src/app/src")]),
        Use::Occupied(10)
    );
    let build = [sits(shell(10), CHECKOUT), sits(cargo(11), CHECKOUT)];
    assert_eq!(judged(&build), Use::Busy(11));
}

#[test]
fn an_idle_rust_analyzer_only_occupies_the_checkout() {
    let analyzer = process(
        10,
        "/nix/store/abc-rust-analyzer/bin/.rust-analyzer-wrapped",
    );
    assert_eq!(judged(&[sits(analyzer, CHECKOUT)]), Use::Occupied(10));
}

#[test]
fn build_tools_are_known_through_wrappers() {
    // Nix wraps some programs as .name-wrapped, and its cc is a script run
    // by bash, known only by the name it was called by.
    let wrapped = process(10, "/nix/store/abc-clippy/bin/.clippy-driver-wrapped");
    assert_eq!(judged(&[sits(wrapped, CHECKOUT)]), Use::Busy(10));
    let mut script = process(11, "/nix/store/abc-bash/bin/bash");
    script.cmdline = ["/nix/store/abc-gcc-wrapper/bin/cc", "-c", "x.c"]
        .map(OsString::from)
        .into();
    assert_eq!(judged(&[sits(script, CHECKOUT)]), Use::Busy(11));
}

#[test]
fn build_variables_make_a_build_busy_and_a_shell_occupied() {
    for (name, value) in [
        ("CARGO_TARGET_DIR", TARGET),
        ("CARGO_BUILD_BUILD_DIR", "/home/u/src/app/target/build"),
        // A variable naming the directory a target sits in counts too.
        ("CARGO_TARGET_DIR", "/home/u/src/app"),
    ] {
        assert_eq!(
            judged(&[carries(shell(10), name, value)]),
            Use::Occupied(10),
            "{value}"
        );
        let rustc = process(11, "/nix/store/abc-rustc/bin/rustc");
        assert_eq!(
            judged(&[carries(rustc, name, value)]),
            Use::Busy(11),
            "{value}"
        );
    }
}

#[test]
fn a_cargo_home_is_used_by_whatever_names_it() {
    // A dev shell between builds reads nothing there, but its next build
    // would fetch everything again.
    let home = Path::new("/home/u/.dev/cargo/abc-dev-shell");
    let named = |process| carries(process, "CARGO_HOME", "/home/u/.dev/cargo/abc-dev-shell");
    assert_eq!(
        judge(&[named(shell(10))], ME, home, None),
        Use::Occupied(10)
    );
    let rustc = process(11, "/nix/store/abc-rustc/bin/rustc");
    assert_eq!(judge(&[named(rustc)], ME, home, None), Use::Busy(11));
}

#[test]
fn a_relative_build_variable_is_read_from_the_cwd() {
    let inside = carries(sits(shell(10), CHECKOUT), "CARGO_TARGET_DIR", "target");
    let elsewhere = carries(sits(shell(11), CHECKOUT), "CARGO_TARGET_DIR", "../other");
    // No checkout is known, as for an orphaned target.
    assert_eq!(
        judge(&[inside], ME, Path::new(TARGET), None),
        Use::Occupied(10)
    );
    assert_eq!(judge(&[elsewhere], ME, Path::new(TARGET), None), Use::Free);
}

#[test]
fn variables_and_paths_elsewhere_leave_it_free() {
    let next_door = sits(
        carries(cargo(10), "CARGO_TARGET_DIR", "/home/u/src/app/target2"),
        "/home/u/src/application",
    );
    let mut neighbour = shell(11);
    neighbour
        .files
        .push(PathBuf::from("/home/u/src/app/targets/notes"));
    assert_eq!(judged(&[next_door, neighbour]), Use::Free);
}

#[test]
fn an_unreadable_process_leaves_it_unknown_unless_something_is_busy() {
    let hidden = Process {
        pid: 20,
        uid: 0,
        readable: false,
        ..Default::default()
    };
    assert_eq!(
        judged(&[hidden.clone(), sits(shell(10), CHECKOUT)]),
        Use::Unknown(20)
    );
    assert_eq!(judged(&[hidden, sits(cargo(11), CHECKOUT)]), Use::Busy(11));
}

#[test]
fn the_guard_itself_is_left_out() {
    assert_eq!(
        judged(&[sits(
            process(ME, "/nix/store/abc-disk-guard/bin/disk-guard"),
            TARGET
        )]),
        Use::Free
    );
}
