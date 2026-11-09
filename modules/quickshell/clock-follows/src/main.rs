//! clock-follows moves the clock's time zone to where the laptop is (Joel,
//! 2026-10-28: "i really want my timezone to automatically change if
//! possible"), and puts it back when he asks.
//!
//! It watches where the laptop is, `here:laptop`, over whereabouts' socket
//! in the shell's daemon interface, and decides each time it's told, so
//! nothing here asks on a clock. Should whereabouts stop, the socket
//! systemd holds takes the next connection, asked again two seconds on,
//! and the watch's first answer is decided as any change is. Only a fix
//! that differs from the last one decided is: the same record also says
//! the clock's zone, and a zone Joel sets by hand, or puts back, mustn't
//! be taken as the laptop moving and undone a moment later. A fix
//! moves the clock only when it names a zone and GeoClue found it to within
//! ten kilometres, as the Wi-Fi networks in reach find it. A position read
//! from the network's address alone, which WARP's exit sends anywhere, is
//! one GeoClue is far less sure of, so it never moves the clock.
//!
//! Each move is written down (`moved.json` in the state folder) for the
//! launcher's Now to say, and `put-back` sets the clock back to the zone it
//! moved from. A zone put back is held (`held.json`) until the laptop is
//! found in some other zone, so the clock isn't moved again while Joel is
//! still where he said no.
//!
//!     clock-follows follow <whereabouts' socket> <state folder> <timedatectl>
//!     clock-follows put-back <state folder> <timedatectl>

use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The key whereabouts says where the laptop is by.
const HERE: &str = "here:laptop";

/// How long a socket that refused, or a whereabouts that stopped, waits
/// before it's asked again.
const AGAIN: Duration = Duration::from_secs(2);

/// How near, in metres, GeoClue has to say the laptop is before its zone
/// moves the clock. Wi-Fi finds a laptop to within a few hundred metres;
/// an address alone, to a city's or a country's width.
const TRUSTED: f64 = 10_000.0;

/// What of whereabouts' record the clock needs, each null while unknown,
/// and when it was found, which tells a fix found again from the same.
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Fix {
    #[serde(default)]
    at_s: Option<u64>,
    #[serde(default)]
    accuracy_m: Option<f64>,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    zone: Option<String>,
}

/// The move the launcher says: from which zone to which, and where the
/// laptop was found, so Now can say "Clock moved to Edmonton time, from
/// Kelpwater".
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
struct Moved {
    from: String,
    to: String,
    place: Option<String>,
    when: u64,
}

/// The zone Joel put back from, held until he's somewhere else.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
struct Held {
    refused: Option<String>,
}

/// What a fix asks of the clock.
#[derive(Debug, PartialEq)]
enum Decision {
    /// Nothing changes.
    Stay,
    /// The laptop has left the zone put back from, and is in the clock's.
    Unhold,
    /// The clock moves to this zone, letting go of any zone held.
    Move(String),
}

/// What a fix asks, given the clock's zone and the one held, if any.
fn decide(fix: &Fix, clock: &str, held: Option<&str>) -> Decision {
    let trusted = fix
        .accuracy_m
        .is_some_and(|metres| metres.is_finite() && metres <= TRUSTED);
    let Some(zone) = fix
        .zone
        .as_deref()
        .filter(|zone| trusted && zone.contains('/'))
    else {
        return Decision::Stay;
    };
    if zone == clock {
        return if held.is_some_and(|refused| refused != zone) {
            Decision::Unhold
        } else {
            Decision::Stay
        };
    }
    if held == Some(zone) {
        return Decision::Stay;
    }
    Decision::Move(zone.to_owned())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The clock's zone, as timedated says it.
fn clock(timedatectl: &str) -> io::Result<String> {
    let said = Command::new(timedatectl)
        .args(["show", "--property=Timezone", "--value"])
        .output()?;
    if !said.status.success() {
        return Err(io::Error::other(
            "timedatectl couldn't say the clock's zone",
        ));
    }
    Ok(String::from_utf8_lossy(&said.stdout).trim().to_owned())
}

/// Sets the clock's zone, which polkit lets the machine's administrators
/// do without a password (nixos.nix's rule).
fn set(timedatectl: &str, zone: &str) -> io::Result<()> {
    let status = Command::new(timedatectl)
        .args(["set-timezone", zone])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "timedated refused the zone {zone}"
        )))
    }
}

fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// Writes a file whole, under another name first, so the launcher never
/// reads half of one.
fn write<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let fresh = path.with_extension("new");
    fs::write(&fresh, serde_json::to_vec(value).map_err(io::Error::other)?)?;
    fs::rename(fresh, path)
}

/// Watches where the laptop is for as long as the program runs, deciding
/// each time it's told, and connecting again whenever whereabouts stops.
fn follow(socket: &Path, state: &Path, timedatectl: &str) -> io::Result<()> {
    loop {
        if let Err(why) = watched(socket, state, timedatectl) {
            eprintln!("<4>where the laptop is couldn't be watched: {why}");
        }
        std::thread::sleep(AGAIN);
    }
}

/// One connection's watch, until whereabouts hangs up.
fn watched(socket: &Path, state: &Path, timedatectl: &str) -> io::Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(
        stream,
        r#"{{"id":"here","ask":"watch","kind":"here","keys":["{HERE}"]}}"#
    )?;
    // A new connection's first answer is decided, as a start is.
    let mut last: Option<Fix> = None;
    for line in BufReader::new(stream).lines() {
        for fix in fixes(&line?) {
            if last.as_ref() == Some(&fix) {
                continue;
            }
            last = Some(fix.clone());
            // A move that fails, as when timedated refuses, is said and
            // the next change decided anew.
            match decided(&fix, state, timedatectl) {
                // Said each time, never where, so what was heard can be
                // counted without the journal keeping where Joel is.
                Ok(()) => eprintln!("<7>where the laptop is was decided"),
                Err(why) => eprintln!("<3>{why}"),
            }
        }
    }
    Ok(())
}

