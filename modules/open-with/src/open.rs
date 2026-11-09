//! What a program has open now, for its Recent in the launcher to lead with:
//! each of its processes and the whole path of the file it shows. A zathura
//! says it over the session bus, as `org.pwmt.zathura.PID-<pid>`, whose
//! `filename` is the document it has open, empty while it has none, with
//! the page it's on, `pagenumber`, counted from nought, and how many the
//! document has, `numberofpages`, so its row says how far through it is. A
//! Neovim runs in a kitty window, many to one kitty process, so it tells
//! that window where its own server listens instead
//! (../../neovim/showing.lua), kitty's remote control gives that back with
//! niri's window it's in, and the Neovim is asked there what it holds
//! (neovim.rs).

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

/// A document a zathura has open: its process, the document's whole path,
/// the page it's on, counted from one as zathura's own bar counts them, and
/// how many pages it has. Either figure is 0 where zathura didn't say it.
#[derive(Debug, PartialEq, Eq)]
pub struct InZathura {
    pub pid: u32,
    pub file: String,
    pub page: u32,
    pub pages: u32,
}

impl InZathura {
    /// Its line, as the launcher reads a file source's `opened`
    /// (../../quickshell/recents.nix): the process, the path, an empty
    /// third field, since Go To needs nothing more to reach its window,
    /// then the page and how many pages, a tab between each.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{}\t{}\t\t{}\t{}",
            self.pid, self.file, self.page, self.pages
        )
    }
}

/// The page zathura says it's on, counted from nought, as its bar counts
/// it, from one; none where it didn't say.
#[must_use]
pub fn counted_from_one(page: Option<u32>) -> u32 {
    page.map_or(0, |page| page.saturating_add(1))
}

/// Every zathura on the bus with a document open, by its process, the
/// document's path, the page it's on and how many it has. One quitting as
/// it's asked is passed over, so what's left is still said; one that says
/// its document but not its pages, as one still opening it, is said with
/// neither figure.
///
/// # Errors
///
/// When the bus can't list its names.
pub async fn zathura(connection: &Connection) -> zbus::Result<Vec<InZathura>> {
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
            let page = asked.get_property::<u32>("pagenumber").await.ok();
            let pages = asked.get_property::<u32>("numberofpages").await.ok();
            open.push(InZathura {
                pid,
                file,
                page: counted_from_one(page),
                pages: pages.unwrap_or(0),
            });
        }
    }
    Ok(open)
}

/// The kitty user variable a Neovim says where its server listens in.
pub const NEOVIM_SERVER: &str = "neovim_server";

/// How long kitty or niri may take to answer before what it holds is passed
/// over, so a stuck one never holds the launcher.
const ANSWER: Duration = Duration::from_secs(2);

/// A Neovim in a kitty window: niri's window it shows in, where its server
/// listens, what Go To is given to bring it forward inside kitty (`go`),
/// and the processes running Neovim in front there, one of which answers
/// at its server unless what it said is stale.
#[derive(Debug, PartialEq, Eq)]
pub struct InKitty {
    pub window: u64,
    pub server: String,
    pub go: String,
    pub pids: Vec<u64>,
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

/// The process group a process is in, from its `stat` line in /proc, or
/// None once it's gone. Its name is in brackets and may hold anything, so
/// the fields are read after the last bracket: its state, its parent, then
/// its group.
#[must_use]
pub fn group_of(stat: &str) -> Option<u64> {
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(2)?.parse().ok()
}

/// A kitty's `ls` with each process in front in each of its windows given
/// its group (`pgrp`, by `group`), so what Joel ran, the group's leader,
/// is told from what it started, which kitty can't: process ids wrap, so
/// the lowest isn't always the first.
pub fn with_groups(listed: &mut Value, group: impl Fn(u64) -> Option<u64>) {
    let windows = listed
        .as_array_mut()
        .into_iter()
        .flatten()
        .filter_map(|os| os["tabs"].as_array_mut())
        .flatten()
        .filter_map(|tab| tab["windows"].as_array_mut())
        .flatten();
    for window in windows {
        for process in window["foreground_processes"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            if let Some(pgrp) = process["pid"].as_u64().and_then(&group) {
                process["pgrp"] = json!(pgrp);
            }
        }
    }
}

/// Each kitty process niri has a window of, once, lowest first: niri lists
/// a kitty once a window, and one kitty holds many.
#[must_use]
pub fn kitty_pids(niri: &[Value]) -> Vec<u32> {
    let mut pids: Vec<u32> = niri
        .iter()
        .filter(|window| window["app_id"] == "kitty")
        .filter_map(|window| window["pid"].as_u64()?.try_into().ok())
        .collect();
    pids.sort_unstable();
    pids.dedup();
    pids
}

/// Every Neovim in a kitty window that said where its server listens, from
/// niri's windows and each kitty's `ls`, by its process. A kitty window is
/// niri's by its process and the title of its active tab, which kitty
/// titles the window by; a title two of one kitty's windows share names
/// neither. One is counted only while a Neovim runs in front in its
/// window, so what a Neovim that died without clearing it said is passed
/// over.
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
                let Some(server) = inside["user_vars"][NEOVIM_SERVER].as_str() else {
                    continue;
                };
                let pids = neovim_pids(inside);
                if server.starts_with('/') && !pids.is_empty() {
                    open.push(InKitty {
                        window,
                        server: server.to_owned(),
                        go: format!("{pid}:{}", inside["id"]),
                        pids,
                    });
                }
            }
        }
    }
    open
}

