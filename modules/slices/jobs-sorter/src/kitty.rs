//! Asks a kitty that draws many session windows which of them is which.
//!
//! A window is found by the app id the shell gave its pi, or by the one
//! scope its kitty has (`focus::scope_of`). On 11 November 48 of Joel's 58
//! pi windows were drawn by one kitty under the bare class `pi-spawned`,
//! started with no app id, beside 73 scopes of that kitty, so neither
//! found them: the sorter held none of them and read none back. That
//! kitty knows, though. Each of its windows is an OS window of one tab and
//! one window, and its remote control (modules/kitty's kitty.conf, on
//! `@kitty-<pid>`) says, for each, whether it has the focus, its title and
//! the pid of the program it runs, whose scope is the window's.
//!
//! Only [`ASK`], kitty's read-only `ls`, is ever sent, and only to a socket
//! whose peer is the very kitty niri names for the window ([`is_kitty`]),
//! so a socket planted under that name tells nothing. An answer past
//! [`LARGEST`] or not as kitty writes one is refused.

use std::io::{Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::time::Duration;

use serde_json::Value;

use crate::focus::Window;
use crate::swap_in;

/// kitty's `ls`, the one command sent: it lists windows and changes
/// nothing.
pub const ASK: &[u8] = b"\x1bP@kitty-cmd{\"cmd\":\"ls\",\"version\":[0,50,0]}\x1b\\";

/// What opens and ends each of kitty's remote control messages.
const OPENS: &[u8] = b"\x1bP@kitty-cmd";
const ENDS: &[u8] = b"\x1b\\";

/// The largest answer taken. kitty's `ls` carries each window's
/// environment: 1.8 MB for 74 windows on 11 November.
pub const LARGEST: usize = 16 << 20;

/// How long kitty has to answer.
const WITHIN: Duration = Duration::from_secs(2);

/// One of kitty's OS windows, as its `ls` says.
#[derive(Debug, PartialEq, Eq)]
pub struct Shown {
    pub focused: bool,
    pub title: String,
    pub pid: u32,
}

/// Whether a socket's peer, as `SO_PEERCRED` gives it, is the kitty `kitty`
/// running as the account `me`.
pub fn is_kitty(peer: (u32, u32), kitty: u32, me: u32) -> bool {
    peer == (kitty, me)
}

/// Reads one answer from `from`, up to the end kitty marks it with, as
/// kitty keeps the socket open, refusing one past [`LARGEST`].
pub fn answer(mut from: impl Read) -> Result<Vec<u8>, String> {
    let mut read = Vec::new();
    let mut chunk = vec![0; 1 << 16];
    while !read.ends_with(ENDS) {
        let got = from
            .read(&mut chunk)
            .map_err(|error| format!("kitty didn't answer: {error}"))?;
        if got == 0 {
            break;
        }
        read.extend_from_slice(&chunk[..got]);
        if read.len() > LARGEST {
            return Err(format!("kitty's answer is past {LARGEST} bytes"));
        }
    }
    Ok(read)
}

/// The OS windows in kitty's answer to [`ASK`].
pub fn shown(answer: &[u8]) -> Result<Vec<Shown>, String> {
    let refused = || "kitty's answer isn't one it writes".to_owned();
    let inner = answer
        .strip_prefix(OPENS)
        .and_then(|rest| rest.strip_suffix(ENDS))
        .ok_or_else(refused)?;
    let reply: Value = serde_json::from_slice(inner).map_err(|_| refused())?;
    if reply.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(format!("kitty answered {reply}"));
    }
    let data = reply
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(refused)?;
    let windows: Value = serde_json::from_str(data).map_err(|_| refused())?;
    windows
        .as_array()
        .ok_or_else(refused)?
        .iter()
        .map(|os| one(os).ok_or_else(refused))
        .collect()
}

/// An OS window from kitty's `ls`: its active tab's active window.
fn one(os: &Value) -> Option<Shown> {
    let active = |list: &Value| {
        list.as_array()?
            .iter()
            .find(|each| each.get("is_active").and_then(Value::as_bool) == Some(true))
            .cloned()
    };
    let tab = active(os.get("tabs")?)?;
    let window = active(tab.get("windows")?)?;
    Some(Shown {
        focused: os.get("is_focused")?.as_bool()?,
        title: window.get("title")?.as_str()?.to_owned(),
        pid: u32::try_from(window.get("pid")?.as_u64()?).ok()?,
    })
}

/// The pid running in `window`, which niri says has the focus when
/// `focused`: the one OS window with its title, and the focus too when
/// niri gives it the focus, so a kitty that hasn't heard the focus move
/// yet names nothing rather than the window Joel left.
pub fn pick(window: &Window, focused: bool, shown: &[Shown]) -> Option<u32> {
    let mut matching = shown
        .iter()
        .filter(|each| each.title == window.title && (!focused || each.focused));
    let found = matching.next()?;
    matching.next().is_none().then_some(found.pid)
}

