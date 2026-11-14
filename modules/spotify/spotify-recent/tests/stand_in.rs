//! The program against a stand-in for Spotify on this machine: that it
//! refreshes a sign-in that's run out and writes it back, Joel's alone,
//! before asking, and leaves one another refreshed while it waited; asks
//! with the new one, says what played lately a line a context, walks a
//! list page after page, plays where Spotify plays or on the first device
//! open, and says by its exit status when it's signed out, has nowhere to
//! play, can't reach Spotify or finds it busy.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// One request the stand-in heard: its first line, its headers and its
/// body.
#[derive(Clone, Debug)]
struct Heard {
    line: String,
    headers: Vec<String>,
    body: String,
}

impl Heard {
    /// The access token it was asked with. A header's name is in any
    /// case, as HTTP has it.
    fn bearer(&self) -> Option<&str> {
        self.headers.iter().find_map(|header| {
            let (name, value) = header.split_once(": ")?;
            name.eq_ignore_ascii_case("authorization")
                .then_some(value)?
                .strip_prefix("Bearer ")
        })
    }
}

/// A stand-in for Spotify's API and accounts server at once, answering
/// each request's first line with a status and a body from `answer`.
struct StandIn {
    origin: String,
    heard: Arc<Mutex<Vec<Heard>>>,
}

