//! Which unit a window's viewer joins, from its words: mycelium's
//! `attach --host <key> --session <id>` names a pi host's unit, or the unit
//! of its own that the host runs the session's conversation in, found by
//! the slot file that holds the session. How a joined unit is ranked is
//! `src/hosts.rs`'s to say; this changes only as the viewer's words or the
//! hosts' slots do.

use std::fs;
use std::path::Path;

/// The host unit a viewer's words join, `<prefix><key>.service`, when
/// they are mycelium's `attach --host <key>`.
pub fn viewed(words: &[String], prefix: &str) -> Option<String> {
    let key = host_key(attached(words)?)?;
    Some(format!("{prefix}{key}.service"))
}

/// The conversation unit a viewer's words join, `<prefix><key>@<slot>.service`,
/// when they are mycelium's `attach --host <key> --session <id>` and that
/// unit is `live`. The slot is one whose file in `<slots>/<key>.d` holds
/// the session, since each resume of a session is a slot of its own.
pub fn conversation(
    words: &[String],
    prefix: &str,
    slots: &Path,
    live: impl Fn(&str) -> bool,
) -> Option<String> {
    let rest = attached(words)?;
    let key = host_key(rest)?;
    let session = flag(rest, "--session")?;
    fs::read_dir(slots.join(format!("{key}.d")))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json") && holds(path, &session))
        .filter_map(|path| {
            Some(format!(
                "{prefix}{key}@{}.service",
                path.file_stem()?.to_str()?
            ))
        })
        .find(|unit| live(unit))
}

/// The words after mycelium's `attach`, when it's there.
fn attached(words: &[String]) -> Option<&[String]> {
    let attach = words.iter().position(|word| word == "attach")?;
    Some(&words[attach + 1..])
}

/// The host's key a viewer's `--host` gives, when it can name a unit.
fn host_key(rest: &[String]) -> Option<String> {
    flag(rest, "--host").filter(|key| !key.is_empty() && !key.contains('/'))
}

/// The value given to `name` among `rest`, as `name value` or `name=value`.
fn flag(rest: &[String], name: &str) -> Option<String> {
    let joined = format!("{name}=");
    rest.iter().enumerate().find_map(|(at, word)| {
        if word == name {
            rest.get(at + 1).cloned()
        } else {
            word.strip_prefix(joined.as_str()).map(str::to_owned)
        }
    })
}

/// Whether the slot file at `path` holds `session`.
fn holds(path: &Path, session: &str) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some_and(|slot| {
            slot.pointer("/session/sessionId")
                .and_then(serde_json::Value::as_str)
                == Some(session)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_viewer_names_the_host_unit_it_joins() {
        let line = "mycelium --realm work attach --host work-attended --session 0193-ab";
        assert_eq!(
            viewed(&words(line), "pi-host-"),
            Some("pi-host-work-attended.service".to_owned()),
            "attach --host names the host's unit"
        );
        assert_eq!(
            viewed(
                &words("mycelium attach --host=work-background --session 1"),
                "pi-host-"
            ),
            Some("pi-host-work-background.service".to_owned()),
            "the joined form names it too"
        );
        assert_eq!(
            viewed(&words("mycelium --realm work attach h0"), "pi-host-"),
            None,
            "attaching a held program joins no host"
        );
        assert_eq!(
            viewed(&words("ssh --host x attach"), "pi-host-"),
            None,
            "a --host before attach is another program's"
        );
        assert_eq!(
            viewed(&words("mycelium attach --host ../x"), "pi-host-"),
            None,
            "a key that isn't a unit's name names none"
        );
    }

    #[test]
    fn a_viewer_names_the_conversation_unit_its_session_runs_in() {
        let slots = std::env::temp_dir().join(format!("hosts-slots-{}", std::process::id()));
        let host = slots.join("work-attended.d");
        fs::create_dir_all(&host).unwrap();
        // Each resume is a new slot, so the slot is found by the session it holds.
        fs::write(
            host.join("s2.json"),
            r#"{"session": {"sessionId": "0193-ab", "file": "/x.jsonl"}}"#,
        )
        .unwrap();
        fs::write(
            host.join("s1.json"),
            r#"{"session": {"sessionId": "0193-cd"}}"#,
        )
        .unwrap();
        fs::write(host.join("s3.sock"), "").unwrap();
        let running = |_: &str| true;
        let line = "mycelium --realm work attach --host work-attended --session 0193-ab";
        assert_eq!(
            conversation(&words(line), "pi-conversation-", &slots, running),
            Some("pi-conversation-work-attended@s2.service".to_owned()),
            "the session's slot names its conversation's unit"
        );
        assert_eq!(
            conversation(
                &words("mycelium attach --host=work-attended --session=0193-cd"),
                "pi-conversation-",
                &slots,
                running
            ),
            Some("pi-conversation-work-attended@s1.service".to_owned()),
            "the joined forms name it too"
        );
        fs::write(
            host.join("s0.json"),
            r#"{"session": {"sessionId": "0193-ab"}}"#,
        )
        .unwrap();
        assert_eq!(
            conversation(&words(line), "pi-conversation-", &slots, |unit| unit
                .ends_with("@s2.service")),
            Some("pi-conversation-work-attended@s2.service".to_owned()),
            "a slot left from before, whose unit has gone, is passed over"
        );
        assert_eq!(
            conversation(&words(line), "pi-conversation-", &slots, |_| false),
            None,
            "a session whose unit isn't running joins none"
        );
        for (line, why) in [
            (
                "mycelium attach --host work-attended --session 0193-ef",
                "a session no slot holds",
            ),
            ("mycelium attach --host work-attended", "no session"),
            (
                "mycelium attach --host work-background --session 0193-ab",
                "another host",
            ),
            (
                "mycelium attach --host ../work-attended.d --session 0193-ab",
                "a key that's a path",
            ),
            ("mycelium --realm work attach h0", "a held program"),
        ] {
            assert_eq!(
                conversation(&words(line), "pi-conversation-", &slots, running),
                None,
                "{why}"
            );
        }
        fs::remove_dir_all(&slots).unwrap();
    }
}
