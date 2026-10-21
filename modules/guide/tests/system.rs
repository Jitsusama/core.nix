//! The real machine handing secrets to a real program: each where that
//! program reads it, gone once it's done, and never in the log.

use std::fs;
use std::path::PathBuf;

use guide::system::System;
use guide::{Given, Journal, Machine, Program, Secret};

fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("guide-system-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

/// The shell, by its store path, as a carried program is named.
fn shell() -> String {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join("bash"))
        .filter_map(|bash| fs::canonicalize(bash).ok())
        .find(|bash| bash.starts_with("/nix/store"))
        .expect("bash from the Nix store on PATH")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn each_secret_reaches_its_program_and_no_log() {
    let directory = scratch("given");
    let log = directory.join("log");
    let secrets = directory.join("secrets");
    let mut machine = System::new(Journal::in_memory(), Some(fs::File::create(&log).unwrap()))
        .keeping_secrets_in(secrets.clone());
    let (pin, enrol, typed) = (
        Secret::new("pin-in-the-environment".into()),
        Secret::new("pin-in-a-credential".into()),
        Secret::new("typed-on-stdin".into()),
    );
    let shell = shell();
    // Only bash's own builtins, since nothing else is on a PATH it hasn't got.
    let script = r#"printf '%s|' "$PLACE" "$PIN" "$(<"$CREDENTIALS_DIRECTORY/cryptenroll.fido2-pin")"; read -r line; printf '%s' "$line""#;
    let ran = machine
        .run_given(
            Program::Carried(&shell),
            &["-c", script],
            Given {
                plain: vec![("PLACE", "the-plain-one")],
                env: vec![("PIN", &pin)],
                credentials: vec![("cryptenroll.fido2-pin", &enrol)],
                stdin: Some(&typed),
                // It prints them back, for the test to see they arrived.
                prints_secret: true,
            },
        )
        .unwrap();
    assert_eq!(
        String::from_utf8(ran.stdout).unwrap(),
        "the-plain-one|pin-in-the-environment|pin-in-a-credential|typed-on-stdin"
    );
    assert_eq!(
        fs::read_dir(&secrets).unwrap().count(),
        0,
        "the credentials outlived their program"
    );
    let logged = fs::read_to_string(&log).unwrap();
    assert!(logged.contains("-- with PLACE=the-plain-one"), "{logged}");
    assert!(
        logged.contains("-- given PIN, cryptenroll.fido2-pin, stdin"),
        "{logged}"
    );
    assert!(
        !logged.contains("pin-in-a-credential") && !logged.contains("pin-in-the-environment"),
        "{logged}"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn what_a_program_prints_as_a_secret_stays_out_of_the_log() {
    let directory = scratch("printed");
    let log = directory.join("log");
    let mut machine = System::new(Journal::in_memory(), Some(fs::File::create(&log).unwrap()));
    let ran = machine
        .run_given(
            Program::Carried(&shell()),
            &["-c", "echo the-$((6 * 7))-key"],
            Given {
                prints_secret: true,
                ..Given::default()
            },
        )
        .unwrap();
    assert_eq!(ran.stdout, b"the-42-key\n");
    let logged = fs::read_to_string(&log).unwrap();
    assert!(!logged.contains("the-42-key"), "{logged}");
    assert!(logged.contains("isn't kept"), "{logged}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn what_it_keeps_only_its_owner_can_read() {
    use std::os::unix::fs::PermissionsExt;
    let directory = scratch("kept");
    let mut machine = System::new(Journal::in_memory(), None);
    let file = directory.join("guide/install-key");
    machine.write(file.to_str().unwrap(), b"key").unwrap();
    let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!((mode(file.parent().unwrap()), mode(&file)), (0o700, 0o600));
    machine.remove(file.to_str().unwrap()).unwrap();
    machine.remove(file.to_str().unwrap()).unwrap();
    assert!(!file.exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_carried_program_outside_the_store_is_refused() {
    let mut machine = System::new(Journal::in_memory(), None);
    for path in ["/bin/sh", "/nix/store/../../bin/sh"] {
        assert!(
            machine
                .run_given(Program::Carried(path), &[], Given::default())
                .is_err(),
            "{path}"
        );
    }
}
