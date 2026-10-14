//! Going to a session: the harness's registry says which terminal holds a
//! live session, so a burn's action brings it forward. kitty is asked to
//! focus the session's window over its socket, then niri to focus kitty's.
//! Nothing here changes the session itself.

use serde_json::Value;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub pid: u32,
    pub driver: String,
    /// The terminal's own id for the session's window.
    pub window: String,
    /// Where the terminal listens, as kitty names a socket.
    pub scope: String,
}

/// Where a session is, from its registry entry, by its id or the start of
/// one.
pub fn find(registry: &Path, session: &str) -> Result<Place, String> {
    let entries = std::fs::read_dir(registry).map_err(|error| {
        format!(
            "reading the session registry {}: {error}",
            registry.display()
        )
    })?;
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".json") && name.starts_with(session) {
            found.push(entry.path());
        }
    }
    let path = match found.as_slice() {
        [one] => one.clone(),
        [] => {
            return Err(format!(
                "no open session's id starts {session}: it may have ended"
            ));
        }
        _ => {
            return Err(format!(
                "{} open sessions' ids start {session}",
                found.len()
            ));
        }
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    if closed(&text) {
        return Err("that session has ended: its log is all that's left".into());
    }
    parse(&text).ok_or_else(|| format!("{} doesn't say where its session is", path.display()))
}

pub fn parse(text: &str) -> Option<Place> {
    let entry: Value = serde_json::from_str(text).ok()?;
    let terminal = entry.get("terminal")?;
    let text_of = |value: Option<&Value>| match value? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    };
    Some(Place {
        pid: entry.get("process")?.get("pid")?.as_u64()? as u32,
        driver: text_of(terminal.get("driverId"))?,
        window: text_of(terminal.get("value"))?,
        scope: text_of(terminal.get("scope"))?,
    })
}

/// Whether the entry says its session closed after it last opened.
pub fn closed(text: &str) -> bool {
    let Ok(entry) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    let at = |key: &str| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .and_then(crate::time::parse)
    };
    match (at("openedAt"), at("closedAt")) {
        (Some(opened), Some(closed)) => closed > opened,
        (None, Some(_)) => true,
        _ => false,
    }
}

/// The terminal's own process, from the socket it names: kitty's is
/// `unix:@kitty-<pid>`.
pub fn terminal_pid(scope: &str) -> Option<u32> {
    scope.rsplit('-').next()?.parse().ok()
}

/// The niri window whose process is `pid`.
pub fn niri_window(windows: &str, pid: u32) -> Option<u64> {
    let windows: Value = serde_json::from_str(windows).ok()?;
    windows
        .as_array()?
        .iter()
        .find(|window| window.get("pid").and_then(Value::as_u64) == Some(pid as u64))
        .and_then(|window| window.get("id")?.as_u64())
}

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("running {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn go(registry: &Path, session: &str) -> Result<(), String> {
    let place = find(registry, session)?;
    if place.driver != "kitty" {
        return Err(format!(
            "the session is in {}, which can't be brought forward",
            place.driver
        ));
    }
    run(
        "kitty",
        &[
            "@",
            "--to",
            &place.scope,
            "focus-window",
            "--match",
            &format!("id:{}", place.window),
        ],
    )?;
    // Outside niri there's no compositor to ask; kitty has focused the tab.
    if let (Some(pid), Ok(windows)) = (
        terminal_pid(&place.scope),
        run("niri", &["msg", "--json", "windows"]),
    ) && let Some(id) = niri_window(&windows, pid)
    {
        run(
            "niri",
            &["msg", "action", "focus-window", "--id", &id.to_string()],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registry_entry_says_where_its_session_is() {
        let entry = r#"{"sessionId":"01a1","process":{"pid":4242,"startToken":"x"},
            "terminal":{"driverId":"kitty","value":"7","scope":"unix:@kitty-1717"}}"#;
        let place = parse(entry).unwrap();
        assert_eq!(place.window, "7");
        assert_eq!(terminal_pid(&place.scope), Some(1717));
        let windows = r#"[{"id":3,"pid":99},{"id":12,"pid":1717}]"#;
        assert_eq!(niri_window(windows, 1717), Some(12));
        assert!(!closed(entry));
        assert!(closed(
            r#"{"openedAt":"2026-10-07T05:14:13Z","closedAt":"2026-10-10T18:39:44Z"}"#
        ));
    }
}
