//! The environment a session's shell starts from, as the user manager
//! holds it at the moment a check runs. The daemon starts with
//! default.target, before the desktop imports the session's own PATH
//! (~/.local/bin among it), and a running service never takes a change
//! to it; so each check reads it afresh rather than keeping the daemon's.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::bounded;

/// How long the user manager has to say its environment. With bounded's
/// grace and a second for each of its two outputs, a check waits at
/// worst about 17 s for it before it starts.
const ASKING: Duration = Duration::from_secs(10);

/// The user manager's environment now, or none where it can't be asked,
/// in which case a check keeps the daemon's own.
pub fn environment(systemctl: &Path) -> Option<Vec<(String, String)>> {
    let mut command = Command::new(systemctl);
    command.args(["--user", "show-environment"]);
    let ended = bounded::run(command, ASKING, bounded::GRACE).ok()?;
    ended
        .status
        .filter(std::process::ExitStatus::success)
        .map(|_| parsed(&String::from_utf8_lossy(&ended.output)))
}

/// The variables `systemctl show-environment` writes, one `NAME=value` a
/// line, a value it must quote written as `$'...'`.
pub fn parsed(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let (name, value) = line.split_once('=')?;
            let value = match value.strip_prefix("$'") {
                Some(quoted) => unquoted(quoted.strip_suffix('\'')?)?,
                None => value.to_owned(),
            };
            Some((name.to_owned(), value))
        })
        .collect()
}

/// A `$'...'` value's insides with its escapes taken out, or none where
/// one isn't one systemd writes: `cescape_char`'s letters, a quote or
/// backslash, and three octal digits for every other control byte.
fn unquoted(quoted: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut chars = quoted.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buffer = [0; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            continue;
        }
        let escaped = match chars.next()? {
            'a' => 0x07,
            'b' => 0x08,
            'f' => 0x0c,
            'n' => b'\n',
            'r' => b'\r',
            't' => b'\t',
            'v' => 0x0b,
            first @ '0'..='3' => {
                let rest: String = chars.by_ref().take(2).collect();
                if rest.len() != 2 {
                    return None;
                }
                u8::from_str_radix(&format!("{first}{rest}"), 8).ok()?
            }
            other @ ('\\' | '\'' | '"') => u8::try_from(other).ok()?,
            _ => return None,
        };
        bytes.push(escaped);
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(name: &str, value: &str) -> (String, String) {
        (name.to_owned(), value.to_owned())
    }

    #[test]
    fn each_line_is_a_variable_and_its_value() {
        let text = "HOME=/home/joel\nPATH=/home/joel/.local/bin:/run/wrappers/bin\nEMPTY=\n";
        assert_eq!(
            parsed(text),
            [
                pair("HOME", "/home/joel"),
                pair("PATH", "/home/joel/.local/bin:/run/wrappers/bin"),
                pair("EMPTY", ""),
            ]
        );
    }

    #[test]
    fn a_quoted_value_has_its_escapes_taken_out() {
        let text = "SAID=$'two words\\nand a \\'quote\\' \\033[0m\\\\'\n";
        assert_eq!(
            parsed(text),
            [pair("SAID", "two words\nand a 'quote' \u{1b}[0m\\")],
            "systemd's letters, quotes and octal bytes should be read as it writes them"
        );
    }

    #[test]
    fn a_line_it_cant_read_is_left_out() {
        let text =
            "no equals\nOPEN=$'never closed\nODD=$'\\q'\nHEX=$'\\x41'\nSHORT=$'\\03'\nKEPT=yes\n";
        assert_eq!(parsed(text), [pair("KEPT", "yes")]);
    }
}
