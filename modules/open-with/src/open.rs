//! What a program has open now, for its Recent in the launcher to lead with:
//! each of its processes and the whole path of the file it shows. A zathura
//! says it over the session bus, as `org.pwmt.zathura.PID-<pid>`, whose
//! `filename` is the document it has open, empty while it has none. A
//! Neovim runs in a kitty window, many to one kitty process, so it says its
//! file to that window instead (../../neovim/showing.lua), and kitty's
//! remote control gives it back with niri's window it's in.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::proxy::{Builder, CacheProperties};

/// The name each zathura takes on the session bus, its process id after it.
pub const ZATHURA: &str = "org.pwmt.zathura.PID-";

/// The process a bus name is for, when it's one `prefix` names.
#[must_use]
pub fn pid_of(name: &str, prefix: &str) -> Option<u32> {
    name.strip_prefix(prefix)?.parse().ok()
}

/// Every zathura on the bus with a document open, by its process and the
/// document's path. One quitting as it's asked is passed over, so what's
/// left is still said.
///
/// # Errors
///
/// When the bus can't list its names.
pub async fn zathura(connection: &Connection) -> zbus::Result<Vec<(u32, String)>> {
    let names = DBusProxy::new(connection).await?.list_names().await?;
    let mut open = Vec::new();
    for name in names {
        let Some(pid) = pid_of(name.as_str(), ZATHURA) else {
            continue;
        };
        let Ok(asked) = Builder::<zbus::Proxy>::new(connection)
            .destination(name.as_str())?
            .path("/org/pwmt/zathura")?
            .interface("org.pwmt.zathura")?
            .cache_properties(CacheProperties::No)
            .build()
            .await
        else {
            continue;
        };
        if let Ok(file) = asked.get_property::<String>("filename").await
            && !file.is_empty()
        {
            open.push((pid, file));
        }
    }
    Ok(open)
}

/// The kitty user variable a Neovim says its file in.
pub const NEOVIM_FILE: &str = "neovim_file";

/// How long kitty or niri may take to answer before what it holds is passed
/// over, so a stuck one never holds the launcher.
const ANSWER: Duration = Duration::from_secs(2);

/// A file a Neovim has open: niri's window it shows in, its whole path, and
/// what Go To is given to bring it forward inside kitty (`go`).
#[derive(Debug, PartialEq, Eq)]
pub struct InKitty {
    pub window: u64,
    pub path: String,
    pub go: String,
}

/// Where a kitty listens for its remote control: the `--listen-on` its
/// command line gives, as a program starting its own kitty names one, or
/// else kitty.conf's, by the process.
#[must_use]
pub fn kitty_socket(pid: u32, cmdline: &[String]) -> String {
    let given = cmdline.iter().enumerate().find_map(|(at, argument)| {
        if argument == "--listen-on" {
            return cmdline.get(at + 1).cloned();
        }
        argument.strip_prefix("--listen-on=").map(str::to_owned)
    });
    given.unwrap_or_else(|| format!("unix:@kitty-{pid}"))
}

/// Every file a Neovim has open in a kitty window, from niri's windows and
/// each kitty's `ls`, by its process. A kitty window is niri's by its
/// process and the title of its active tab, which kitty titles the window
/// by; a title two of one kitty's windows share names neither. A file is
/// counted only while a Neovim runs in front in its window, so one said by
/// a Neovim that died without clearing it is passed over.
#[must_use]
pub fn neovims(niri: &[Value], kitties: &[(u32, Value)]) -> Vec<InKitty> {
    let mut open = Vec::new();
    for (pid, listed) in kitties {
        for kitty_window in listed.as_array().into_iter().flatten() {
            let tabs = kitty_window["tabs"]
                .as_array()
                .map_or(&[][..], Vec::as_slice);
            let Some(title) = tabs
                .iter()
                .find(|tab| tab["is_active"] == true)
                .and_then(|tab| tab["title"].as_str())
            else {
                continue;
            };
            let mut titled = niri.iter().filter(|window| {
                window["pid"].as_u64() == Some(u64::from(*pid))
                    && window["title"].as_str() == Some(title)
            });
            let (Some(window), None) = (titled.next(), titled.next()) else {
                continue;
            };
            let Some(window) = window["id"].as_u64() else {
                continue;
            };
            for inside in tabs
                .iter()
                .flat_map(|tab| tab["windows"].as_array().into_iter().flatten())
            {
                let Some(path) = inside["user_vars"][NEOVIM_FILE].as_str() else {
                    continue;
                };
                if path.starts_with('/') && runs_neovim(inside) {
                    open.push(InKitty {
                        window,
                        path: path.to_owned(),
                        go: format!("{pid}:{}", inside["id"]),
                    });
                }
            }
        }
    }
    open
}

/// Whether a Neovim is among what runs in front in a kitty window.
fn runs_neovim(window: &Value) -> bool {
    window["foreground_processes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|process| process["cmdline"][0].as_str())
        .any(|program| {
            Path::new(program)
                .file_name()
                .is_some_and(|name| name == "nvim")
        })
}

/// niri's windows, asked of its socket.
///
/// # Errors
///
/// When niri can't be reached or answers something else.
pub fn niri_windows(socket: &Path) -> io::Result<Vec<Value>> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(ANSWER))?;
    stream.write_all(b"\"Windows\"\n")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let said: Value = serde_json::from_str(&line).map_err(io::Error::other)?;
    said["Ok"]["Windows"]
        .as_array()
        .cloned()
        .ok_or_else(|| io::Error::other(format!("niri answered {line}")))
}

