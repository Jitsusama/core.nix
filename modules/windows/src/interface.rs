//! The windows daemon as its clients know it, in the shell's daemon
//! interface (RSCH-20261029-0E8050, "Daemons: One Interface"), which
//! daemon-serve answers for it: the kinds of record it keeps, which of
//! them are one record keyed by the kind alone, and which an ask gets
//! only by naming them.

use daemon_serve::asks::Domain;

/// The kinds of record this daemon keeps.
pub const KINDS: [&str; 8] = [
    "window",
    "workspace",
    "desktop",
    "focus",
    "cast",
    "layout",
    "screenshot",
    "row",
];

/// The kinds of which there's one record, keyed by its kind alone.
const SOLE: [&str; 3] = ["desktop", "focus", "screenshot"];

/// The kinds an ask gets only by naming them: a window's tile moves at
/// every resize and scroll, and the window niri says has the keys moves
/// to none at every opening of the launcher, which would wake every watch
/// that names no kind, where only the few that draw tiles, or follow the
/// keys as niri does, want them. A row is the launcher's alone, ready to
/// show, so only a watch of its place wants it.
const NAMED_ONLY: [&str; 3] = ["focus", "layout", "row"];

/// What daemon-serve serves it as.
pub const DOMAIN: Domain = Domain {
    name: "windows",
    kinds: &KINDS,
    sole: &SOLE,
    named_only: &NAMED_ONLY,
};

#[cfg(test)]
mod tests {
    use daemon_serve::asks::{Ask, place_of};
    use serde_json::json;

    use super::*;

    #[test]
    fn a_watch_names_keys_of_its_kinds() {
        assert_eq!(
            DOMAIN.ask(r#"{"id":1,"ask":"watch","keys":["window:12","desktop"]}"#),
            Ask::Watch {
                id: json!(1),
                kinds: vec![
                    "window".into(),
                    "workspace".into(),
                    "desktop".into(),
                    "cast".into(),
                    "screenshot".into()
                ],
                keys: Some(vec!["window:12".into(), "desktop".into()]),
                places: None,
                fields: None,
                every: None,
            }
        );
    }

    #[test]
    fn layouts_focus_and_rows_are_watched_only_when_named() {
        let Ask::Watch { kinds, .. } =
            DOMAIN.ask(r#"{"id":4,"ask":"watch","kind":["window","layout"]}"#)
        else {
            panic!("a watch of windows and layouts wasn't read as a watch");
        };
        assert_eq!(kinds, vec!["window".to_owned(), "layout".to_owned()]);
        let Ask::Watch { kinds, .. } = DOMAIN.ask(r#"{"id":4,"ask":"watch"}"#) else {
            panic!("a watch of every kind wasn't read as a watch");
        };
        for named in NAMED_ONLY {
            assert!(
                !kinds.iter().any(|kind| kind == named),
                "a watch naming no kind takes {named}: {kinds:?}"
            );
        }
    }

    #[test]
    fn the_screenshot_is_keyed_by_its_kind_alone() {
        assert_eq!(DOMAIN.kind_of("screenshot"), Some("screenshot"));
        assert_eq!(DOMAIN.kind_of("screenshot:1"), None);
        assert_eq!(DOMAIN.kind_of("focus"), Some("focus"));
        assert_eq!(DOMAIN.kind_of("cast:3"), Some("cast"));
        assert_eq!(DOMAIN.kind_of("layout:12"), Some("layout"));
    }

    #[test]
    fn a_row_names_its_place() {
        assert_eq!(DOMAIN.kind_of("row:displays:display:eDP-1"), Some("row"));
        assert_eq!(place_of("row:displays:display:eDP-1"), Some("displays"));
    }

    #[test]
    fn a_key_of_another_kind_is_left_out() {
        assert_eq!(
            DOMAIN.ask(
                r#"{"id":3,"ask":"get","keys":["workspace:2","disk:nvme0n1","window:","desktop:1"]}"#
            ),
            Ask::Get {
                id: json!(3),
                keys: vec!["workspace:2".into()]
            }
        );
    }

    #[test]
    fn a_kind_it_doesnt_keep_is_refused() {
        assert_eq!(
            DOMAIN.ask(r#"{"id":2,"ask":"list","kind":"disk"}"#),
            Ask::UnknownKind {
                id: json!(2),
                kind: "disk".into()
            }
        );
    }
}
