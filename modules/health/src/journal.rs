//! One entry of the journal as `journalctl -o json` writes it, cut down to
//! what the day is tallied by: when, how grave, which thing said it, and
//! what it said.

use serde_json::{Map, Value};

/// systemd's message for a unit whose restart is scheduled, the one line a
/// restart always leaves, at a priority the warning reader doesn't see.
pub const RESTART_SCHEDULED: &str = "5eb03494b6584870a536b337290809b3";

/// A journal entry, as the day takes it.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    /// When it was written, in microseconds since the epoch.
    pub at: i64,
    /// syslog's priority: 0 is an emergency, 3 an error, 4 a warning.
    pub priority: u8,
    /// Which thing said it: a program's identifier, or the unit systemd
    /// spoke of.
    pub thing: String,
    /// What it said.
    pub message: String,
    /// The unit that wrote it, a service or a program's scope.
    pub ran_in: Option<String>,
    /// The program a core dump was of, when it's one.
    pub crashed: Option<String>,
    /// Whose a core dump's program was, when it's one.
    pub whose: Option<Whose>,
    /// Whether it's systemd scheduling a unit's restart.
    pub restart: bool,
    /// Whether the kernel said it.
    pub kernel: bool,
}

/// Whose a program is, from where systemd ran it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    /// Joel's: in his programs' or his session's slice.
    His,
    /// Agents' and their tools', test browsers' included.
    Agents,
}

/// The entry one line of `journalctl -o json` holds, or none when the line
/// isn't one.
#[must_use]
pub fn parse(line: &str) -> Option<Entry> {
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(line) else {
        return None;
    };
    let text = |name: &str| fields.get(name).and_then(said);
    let at = text("__REALTIME_TIMESTAMP")?.parse().ok()?;
    let priority = text("PRIORITY").and_then(|p| p.parse().ok()).unwrap_or(6);
    let message = text("MESSAGE").unwrap_or_default();
    let identifier = text("SYSLOG_IDENTIFIER").or_else(|| text("_COMM"));
    let unit = text("USER_UNIT").or_else(|| text("UNIT"));
    let thing = match (identifier.as_deref(), unit) {
        (Some("systemd"), Some(unit)) => unit,
        (Some(identifier), _) => identifier.to_owned(),
        (None, Some(unit)) => unit,
        (None, None) => "unnamed".to_owned(),
    };
    let crashed = text("COREDUMP_COMM");
    let whose = crashed.as_ref().map(|_| whose(&fields));
    Some(Entry {
        at,
        priority,
        thing,
        message,
        ran_in: text("_SYSTEMD_USER_UNIT").or_else(|| text("_SYSTEMD_UNIT")),
        crashed,
        whose,
        restart: text("MESSAGE_ID").as_deref() == Some(RESTART_SCHEDULED),
        kernel: text("_TRANSPORT").as_deref() == Some("kernel"),
    })
}

/// A field's words: journald writes a message that isn't UTF-8 as its
/// bytes, which are read as far as they go.
fn said(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(bytes) => {
            let bytes: Vec<u8> = bytes
                .iter()
                .filter_map(Value::as_u64)
                .filter_map(|byte| u8::try_from(byte).ok())
                .collect();
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
        _ => None,
    }
}

/// Whose a crashed program was, from the slice it ran in: his programs and
/// his session are his, everything else (jobs, the background) is agents'.
fn whose(fields: &Map<String, Value>) -> Whose {
    let cgroup = fields.get("COREDUMP_CGROUP").and_then(said).unwrap_or_default();
    if cgroup.contains("/app.slice") || cgroup.contains("/session.slice") {
        Whose::His
    } else {
        Whose::Agents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_warning_is_its_programs() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1794450621321755","PRIORITY":"4",
            "SYSLOG_IDENTIFIER":"pebble-lights","_SYSTEMD_USER_UNIT":"pebble-lights.service",
            "MESSAGE":"light 3 answered in 812 ms"}"#
            .replace('\n', "");
        let entry = parse(&line).expect("a journal line is an entry");
        assert_eq!(entry.thing, "pebble-lights", "the identifier names the thing");
        assert_eq!(entry.priority, 4, "the priority is read");
        assert_eq!(entry.at, 1_794_450_621_321_755, "the time is read");
        assert_eq!(entry.ran_in.as_deref(), Some("pebble-lights.service"), "its unit");
        assert!(!entry.restart && !entry.kernel, "it's neither a restart nor the kernel's");
    }

    #[test]
    fn systemd_speaking_of_a_unit_is_the_units() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
            "USER_UNIT":"merge-net.service","MESSAGE_ID":"5eb03494b6584870a536b337290809b3",
            "MESSAGE":"merge-net.service: Scheduled restart job, restart counter is at 1."}"#
            .replace('\n', "");
        let entry = parse(&line).expect("a journal line is an entry");
        assert_eq!(entry.thing, "merge-net.service", "the unit spoken of is the thing");
        assert!(entry.restart, "systemd's restart message is a restart");
    }

    #[test]
    fn a_crash_says_whose_program_it_was() {
        let his = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"2",
            "SYSLOG_IDENTIFIER":"systemd-coredump","COREDUMP_COMM":"1password",
            "COREDUMP_CGROUP":"/user.slice/user-1000.slice/app.slice/app-1password.scope",
            "MESSAGE":"Process 4169227 (1password) of user 1000 dumped core."}"#
            .replace('\n', "");
        let agents = his
            .replace("1password", "chrome")
            .replace("/app.slice/", "/jobs.slice/jobs-timed.slice/");
        let his = parse(&his).expect("a crash is an entry");
        let agents = parse(&agents).expect("a crash is an entry");
        assert_eq!(his.crashed.as_deref(), Some("1password"), "the program is named");
        assert_eq!(his.whose, Some(Whose::His), "a program in his slice is his");
        assert_eq!(agents.whose, Some(Whose::Agents), "a job's program is agents'");
    }

    #[test]
    fn a_message_in_bytes_is_read_as_far_as_it_goes() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"3","_TRANSPORT":"kernel",
            "SYSLOG_IDENTIFIER":"kernel","MESSAGE":[110,118,109,101,255]}"#
            .replace('\n', "");
        let entry = parse(&line).expect("a kernel line is an entry");
        assert!(entry.message.starts_with("nvme"), "the bytes are read: {}", entry.message);
        assert!(entry.kernel, "the kernel's transport is the kernel's");
    }

    #[test]
    fn a_line_that_isnt_an_entry_is_none() {
        assert_eq!(parse("-- No entries --"), None, "words aren't an entry");
        assert_eq!(parse(r#"{"MESSAGE":"no time"}"#), None, "an entry needs its time");
    }
}