/// Asks a kitty's remote control (`cmd` with its `payload`), and gives back
/// the data it answers with, read as JSON when it's text, or null.
///
/// # Errors
///
/// When the kitty can't be reached, or answers that it couldn't.
pub fn kitty(socket: &str, cmd: &str, payload: &Value) -> io::Result<Value> {
    let place = socket.strip_prefix("unix:").unwrap_or(socket);
    let mut stream = match place.strip_prefix('@') {
        Some(name) => UnixStream::connect_addr(&SocketAddr::from_abstract_name(name)?)?,
        None => UnixStream::connect(place)?,
    };
    stream.set_read_timeout(Some(ANSWER))?;
    stream.set_write_timeout(Some(ANSWER))?;
    let asked = json!({ "cmd": cmd, "version": [0, 49, 2], "payload": payload });
    stream.write_all(format!("\x1bP@kitty-cmd{asked}\x1b\\").as_bytes())?;
    let mut said = Vec::new();
    let mut chunk = vec![0; 1 << 16];
    while !said.ends_with(b"\x1b\\") {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        said.extend_from_slice(&chunk[..read]);
    }
    let body = said
        .strip_prefix(b"\x1bP@kitty-cmd")
        .and_then(|rest| rest.strip_suffix(b"\x1b\\"))
        .ok_or_else(|| io::Error::other("kitty answered out of its frame"))?;
    let answer: Value = serde_json::from_slice(body).map_err(io::Error::other)?;
    if answer["ok"] != true {
        return Err(io::Error::other(format!("kitty said {}", answer["error"])));
    }
    match answer["data"].as_str() {
        Some(text) => serde_json::from_str(text).map_err(io::Error::other),
        None => Ok(Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kitty_window(id: u64, tabs: &[(bool, &str, Value)]) -> Value {
        json!({
            "id": id,
            "tabs": tabs.iter().map(|(active, title, windows)| json!({
                "is_active": active, "title": title, "windows": windows
            })).collect::<Vec<_>>()
        })
    }

    fn inside(id: u64, file: Option<&str>, program: &str) -> Value {
        let vars = file.map_or_else(|| json!({}), |file| json!({ NEOVIM_FILE: file }));
        json!({
            "id": id,
            "user_vars": vars,
            "foreground_processes": [{ "cmdline": [program, "--flag"] }]
        })
    }

    #[test]
    fn a_kitty_listens_where_its_command_line_says() {
        let given = |line: &[&str]| {
            kitty_socket(
                7,
                &line.iter().map(|&one| one.to_owned()).collect::<Vec<_>>(),
            )
        };
        assert_eq!(given(&["kitty", "--listen-on", "unix:@mine"]), "unix:@mine");
        assert_eq!(given(&["kitty", "--listen-on=unix:/run/k"]), "unix:/run/k");
        assert_eq!(given(&["kitty"]), "unix:@kitty-7");
        assert_eq!(given(&["kitty", "--listen-on"]), "unix:@kitty-7");
    }

    #[test]
    fn each_neovim_file_is_in_niris_window_by_its_kittys_title() {
        let niri = [
            json!({ "id": 31, "pid": 10, "title": "notes" }),
            json!({ "id": 32, "pid": 10, "title": "plan" }),
            json!({ "id": 33, "pid": 11, "title": "notes" }),
        ];
        let kitty = json!([
            kitty_window(
                1,
                &[
                    (
                        false,
                        "shell",
                        json!([inside(5, Some("/home/one/back.md"), "/bin/nvim")])
                    ),
                    (
                        true,
                        "notes",
                        json!([inside(
                            6,
                            Some("/home/one/notes.md"),
                            "/nix/store/x/bin/nvim"
                        )])
                    ),
                ]
            ),
            kitty_window(
                2,
                &[(
                    true,
                    "plan",
                    json!([
                        inside(7, Some("/home/one/plan.md"), "nvim"),
                        inside(8, None, "nvim"),
                    ])
                )]
            ),
        ]);
        assert_eq!(
            neovims(&niri, &[(10, kitty)]),
            [
                InKitty {
                    window: 31,
                    path: "/home/one/back.md".into(),
                    go: "10:5".into()
                },
                InKitty {
                    window: 31,
                    path: "/home/one/notes.md".into(),
                    go: "10:6".into()
                },
                InKitty {
                    window: 32,
                    path: "/home/one/plan.md".into(),
                    go: "10:7".into()
                },
            ]
        );
    }

    #[test]
    fn a_file_said_where_no_neovim_runs_or_by_a_shared_title_is_passed_over() {
        let niri = [
            json!({ "id": 31, "pid": 10, "title": "twin" }),
            json!({ "id": 32, "pid": 10, "title": "twin" }),
            json!({ "id": 33, "pid": 10, "title": "shell" }),
        ];
        let kitty = json!([
            kitty_window(
                1,
                &[(
                    true,
                    "twin",
                    json!([inside(5, Some("/home/one/a.md"), "nvim")])
                )]
            ),
            kitty_window(
                2,
                &[(
                    true,
                    "shell",
                    json!([
                        inside(6, Some("/home/one/b.md"), "/bin/zsh"),
                        inside(7, Some("relative.md"), "nvim"),
                    ])
                )]
            ),
        ]);
        assert_eq!(neovims(&niri, &[(10, kitty)]), []);
    }

    #[test]
    fn a_zathura_is_known_by_its_process() {
        assert_eq!(pid_of("org.pwmt.zathura.PID-4242", ZATHURA), Some(4242));
    }

    #[test]
    fn another_name_is_no_zathura() {
        assert_eq!(pid_of("org.freedesktop.Notifications", ZATHURA), None);
        assert_eq!(pid_of("org.pwmt.zathura.PID-", ZATHURA), None);
        assert_eq!(pid_of("org.pwmt.zathura.PID-12x", ZATHURA), None);
    }
}
