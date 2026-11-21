//! Which unit a window's viewer joins, from its words: mycelium's
//! `attach --host <key> --session <id>` names a pi host's unit, or the unit
//! of its own that the host runs the session's conversation in, found by
//! the slot file that holds the session. How a joined unit is ranked is
//! `src/hosts.rs`'s to say; this changes only as the viewer's words or the
//! hosts' slots do.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

/// The most a slot file holds; anything larger isn't one the host wrote,
/// so it isn't read.
const SLOT_MOST: u64 = 64 << 10;

/// The host unit a viewer's words join, `<prefix><key>.service`, when
/// they are mycelium's `attach --host <key>`.
pub fn viewed(words: &[String], prefix: &str) -> Option<String> {
    let key = host_key(attached(words)?)?;
    Some(format!("{prefix}{key}.service"))
}

/// The conversation unit a viewer's words join, `<prefix><key>@<slot>.service`,
/// when they are mycelium's `attach --host <key> --session <id>` and that
/// unit is `live`. The slot is one whose file in `<slots>/<key>.d` holds
/// the session. The host names a slot for its session, so that file is
/// read first, alone; but a resume may be a slot of its own, so failing
/// it the folder is read, and of several running slots holding the
/// session the newest is taken, as the one the session last resumed in.
pub fn conversation(
    words: &[String],
    prefix: &str,
    slots: &Path,
    live: impl Fn(&str) -> bool,
) -> Option<String> {
    let rest = attached(words)?;
    let key = host_key(rest)?;
    let session = flag(rest, "--session")?;
    let folder = slots.join(format!("{key}.d"));
    let unit = |slot: &str| format!("{prefix}{key}@{slot}.service");
    if is_name(&session) {
        let named = unit(&session);
        if live(&named) && holds(&folder.join(format!("{session}.json")), &session) {
            return Some(named);
        }
    }
    fs::read_dir(&folder)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| Some((unit(path.file_stem()?.to_str()?), path)))
        .filter(|(unit, path)| live(unit) && holds(path, &session))
        .max_by_key(|(_, path)| written(path))
        .map(|(unit, _)| unit)
}

/// Whether `name` can name a slot: a file's name alone, not a path.
fn is_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && name != "." && name != ".."
}

/// When the file at `path` was last written, or the start of time when it
/// can't be said.
fn written(path: &Path) -> SystemTime {
    fs::symlink_metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// The unit a viewer's words join: the conversation's own unit, when
/// `conversations` names its prefix and the hosts' slots and that unit is
/// `live`, or else the host's under `prefix`.
pub fn unit_of(
    words: &[String],
    prefix: &str,
    conversations: Option<(&str, &Path)>,
    live: impl Fn(&str) -> bool,
) -> Option<String> {
    conversations
        .and_then(|(conversations, slots)| conversation(words, conversations, slots, &live))
        .or_else(|| viewed(words, prefix))
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

/// Whether the slot file at `path` holds `session`. Only a file of a slot's
/// size is read, never a link, a pipe or a folder, so nothing put beside
/// the slots can stall or mislead the loop that reads them. The path is
/// checked before it's opened, as opening a pipe would wait, and what was
/// opened is checked again, and read no further than a slot's size, in
/// case it changed between.
fn holds(path: &Path, session: &str) -> bool {
    let is_slot = |meta: &fs::Metadata| meta.file_type().is_file() && meta.len() <= SLOT_MOST;
    fs::symlink_metadata(path)
        .ok()
        .filter(is_slot)
        .and_then(|_| fs::File::open(path).ok())
        .filter(|file| file.metadata().is_ok_and(|meta| is_slot(&meta)))
        .and_then(|file| {
            let mut text = String::new();
            file.take(SLOT_MOST)
                .read_to_string(&mut text)
                .ok()
                .map(|_| text)
        })
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
    fn a_viewer_joins_its_conversations_unit_rather_than_its_host() {
        let slots = std::env::temp_dir().join(format!("joins-unit-{}", std::process::id()));
        let host = slots.join("work-attended.d");
        fs::create_dir_all(&host).unwrap();
        fs::write(
            host.join("0193-ab.json"),
            r#"{"session": {"sessionId": "0193-ab"}}"#,
        )
        .unwrap();
        let line = words("mycelium attach --host work-attended --session 0193-ab");
        let conversations = Some(("pi-conversation-", slots.as_path()));
        let joined = unit_of(&line, "pi-host-", conversations, |_| true);
        let stopped = unit_of(&line, "pi-host-", conversations, |_| false);
        // A slot past what one holds isn't read, though it names the session.
        fs::write(
            host.join("0193-ab.json"),
            format!(
                r#"{{"session": {{"sessionId": "0193-ab"}}, "pad": "{}"}}"#,
                " ".repeat(64 << 10)
            ),
        )
        .unwrap();
        let large = unit_of(&line, "pi-host-", conversations, |_| true);
        fs::remove_dir_all(&slots).unwrap();
        assert_eq!(
            joined.as_deref(),
            Some("pi-conversation-work-attended@0193-ab.service"),
            "a conversation in a unit of its own is what the window joins"
        );
        assert_eq!(
            stopped.as_deref(),
            Some("pi-host-work-attended.service"),
            "with no unit of its own running, the host runs it"
        );
        assert_eq!(
            large.as_deref(),
            Some("pi-host-work-attended.service"),
            "a slot too large to be one is passed over"
        );
        assert_eq!(
            unit_of(&line, "pi-host-", None, |_| true).as_deref(),
            Some("pi-host-work-attended.service"),
            "with no conversations named, the host runs it"
        );
    }

    #[test]
    fn a_session_is_found_in_the_slot_named_for_it_or_its_newest() {
        let slots = std::env::temp_dir().join(format!("joins-newest-{}", std::process::id()));
        let host = slots.join("work-attended.d");
        fs::create_dir_all(&host).unwrap();
        let older = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        let slot = |name: &str, at: std::time::SystemTime| {
            let path = host.join(format!("{name}.json"));
            fs::write(&path, r#"{"session": {"sessionId": "0193-ab"}}"#).unwrap();
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(at)
                .unwrap();
        };
        slot("s2", older);
        slot("s0", std::time::SystemTime::now());
        let line = words("mycelium attach --host work-attended --session 0193-ab");
        let running = |_: &str| true;
        let newest = conversation(&line, "pi-conversation-", &slots, running);
        slot("0193-ab", older);
        let named = conversation(&line, "pi-conversation-", &slots, running);
        fs::remove_dir_all(&slots).unwrap();
        assert_eq!(
            newest.as_deref(),
            Some("pi-conversation-work-attended@s0.service"),
            "of two running slots holding one session, the newest is its resume"
        );
        assert_eq!(
            named.as_deref(),
            Some("pi-conversation-work-attended@0193-ab.service"),
            "a slot named for its session is taken first, without reading the rest"
        );
    }

    #[test]
    fn a_viewer_names_the_conversation_unit_its_session_runs_in() {
        let slots = std::env::temp_dir().join(format!("joins-slots-{}", std::process::id()));
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
        // Only a slot's .json names its session, though another file may hold the same words.
        fs::write(
            host.join("s3.sock"),
            r#"{"session": {"sessionId": "0193-ab"}}"#,
        )
        .unwrap();
        std::os::unix::fs::symlink(host.join("s2.json"), host.join("s4.json")).unwrap();
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
            conversation(&words(line), "pi-conversation-", &slots, |unit| unit
                .ends_with("@s4.service")),
            None,
            "a link isn't a slot the host wrote, so it's passed over"
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