impl StandIn {
    fn new(answer: fn(&str) -> (u16, String)) -> StandIn {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        let own = origin.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let mut headers = Vec::new();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    let header = header.trim_end().to_owned();
                    if header.is_empty() {
                        break;
                    }
                    if let Some(size) = header.to_lowercase().strip_prefix("content-length: ") {
                        length = size.parse().unwrap_or(0);
                    }
                    headers.push(header);
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let line = line.trim_end().to_owned();
                let (status, said) = answer(&line);
                // A page whose next is Spotify's own address leads back
                // here, so no test ever asks the real Spotify.
                let said = said.replace("https://api.spotify.com", &own);
                kept.lock().unwrap().push(Heard {
                    line,
                    headers,
                    body: String::from_utf8_lossy(&body).into_owned(),
                });
                let reply = format!(
                    "HTTP/1.1 {status} Said\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{said}",
                    said.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        StandIn { origin, heard }
    }

    fn heard(&self) -> Vec<Heard> {
        self.heard.lock().unwrap().clone()
    }

    fn lines(&self) -> Vec<String> {
        self.heard().into_iter().map(|heard| heard.line).collect()
    }
}

/// A sign-in in a folder of the test's own, run out long ago.
fn signed_in(test: &str) -> PathBuf {
    let named = format!("spotify-recent-{test}-{}", std::process::id());
    let folder = std::env::temp_dir().join(named);
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("token.json");
    std::fs::write(&path, include_str!("../fixtures/token.json")).unwrap();
    path
}

fn command(stand_in: &StandIn, token: &Path, asked: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_spotify-recent"));
    command
        .args(asked)
        .env("SPOTIFY_RECENT_API", format!("{}/v1", stand_in.origin))
        .env("SPOTIFY_RECENT_ACCOUNTS", &stand_in.origin)
        .env("SPOTIFY_RECENT_TOKEN", token);
    command
}

fn run(stand_in: &StandIn, token: &Path, asked: &[&str]) -> Output {
    command(stand_in, token, asked).output().unwrap()
}

/// Waits until process `pid` is held waiting on the lock `lock`, as
/// Linux lists a waiter in /proc/locks, by the file's inode, beside an
/// arrow.
fn waits_on(pid: u32, lock: &Path) {
    let inode = format!(":{} ", std::fs::metadata(lock).unwrap().ino());
    let given_up = Instant::now() + Duration::from_secs(20);
    loop {
        let locks = std::fs::read_to_string("/proc/locks").unwrap();
        let waiting = locks.lines().any(|line| {
            line.contains("->")
                && line.contains(&inode)
                && line.split_whitespace().any(|word| word == pid.to_string())
        });
        if waiting {
            return;
        }
        assert!(Instant::now() < given_up, "it never waited on the lock");
        thread::sleep(Duration::from_millis(10));
    }
}

/// Spotify as it answers a signed-in account playing a playlist.
fn spotify(line: &str) -> (u16, String) {
    let path = line.split(' ').nth(1).unwrap_or_default();
    let said = match path {
        "/api/token" => {
            r#"{"access_token":"fresh-access","token_type":"Bearer","expires_in":3600}"#
        }
        "/v1/me/player?additional_types=episode" => include_str!("../fixtures/player.json"),
        "/v1/me/player/recently-played?limit=50" => {
            include_str!("../fixtures/recently-played.json")
        }
        "/v1/playlists/p9?fields=name,owner(display_name),items(total)" => {
            r#"{"name":"Morning","owner":{"display_name":"Joel"},"items":{"total":12}}"#
        }
        "/v1/playlists/p1?fields=name,owner(display_name),items(total)" => {
            r#"{"name":"Focus","owner":{"display_name":"Joel"},"items":{"total":46}}"#
        }
        "/v1/playlists/p3?fields=name,owner(display_name),items(total)" => {
            r#"{"name":"Road","owner":{"display_name":"Spotify"},"items":{"total":50}}"#
        }
        // The artists Joel follows, a page at a time by a cursor, the next
        // given as Spotify gives it, whole.
        "/v1/me/following?type=artist&limit=50" => include_str!("../fixtures/artists.json"),
        "/v1/me/following?type=artist&after=r1&limit=50" => {
            r#"{"artists":{"items":[{"uri":"spotify:artist:r2","name":"Other"}],"next":null}}"#
        }
        "/v1/me/tracks?limit=1" => r#"{"items":[],"total":1234}"#,
        "/v1/me" => r#"{"id":"joel"}"#,
        "/v1/me/player/play" => return (404, r#"{"error":{"status":404}}"#.to_owned()),
        "/v1/me/player/devices" => {
            r#"{"devices":[{"id":"shut","is_restricted":true},{"id":"laptop"}]}"#
        }
        "/v1/me/player/play?device_id=laptop" => return (204, String::new()),
        _ => {
            return (
                404,
                r#"{"error":{"status":404,"message":"no such thing"}}"#.to_owned(),
            );
        }
    };
    (200, said.to_owned())
}

#[test]
fn a_spent_sign_in_is_refreshed_and_kept_before_anything_is_asked() {
    let stand_in = StandIn::new(spotify);
    let token = signed_in("refresh");
    let out = run(&stand_in, &token, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let heard = stand_in.heard();
    assert_eq!(heard[0].line, "POST /api/token HTTP/1.1");
    let form = &heard[0].body;
    assert!(form.contains("grant_type=refresh_token"), "{form}");
    assert!(form.contains("refresh_token=old-refresh"), "{form}");
    assert!(
        form.contains("client_id=d420a117a32841c2b3474932e49fb54b"),
        "{form}"
    );
    assert!(
        heard[1..]
            .iter()
            .all(|asked| asked.bearer() == Some("fresh-access"))
    );
    let kept: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&token).unwrap()).unwrap();
    assert_eq!(kept["access_token"], "fresh-access");
    assert_eq!(kept["refresh_token"], "old-refresh");
    let mode = std::fs::metadata(&token).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    let said = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(
        lines[0],
        "spotify:playlist:p9\tplaylist\tMorning\tJoel\tplaying\t12"
    );
    assert_eq!(
        lines[1],
        "spotify:playlist:p1\tplaylist\tFocus\tJoel\t2026-11-07T08:50:00.000Z\t46"
    );
    assert_eq!(lines.len(), 9);
    // The second run finds the sign-in fresh, and refreshes nothing.
    let again = StandIn::new(spotify);
    assert!(run(&again, &token, &["liked"]).status.success());
    assert_eq!(again.lines(), ["GET /v1/me/tracks?limit=1 HTTP/1.1"]);
}

#[test]
fn a_sign_in_refreshed_while_it_waited_is_taken_as_it_is() {
    let stand_in = StandIn::new(spotify);
    let token = signed_in("waited");
    // Another refresh holds the lock, as the launcher's other reads may.
    let lock_path = token.with_file_name("token.json.lock");
    let lock = File::create(&lock_path).unwrap();
    lock.lock().unwrap();
    let child = command(&stand_in, &token, &["liked"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    // Once it has read the spent sign-in and waits to refresh it, the
    // other refresh writes its own and lets go.
    waits_on(child.id(), &lock_path);
    std::fs::write(
        &token,
        concat!(
            r#"{"access_token":"theirs","expires_in":3600,"#,
            r#""expires_at":"2099-01-01T00:00:00Z","refresh_token":"their-refresh","scope":""}"#,
        ),
    )
    .unwrap();
    lock.unlock().unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    // Refreshing again would spend the refresh token the other was given.
    assert_eq!(stand_in.lines(), ["GET /v1/me/tracks?limit=1 HTTP/1.1"]);
    assert_eq!(stand_in.heard()[0].bearer(), Some("theirs"));
}

#[test]
fn a_list_is_walked_page_after_page() {
    let stand_in = StandIn::new(spotify);
    let out = run(&stand_in, &signed_in("artists"), &["artists"]);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "spotify:artist:r1\tartist\tArtist\t\t\t\nspotify:artist:r2\tartist\tOther\t\t\t\n"
    );
}

#[test]
fn liked_songs_are_counted() {
    let stand_in = StandIn::new(spotify);
    let out = run(&stand_in, &signed_in("liked"), &["liked"]);
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "1234\n");
}

#[test]
fn with_nothing_playing_it_plays_on_the_first_device_that_takes_it() {
    let stand_in = StandIn::new(spotify);
    let out = run(
        &stand_in,
        &signed_in("play"),
        &["play", "spotify:collection:tracks"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let heard = stand_in.heard();
    let lines: Vec<&str> = heard.iter().map(|one| one.line.as_str()).collect();
    assert_eq!(
        lines,
        [
            "POST /api/token HTTP/1.1",
            "GET /v1/me HTTP/1.1",
            "PUT /v1/me/player/play HTTP/1.1",
            "GET /v1/me/player/devices HTTP/1.1",
            "PUT /v1/me/player/play?device_id=laptop HTTP/1.1"
        ]
    );
    let body: serde_json::Value = serde_json::from_str(&heard[4].body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({ "context_uri": "spotify:user:joel:collection" })
    );
}

#[test]
fn with_no_device_open_it_says_so() {
    let stand_in = StandIn::new(|line| match line.split(' ').nth(1) {
        Some("/v1/me/player/devices") => (200, r#"{"devices":[]}"#.to_owned()),
        _ => spotify(line),
    });
    let out = run(
        &stand_in,
        &signed_in("nowhere"),
        &["play", "spotify:album:a1"],
    );
    assert_eq!(
        out.status.code(),
        Some(66),
        "nowhere to play isn't 66: {out:?}"
    );
}

#[test]
fn a_refresh_spotify_refuses_is_signed_out_and_keeps_the_file() {
    let stand_in = StandIn::new(|_| (400, r#"{"error":"invalid_grant"}"#.to_owned()));
    let token = signed_in("refused");
    let out = run(&stand_in, &token, &[]);
    assert_eq!(
        out.status.code(),
        Some(78),
        "a refused refresh isn't 78: {out:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&token).unwrap(),
        include_str!("../fixtures/token.json")
    );
    assert!(!String::from_utf8_lossy(&out.stderr).contains("old-refresh"));
}

#[test]
fn no_sign_in_is_signed_out_without_asking() {
    let stand_in = StandIn::new(spotify);
    let missing = std::env::temp_dir().join("spotify-recent-nobody/token.json");
    let out = run(&stand_in, &missing, &[]);
    assert_eq!(out.status.code(), Some(78), "no sign-in isn't 78: {out:?}");
    assert!(stand_in.heard().is_empty());
}

#[test]
fn spotify_out_of_reach_says_so() {
    // A port nothing listens on, taken and let go.
    let origin = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let out = Command::new(env!("CARGO_BIN_EXE_spotify-recent"))
        .env("SPOTIFY_RECENT_API", &origin)
        .env("SPOTIFY_RECENT_ACCOUNTS", &origin)
        .env("SPOTIFY_RECENT_TOKEN", signed_in("unreachable"))
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(69),
        "out of reach isn't 69: {out:?}"
    );
}

#[test]
fn spotify_busy_says_to_try_again() {
    let stand_in = StandIn::new(|line| match line.split(' ').nth(1) {
        Some(path) if path.starts_with("/v1/") => (429, String::new()),
        _ => spotify(line),
    });
    let out = run(&stand_in, &signed_in("busy"), &[]);
    assert_eq!(out.status.code(), Some(75), "busy isn't 75: {out:?}");
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("Spotify is busy."), "busy isn't said: {said}");
}
