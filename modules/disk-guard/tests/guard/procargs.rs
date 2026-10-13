use disk_guard::procargs::{Args, parse};
use std::ffi::OsString;

/// A buffer as macOS's `KERN_PROCARGS2` gives it: the argument count, the
/// executable's path, padding, the arguments, the environment, and the
/// strings the kernel adds for the loader after an empty one.
fn buffer(argc: i32, path: &str, words: &[&str]) -> Vec<u8> {
    let mut bytes = argc.to_ne_bytes().to_vec();
    bytes.extend(path.as_bytes());
    bytes.extend([0; 7]);
    for word in words {
        bytes.extend(word.as_bytes());
        bytes.push(0);
    }
    bytes
}

fn os(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

fn keep_cargo(name: &[u8]) -> bool {
    name.starts_with(b"CARGO_")
}

#[test]
fn the_arguments_and_the_kept_variables_are_read() {
    let bytes = buffer(
        3,
        "/usr/bin/cargo",
        &[
            "cargo",
            "build",
            "--release",
            "HOME=/Users/u",
            "CARGO_TARGET_DIR=/Users/u/target",
            "CARGO_HOME=/Users/u/.cargo=odd",
            "",
            "executable_path=/usr/bin/cargo",
            "CARGO_LATE=not the environment",
        ],
    );
    assert_eq!(
        parse(&bytes, keep_cargo),
        Some(Args {
            argv: os(&["cargo", "build", "--release"]),
            env: vec![
                ("CARGO_TARGET_DIR".into(), "/Users/u/target".into()),
                ("CARGO_HOME".into(), "/Users/u/.cargo=odd".into()),
            ],
        })
    );
}

#[test]
fn an_environment_running_to_the_end_is_read_whole() {
    let bytes = buffer(1, "/bin/sh", &["sh", "CARGO_HOME=/c"]);
    let args = parse(&bytes, keep_cargo).unwrap();
    assert_eq!(args.argv, os(&["sh"]));
    assert_eq!(args.env, vec![("CARGO_HOME".into(), "/c".into())]);
}

#[test]
fn a_buffer_too_short_for_its_count_is_refused() {
    assert_eq!(parse(&[1, 0], keep_cargo), None);
    assert_eq!(
        parse(&buffer(4, "/bin/sh", &["sh", "-c"]), keep_cargo),
        None
    );
    assert_eq!(parse(&buffer(-1, "/bin/sh", &[]), keep_cargo), None);
}