/// The Neovims among what runs in front in a kitty window, by process.
fn neovim_pids(window: &Value) -> Vec<u64> {
    window["foreground_processes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|process| {
            process["cmdline"][0].as_str().is_some_and(|program| {
                Path::new(program)
                    .file_name()
                    .is_some_and(|name| name == "nvim")
            })
        })
        .filter_map(|process| process["pid"].as_u64())
        .collect()
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

    /// A kitty window whose program in front is `program`, its process
    /// a hundred past the window's id, its server said where it's given.
    fn inside(id: u64, server: Option<&str>, program: &str) -> Value {
        let vars = server.map_or_else(|| json!({}), |server| json!({ NEOVIM_SERVER: server }));
        json!({
            "id": id,
            "user_vars": vars,
            "foreground_processes": [
                { "pid": id + 100, "cmdline": [program, "--flag"] },
                { "pid": id + 200, "cmdline": ["lua-language-server"] }
            ]
        })
    }

    #[test]
    fn a_process_group_is_read_past_its_name() {
        assert_eq!(group_of("42 (pi) S 1 42 42 0 -1"), Some(42));
        assert_eq!(group_of("43 (a (b) c) R 42 40 40 0"), Some(40));
        assert_eq!(group_of("44 (gone"), None);
        assert_eq!(group_of(""), None);
    }

    #[test]
    fn each_process_in_front_is_given_its_group() {
        let mut listed = json!([{ "tabs": [{ "windows": [{ "foreground_processes": [
            { "pid": 7, "cmdline": ["pi"] },
            { "pid": 3, "cmdline": ["git-engine"] },
            { "pid": 9, "cmdline": ["gone"] }
        ] }] }] }]);
        with_groups(&mut listed, |pid| (pid != 9).then_some(7));
        let processes = &listed[0]["tabs"][0]["windows"][0]["foreground_processes"];
        assert_eq!(processes[0]["pgrp"], 7);
        assert_eq!(processes[1]["pgrp"], 7);
        assert!(processes[2].get("pgrp").is_none());
    }

    #[test]
    fn each_kitty_is_asked_once_however_many_windows_it_has() {
        let niri = [
            json!({ "id": 31, "pid": 11, "app_id": "kitty" }),
            json!({ "id": 32, "pid": 10, "app_id": "kitty" }),
            json!({ "id": 33, "pid": 11, "app_id": "kitty" }),
            json!({ "id": 34, "pid": 12, "app_id": "google-chrome" }),
            json!({ "id": 35, "app_id": "kitty" }),
        ];
        assert_eq!(kitty_pids(&niri), [10, 11]);
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
    fn each_neovim_is_in_niris_window_by_its_kittys_title() {
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
                        json!([inside(5, Some("/run/user/1/nvim.105.0"), "/bin/nvim")])
                    ),
                    (
                        true,
                        "notes",
                        json!([inside(
                            6,
                            Some("/run/user/1/nvim.106.0"),
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
                        inside(7, Some("/run/user/1/nvim.107.0"), "nvim"),
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
                    server: "/run/user/1/nvim.105.0".into(),
                    go: "10:5".into(),
                    pids: vec![105]
                },
                InKitty {
                    window: 31,
                    server: "/run/user/1/nvim.106.0".into(),
                    go: "10:6".into(),
                    pids: vec![106]
                },
                InKitty {
                    window: 32,
                    server: "/run/user/1/nvim.107.0".into(),
                    go: "10:7".into(),
                    pids: vec![107]
                },
            ]
        );
    }

    #[test]
    fn a_server_said_where_no_neovim_runs_or_by_a_shared_title_is_passed_over() {
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
                    json!([inside(5, Some("/run/user/1/nvim.105.0"), "nvim")])
                )]
            ),
            kitty_window(
                2,
                &[(
                    true,
                    "shell",
                    json!([
                        inside(6, Some("/run/user/1/nvim.106.0"), "/bin/zsh"),
                        inside(7, Some("nvim.107.0"), "nvim"),
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
    fn a_zathura_says_its_page_from_one_as_its_bar_does() {
        assert_eq!(counted_from_one(Some(0)), 1);
        assert_eq!(counted_from_one(Some(40)), 41);
        assert_eq!(counted_from_one(None), 0);
        assert_eq!(counted_from_one(Some(u32::MAX)), u32::MAX);
    }

    #[test]
    fn a_zathuras_line_leaves_go_empty_before_its_pages() {
        let open = InZathura {
            pid: 4242,
            file: "/home/one/charts.pdf".into(),
            page: 41,
            pages: 212,
        };
        let line = open.line();
        assert_eq!(line, "4242\t/home/one/charts.pdf\t\t41\t212");
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields, ["4242", "/home/one/charts.pdf", "", "41", "212"]);
    }

    #[test]
    fn another_name_is_no_zathura() {
        assert_eq!(pid_of("org.freedesktop.Notifications", ZATHURA), None);
        assert_eq!(pid_of("org.pwmt.zathura.PID-", ZATHURA), None);
        assert_eq!(pid_of("org.pwmt.zathura.PID-12x", ZATHURA), None);
    }
}
