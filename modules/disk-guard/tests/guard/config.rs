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
                  "from": "urgent", "protocol": "go-build" },
    "scratch": { "kind": "scratch", "path": "/home/u/.dev/cargo", "healthy": "30d" },
    "snapshots": { "kind": "snapshots", "path": "/home/u/snapshots",
                   "pattern": "^[a-z]+_\\d+_(?P<pid>\\d+)$", "command": "snapd" },
    "gc": { "kind": "producer", "command": ["/bin/gc", "now"],
            "every": "6h", "from": "healthy", "timeout": "5m" },
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
        }
    );
    match &config.sources["gc"] {
        Source::Producer { every, timeout, .. } => {
            assert_eq!(*every, Some(Span(Duration::from_secs(6 * 3600))));
            assert_eq!(*timeout, Span(Duration::from_secs(300)));
        }
        other => panic!("gc read as {other:?}"),
    }
}

#[test]
fn bands_default_to_shares_of_the_filesystem() {
    let config = Config::parse(r#"{"version": 1, "scope": "user", "sources": {}}"#).unwrap();
    assert_eq!(config.bands.pressure, Size::Percent(15.0));
    assert_eq!(config.bands.urgent, Size::Percent(8.0));
    assert_eq!(config.bands.critical, Size::Percent(3.0));
    assert_eq!(config.bands.target, Size::Percent(20.0));
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
            "generations": {"keep": 5, "young": "7d"},
            "dedupe": {"every": "7d"},
            "freeze": true,
            "boot": "/boot",
            "sources": {"cores": {"kind": "coredumps", "path": "/var/lib/systemd/coredump"}}}"#,
    )
    .unwrap();
    assert_eq!(config.scope, Scope::System);
    let system = config.system.unwrap();
    assert_eq!(system.garbage.budget, Size::Percent(10.0));
    assert_eq!(system.generations.keep, 5);
    assert_eq!(system.boot, PathBuf::from("/boot"));
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
        (with(USER, r#"(?P<pid>\\d+)"#, r#"(\\d+)"#), "pid group"),
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
