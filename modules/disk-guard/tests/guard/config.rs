use disk_guard::band::Band;
use disk_guard::config::{CacheProtocol, Config, Scope, Source};
use disk_guard::units::{Size, Span};
use std::path::PathBuf;
use std::time::Duration;

const USER: &str = r#"{
  "version": 1,
  "scope": "user",
  "bands": { "pressure": "15%", "urgent": "8%", "critical": "3%", "target": "20%" },
  "notify": true,
  "status_file": true,
  "sources": {
    "src": { "kind": "cargo", "roots": ["/home/u/src"], "depth": null },
    "trees": { "kind": "cargo", "roots": ["/home/u/trees"], "depth": 3 },
    "go-build": { "kind": "cache", "path": "/home/u/.cache/go-build",
                  "from": "urgent", "protocol": "go-build", "rank": 1 },
    "npm": { "kind": "cache", "path": "/home/u/.npm/_cacache" },
    "scratch": { "kind": "scratch", "path": "/home/u/.dev/cargo", "healthy": "30d" },
    "snapshots": { "kind": "snapshots", "path": "/home/u/snapshots",
                   "pattern": "^[a-z]+_\\d+_(?P<pid>\\d+)$", "command": "snapd" },
    "gc": { "kind": "producer", "command": ["/bin/gc", "now"], "path": "/home/u/src",
            "every": "6h", "from": "healthy", "timeout": "5m" },
    "podman": { "kind": "podman", "command": "/bin/podman",
                "path": "/home/u/.local/share/containers/storage" },
    "merged": { "kind": "finished", "remote": "^https://example\\.com/",
                "command": ["/bin/merged"], "timeout": "20s" }
  }
}"#;

fn with(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the base config has no {from:?}");
    text.replacen(from, to, 1)
}

#[test]
fn a_user_config_is_read() {
    let config = Config::parse(USER).unwrap();
    assert_eq!(config.scope, Scope::User);
    assert_eq!(config.bands.pressure, Size::Percent(15.0));
    assert!(config.notify && config.status_file);
    assert_eq!(
        config.sources["trees"],
        Source::Cargo {
            roots: vec![PathBuf::from("/home/u/trees")],
            depth: Some(3)
        }
    );
    assert_eq!(
        config.sources["go-build"],
        Source::Cache {
            path: PathBuf::from("/home/u/.cache/go-build"),
            from: Band::Urgent,
            protocol: Some(CacheProtocol::GoBuild),
            rank: 1,
        }
    );
    assert_eq!(
        config.sources["npm"],
        Source::Cache {
            path: PathBuf::from("/home/u/.npm/_cacache"),
            from: Band::Urgent,
            protocol: None,
            rank: 50,
        }
    );
    match &config.sources["gc"] {
        Source::Producer {
            every,
            timeout,
            path,
            ..
        } => {
            assert_eq!(path, &PathBuf::from("/home/u/src"));
            assert_eq!(*every, Some(Span(Duration::from_hours(6))));
            assert_eq!(*timeout, Span(Duration::from_secs(300)));
        }
        other => panic!("gc read as {other:?}"),
    }
    assert_eq!(
        config.sources["podman"],
        Source::Podman {
            command: PathBuf::from("/bin/podman"),
            path: PathBuf::from("/home/u/.local/share/containers/storage"),
        }
    );
}