/// Asks the kitty drawing `window`, as the account `me`, which OS windows
/// it has.
pub fn ask(window: &Window, me: u32) -> Result<Vec<Shown>, String> {
    let name = format!("kitty-{}", window.pid);
    let address = SocketAddr::from_abstract_name(name.as_bytes())
        .map_err(|error| format!("@{name}: {error}"))?;
    let mut stream =
        UnixStream::connect_addr(&address).map_err(|error| format!("@{name}: {error}"))?;
    if !swap_in::peer(&stream).is_some_and(|peer| is_kitty(peer, window.pid, me)) {
        return Err(format!("@{name} isn't kitty {}'s own", window.pid));
    }
    stream
        .set_read_timeout(Some(WITHIN))
        .and_then(|()| stream.write_all(ASK))
        .map_err(|error| format!("@{name}: {error}"))?;
    shown(&answer(stream)?)
}

/// The account this runs as.
pub fn me() -> Option<u32> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    swap_in::owner(&status, "Uid:")
}

/// The scope of the program `shown`, its kitty's answer, names as running
/// in `window`, the focused one when `focused`.
pub fn named(window: &Window, focused: bool, shown: &[Shown]) -> Option<String> {
    let pid = pick(window, focused, shown)?;
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    let unit = cgroup.trim().rsplit('/').next()?.to_owned();
    unit.starts_with(&format!("kitty-{}-", window.pid))
        .then_some(unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(data: &str) -> Vec<u8> {
        let reply = serde_json::json!({ "ok": true, "data": data });
        [OPENS, reply.to_string().as_bytes(), ENDS].concat()
    }

    fn os(id: u32, focused: bool, title: &str, pid: u32) -> Value {
        serde_json::json!({
            "id": id,
            "is_focused": focused,
            "tabs": [{
                "is_active": true,
                "windows": [{ "is_active": true, "title": title, "pid": pid, "env": {} }],
            }],
        })
    }

    fn window(title: &str) -> Window {
        Window {
            id: 7,
            app_id: "pi-spawned".into(),
            pid: 2684,
            title: title.into(),
        }
    }

    #[test]
    fn only_kittys_read_only_list_is_ever_asked() {
        let inner = ASK
            .strip_prefix(OPENS)
            .and_then(|rest| rest.strip_suffix(ENDS))
            .expect("the request is framed as kitty's remote control");
        let asked: Value = serde_json::from_slice(inner).expect("the request is JSON");
        assert_eq!(asked["cmd"], "ls", "only ls, which changes nothing");
        assert_eq!(
            asked.as_object().map(serde_json::Map::len),
            Some(2),
            "nothing but the command and the version: {asked}"
        );
    }

    #[test]
    fn a_socket_not_the_windows_own_kitty_is_refused() {
        assert!(is_kitty((2684, 1000), 2684, 1000), "the window's kitty");
        assert!(
            !is_kitty((4242, 1000), 2684, 1000),
            "a socket planted under its name"
        );
        assert!(!is_kitty((2684, 1001), 2684, 1000), "another account's");
    }

    #[test]
    fn an_answer_past_the_largest_is_refused() {
        let large = vec![b'x'; LARGEST + 1];
        assert!(answer(&large[..]).is_err(), "one past the cap");
        assert_eq!(
            answer(&b"short"[..]).as_deref(),
            Ok(&b"short"[..]),
            "one under it is read whole"
        );
    }

    #[test]
    fn an_answer_kitty_wouldnt_write_is_refused() {
        assert!(shown(b"[]").is_err(), "not framed");
        assert!(shown(&framed("not json")).is_err(), "its data isn't JSON");
        assert!(
            shown(&framed("[{\"id\": 1}]")).is_err(),
            "a window without tabs"
        );
        let failed = [OPENS, b"{\"ok\": false, \"error\": \"no\"}", ENDS].concat();
        assert!(shown(&failed).is_err(), "kitty refused");
    }

    #[test]
    fn each_os_window_gives_its_focus_title_and_program() {
        let data = serde_json::json!([os(1, false, "π - one", 11), os(2, true, "π - two", 22)]);
        assert_eq!(
            shown(&framed(&data.to_string())),
            Ok(vec![
                Shown {
                    focused: false,
                    title: "π - one".into(),
                    pid: 11
                },
                Shown {
                    focused: true,
                    title: "π - two".into(),
                    pid: 22
                },
            ])
        );
    }

    #[test]
    fn the_focused_window_is_the_one_kitty_and_niri_agree_on() {
        let shown = |focused_two: bool| {
            vec![
                Shown {
                    focused: !focused_two,
                    title: "π - one".into(),
                    pid: 11,
                },
                Shown {
                    focused: focused_two,
                    title: "π - two".into(),
                    pid: 22,
                },
            ]
        };
        assert_eq!(pick(&window("π - two"), true, &shown(true)), Some(22));
        assert_eq!(
            pick(&window("π - two"), true, &shown(false)),
            None,
            "kitty hasn't heard the focus move yet"
        );
        assert_eq!(
            pick(&window("π - two"), false, &shown(false)),
            Some(22),
            "a window left a moment ago, by its title"
        );
    }

    #[test]
    fn a_title_two_windows_share_names_neither() {
        let twins = [
            Shown {
                focused: false,
                title: "π - same".into(),
                pid: 11,
            },
            Shown {
                focused: false,
                title: "π - same".into(),
                pid: 22,
            },
        ];
        assert_eq!(pick(&window("π - same"), false, &twins), None);
        assert_eq!(
            pick(&window("π - other"), false, &twins),
            None,
            "no title matches"
        );
    }
}
