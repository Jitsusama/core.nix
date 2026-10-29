//! clock-follows moves the clock's time zone to where the laptop is (Joel,
//! 2026-10-28: "i really want my timezone to automatically change if
//! possible"), and puts it back when he asks.
//!
//! It runs once each time whereabouts writes where the laptop is (a path
//! unit watches the file), so nothing here waits or asks on a clock. A fix
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
//!     clock-follows follow <whereabouts.json> <state folder> <timedatectl>
//!     clock-follows put-back <state folder> <timedatectl>

use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// How near, in metres, GeoClue has to say the laptop is before its zone
/// moves the clock. Wi-Fi finds a laptop to within a few hundred metres;
/// an address alone, to a city's or a country's width.
const TRUSTED: f64 = 10_000.0;

/// The version of whereabouts' file that names a zone.
const NAMED: u32 = 2;

/// What of whereabouts' file the clock needs.
#[derive(Debug, Deserialize)]
struct Fix {
    v: u32,
    accuracy: f64,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    zone: Option<String>,
}

/// The move the launcher says: from which zone to which, and where the
/// laptop was found, so Now can say "Clock moved to Edmonton time, from
/// Calgary".
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
    let trusted = fix.v == NAMED && fix.accuracy.is_finite() && fix.accuracy <= TRUSTED;
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

fn follow(fix: &Path, state: &Path, timedatectl: &str) -> io::Result<()> {
    let Some(fix) = read::<Fix>(fix) else {
        return Ok(());
    };
    let held_path = state.join("held.json");
    let held = read::<Held>(&held_path).unwrap_or_default();
    let from = clock(timedatectl)?;
    match decide(&fix, &from, held.refused.as_deref()) {
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
                    place: fix.place,
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
    set(timedatectl, &moved.from)?;
    write(
        &state.join("held.json"),
        &Held {
            refused: Some(moved.to),
        },
    )?;
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
        ["follow", fix, state, timedatectl] => fs::create_dir_all(state)
            .and_then(|()| follow(Path::new(fix), Path::new(state), timedatectl)),
        ["put-back", state, timedatectl] => put_back(Path::new(state), timedatectl),
        _ => {
            eprintln!("usage: clock-follows follow <whereabouts.json> <state> <timedatectl>");
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
            v: NAMED,
            accuracy,
            place: Some("Lighthouse, Alberta".into()),
            zone: zone.map(Into::into),
        }
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
    fn a_fix_without_a_zone_or_of_another_version_changes_nothing() {
        assert_eq!(
            decide(&fix(300.0, None), "America/Toronto", None),
            Decision::Stay
        );
        assert_eq!(
            decide(&fix(300.0, Some("UTC")), "America/Toronto", None),
            Decision::Stay
        );
        let older = Fix {
            v: 1,
            ..fix(300.0, Some("America/Edmonton"))
        };
        assert_eq!(decide(&older, "America/Toronto", None), Decision::Stay);
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
}