/// The laptop's fixes one line of whereabouts' says: its watch's first
/// answer, or a change.
fn fixes(line: &str) -> Vec<Fix> {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    said.get("records")
        .or_else(|| said.get("changed"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|record| record.get("key").and_then(Value::as_str) == Some(HERE))
        .filter_map(|record| serde_json::from_value(record.clone()).ok())
        .collect()
}

fn decided(fix: &Fix, state: &Path, timedatectl: &str) -> io::Result<()> {
    let held_path = state.join("held.json");
    let held = read::<Held>(&held_path).unwrap_or_default();
    let from = clock(timedatectl)?;
    match decide(fix, &from, held.refused.as_deref()) {
        Decision::Stay => Ok(()),
        Decision::Unhold => write(&held_path, &Held::default()),
        Decision::Move(to) => {
            set(timedatectl, &to)?;
            write(&held_path, &Held::default())?;
            eprintln!("<6>the clock moved to another zone");
            write(
                &state.join("moved.json"),
                &Moved {
                    from,
                    to,
                    place: fix.place.clone(),
                    when: now(),
                },
            )
        }
    }
}

fn put_back(state: &Path, timedatectl: &str) -> io::Result<()> {
    let moved_path = state.join("moved.json");
    let Some(moved) = read::<Moved>(&moved_path) else {
        return Ok(());
    };
    // Held before the clock moves, so a fix decided meanwhile is already
    // told no, and let go again where timedated refuses the zone.
    let held_path = state.join("held.json");
    let was = read::<Held>(&held_path).unwrap_or_default();
    write(
        &held_path,
        &Held {
            refused: Some(moved.to),
        },
    )?;
    if let Err(why) = set(timedatectl, &moved.from) {
        write(&held_path, &was)?;
        return Err(why);
    }
    eprintln!("<6>the clock was put back");
    fs::remove_file(moved_path)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let done = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["follow", socket, state, timedatectl] => fs::create_dir_all(state)
            .and_then(|()| follow(Path::new(socket), Path::new(state), timedatectl)),
        ["put-back", state, timedatectl] => put_back(Path::new(state), timedatectl),
        _ => {
            eprintln!("usage: clock-follows follow <whereabouts' socket> <state> <timedatectl>");
            eprintln!("       clock-follows put-back <state> <timedatectl>");
            return ExitCode::from(64);
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(accuracy: f64, zone: Option<&str>) -> Fix {
        Fix {
            at_s: Some(1_790_000_000),
            accuracy_m: Some(accuracy),
            place: Some("Kelpwater, Highreach".into()),
            zone: zone.map(Into::into),
        }
    }

    #[test]
    fn the_laptops_record_is_read_from_an_answer_or_a_change() {
        let answer = r#"{"id":"here","records":[{"key":"here:laptop","kind":"here",
            "accuracy_m":300.0,"zone":"America/Edmonton","place":null}]}"#;
        let said = fixes(answer);
        assert_eq!(said.len(), 1);
        assert_eq!(said[0].zone.as_deref(), Some("America/Edmonton"));
        let change = r#"{"id":"here","changed":[{"key":"here:laptop","accuracy_m":null}]}"#;
        assert_eq!(
            decide(&fixes(change)[0], "America/Toronto", None),
            Decision::Stay,
            "nowhere moves nothing"
        );
        assert_eq!(
            fixes(r#"{"id":"here","unwatched":true}"#),
            Vec::<Fix>::new()
        );
        assert_eq!(fixes("not a line"), Vec::<Fix>::new());
    }

    #[test]
    fn a_trusted_fix_in_another_zone_moves_the_clock() {
        let moved = decide(
            &fix(300.0, Some("America/Edmonton")),
            "America/Toronto",
            None,
        );
        assert_eq!(moved, Decision::Move("America/Edmonton".into()));
    }

    #[test]
    fn a_fix_in_the_clocks_zone_changes_nothing() {
        assert_eq!(
            decide(
                &fix(300.0, Some("America/Toronto")),
                "America/Toronto",
                None
            ),
            Decision::Stay
        );
    }

    #[test]
    fn a_fix_found_from_an_address_alone_never_moves_the_clock() {
        // BeaconDB's answer from the network's address, as at a hotel or
        // through WARP, is tens of kilometres wide.
        assert_eq!(
            decide(&fix(26_000.0, Some("Asia/Tokyo")), "America/Toronto", None),
            Decision::Stay
        );
        assert_eq!(
            decide(&fix(f64::NAN, Some("Asia/Tokyo")), "America/Toronto", None),
            Decision::Stay
        );
    }

    #[test]
    fn a_fix_without_a_zone_or_an_accuracy_changes_nothing() {
        assert_eq!(
            decide(&fix(300.0, None), "America/Toronto", None),
            Decision::Stay
        );
        assert_eq!(
            decide(&fix(300.0, Some("UTC")), "America/Toronto", None),
            Decision::Stay
        );
        let unsure = Fix {
            accuracy_m: None,
            ..fix(300.0, Some("America/Edmonton"))
        };
        assert_eq!(decide(&unsure, "America/Toronto", None), Decision::Stay);
    }

    #[test]
    fn a_zone_put_back_from_holds_while_the_laptop_is_there() {
        let held = Some("America/Edmonton");
        assert_eq!(
            decide(
                &fix(300.0, Some("America/Edmonton")),
                "America/Toronto",
                held
            ),
            Decision::Stay
        );
    }

    #[test]
    fn somewhere_else_lets_the_zone_held_go() {
        let held = Some("America/Edmonton");
        let moved = decide(
            &fix(300.0, Some("America/Vancouver")),
            "America/Toronto",
            held,
        );
        assert_eq!(moved, Decision::Move("America/Vancouver".into()));
        assert_eq!(
            decide(
                &fix(300.0, Some("America/Toronto")),
                "America/Toronto",
                held
            ),
            Decision::Unhold
        );
    }

    #[test]
    fn a_zone_put_back_is_held_only_once_timedated_takes_it() {
        let state =
            std::env::temp_dir().join(format!("clock-follows-put-back-{}", std::process::id()));
        let _ = fs::remove_dir_all(&state);
        fs::create_dir_all(&state).unwrap();
        let moved = Moved {
            from: "America/Toronto".into(),
            to: "America/Edmonton".into(),
            place: None,
            when: 1_790_000_000,
        };
        write(&state.join("moved.json"), &moved).unwrap();
        assert!(put_back(&state, "false").is_err());
        assert_eq!(
            read::<Held>(&state.join("held.json")),
            Some(Held::default())
        );
        assert_eq!(read::<Moved>(&state.join("moved.json")), Some(moved));
        put_back(&state, "true").unwrap();
        assert_eq!(
            read::<Held>(&state.join("held.json")),
            Some(Held {
                refused: Some("America/Edmonton".into())
            })
        );
        assert!(!state.join("moved.json").exists());
        fs::remove_dir_all(&state).unwrap();
    }
}