#[test]
fn bands_default_to_shares_of_the_filesystem() {
    let config = Config::parse(r#"{"version": 1, "scope": "user", "sources": {}}"#).unwrap();
    assert_eq!(config.bands.pressure, Size::Percent(25.0));
    assert_eq!(config.bands.urgent, Size::Percent(10.0));
    assert_eq!(config.bands.critical, Size::Percent(4.0));
    assert_eq!(config.bands.target, Size::Percent(35.0));
}

#[test]
fn pacing_defaults_to_a_rest_past_btrfss_commit_every_two_thousand_files() {
    let config = Config::parse(r#"{"version": 1, "scope": "user", "sources": {}}"#).unwrap();
    assert_eq!(config.pacing.files, 2000);
    assert_eq!(config.pacing.rest, Span(Duration::from_secs(30)));
    let quick = Config::parse(
        r#"{"version": 1, "scope": "user", "pacing": {"rest": "1s"}, "sources": {}}"#,
    )
    .unwrap();
    assert_eq!(quick.pacing.rest, Span(Duration::from_secs(1)));
    assert_eq!(quick.pacing.files, 2000);
}

#[test]
fn pacing_refuses_a_batch_of_no_files() {
    let refused =
        Config::parse(r#"{"version": 1, "scope": "user", "pacing": {"files": 0}, "sources": {}}"#);
    assert_eq!(
        refused.unwrap_err(),
        "pacing needs at least one file a batch"
    );
}

#[test]
fn a_temp_source_with_no_path_is_the_accounts_own() {
    let config =
        Config::parse(r#"{"version": 1, "scope": "user", "sources": {"mine": {"kind": "temp"}}}"#)
            .unwrap();
    assert_eq!(
        config.sources["mine"],
        Source::Temp {
            path: disk_guard::config::account_temp()
        }
    );
}

#[test]
fn a_band_may_be_set_in_bytes() {
    let config = Config::parse(&with(USER, r#""urgent": "8%""#, r#""urgent": 80000000000"#));
    assert_eq!(config.unwrap().bands.urgent, Size::Bytes(80_000_000_000));
}

#[test]
fn a_system_config_carries_its_own_sections() {
    let config = Config::parse(
        r#"{"version": 1, "scope": "system",
            "garbage": {"keep": "3d", "budget": "10%"},
            "dedupe": {"every": "7d"},
            "freeze": true,
            "boot": "/boot",
            "sources": {"cores": {"kind": "coredumps", "path": "/var/lib/systemd/coredump"}}}"#,
    )
    .unwrap();
    assert_eq!(config.scope, Scope::System);
    let system = config.system.unwrap();
    assert_eq!(system.garbage.budget, Size::Percent(10.0));
    assert_eq!(system.boot, PathBuf::from("/boot"));
    assert_eq!(
        system.dedupe.systemd_run,
        Some(PathBuf::from("/run/current-system/systemd/bin/systemd-run")),
        "NixOS's own unless told"
    );
    let alone = Config::parse(
        r#"{"version": 1, "scope": "system", "boot": "/var/empty",
            "dedupe": {"systemd_run": null}, "sources": {}}"#,
    );
    assert_eq!(
        alone.unwrap().system.unwrap().dedupe.systemd_run,
        None,
        "nothing to confine a run, as on a Mac"
    );
    let relative = Config::parse(
        r#"{"version": 1, "scope": "system", "boot": "/boot",
            "dedupe": {"systemd_run": "bin/systemd-run"}, "sources": {}}"#,
    );
    assert!(relative.unwrap_err().contains("bin/systemd-run"));
    assert_eq!(
        config.nix.profiles,
        Some(PathBuf::from("/nix/var/nix/profiles")),
        "the machine's profiles unless told"
    );
}

#[test]
fn either_daemon_says_where_nix_is_and_which_generations_stay() {
    let user = Config::parse(&with(
        USER,
        r#""notify": true"#,
        r#""notify": true, "nix": {"bin": "/n/bin/nix",
            "profiles": "/home/u/.local/state/nix/profiles", "generations": {"keep": 3}}"#,
    ))
    .unwrap();
    assert_eq!(user.nix.bin, PathBuf::from("/n/bin/nix"));
    assert_eq!(user.nix.store, PathBuf::from("/nix/store"));
    assert_eq!(
        user.nix.profiles,
        Some(PathBuf::from("/home/u/.local/state/nix/profiles"))
    );
    assert_eq!(user.nix.generations.keep, 3);
    assert_eq!(
        user.nix.generations.young,
        Span(Duration::from_hours(7 * 24))
    );
    let bare = Config::parse(USER).unwrap();
    assert_eq!(
        bare.nix.bin,
        PathBuf::from("/run/current-system/sw/bin/nix"),
        "the system's own nix unless told"
    );
    assert_eq!(
        bare.nix.profiles, None,
        "an account's profiles only when told"
    );
    for (field, path) in [
        ("bin", "bin/nix"),
        ("store", "/nix/../store"),
        ("profiles", "p"),
    ] {
        let text = with(
            USER,
            r#""notify": true"#,
            &format!(r#""notify": true, "nix": {{"{field}": "{path}"}}"#),
        );
        let error = Config::parse(&text).unwrap_err();
        assert!(error.contains(path), "{error:?}");
    }
}

#[test]
fn refusals_say_why() {
    let refused = [
        (
            with(USER, r#""notify": true"#, r#""notify": true, "loud": true"#),
            "loud",
        ),
        (
            with(USER, r#""kind": "cargo""#, r#""kind": "everything""#),
            "everything",
        ),
        (with(USER, r#""/home/u/src""#, r#""src""#), "absolute"),
        (
            with(USER, r#""/home/u/src""#, r#""/home/u/../src""#),
            "absolute",
        ),
        (
            with(USER, r#""version": 1"#, r#""version": 2"#),
            "version 2",
        ),
        (with(USER, r"(?P<pid>\\d+)", r"(\\d+)"), "pid group"),
        (
            with(USER, r#""timeout": "5m""#, r#""timeout": "5 minutes""#),
            "span",
        ),
        (
            with(USER, r#""from": "urgent""#, r#""from": "dire""#),
            "dire",
        ),
        (
            with(
                USER,
                r#""status_file": true"#,
                r#""status_file": true, "dedupe": {}"#,
            ),
            "dedupe belongs to the system",
        ),
        (
            with(USER, r#""command": ["/bin/gc", "now"]"#, r#""command": []"#),
            "empty",
        ),
        (
            with(
                USER,
                r#""command": ["/bin/gc", "now"]"#,
                r#""command": ["gc"]"#,
            ),
            "absolute",
        ),
        (
            with(USER, r#""every": "6h", "from": "healthy", "#, ""),
            "neither",
        ),
        (
            with(USER, r#""target": "20%""#, r#""target": "10%""#),
            "target 10% is under pressure 15%",
        ),
        (
            with(USER, r#""critical": "3%""#, r#""critical": "9%""#),
            "critical 9% is over urgent 8%",
        ),
        (
            r#"{"version": 1, "scope": "system", "sources": {}}"#.to_string(),
            "needs boot",
        ),
        (
            r#"{"version": 1, "scope": "system", "boot": "/boot",
                "sources": {"src": {"kind": "cargo", "roots": ["/src"]}}}"#
                .to_string(),
            "doesn't own cargo",
        ),
    ];
    for (text, reason) in refused {
        let error = Config::parse(&text).expect_err(&text);
        assert!(error.contains(reason), "{error:?} doesn't say {reason:?}");
    }
}
