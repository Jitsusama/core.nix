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
//! be taken as the laptop moving and undone a moment later. The last fix
//! decided is kept in the state folder (`decided.json`), so a restart, a
//! switch or a login doesn't decide it again over his zone either.
//!
//! A fix moves the clock only when it names a zone and GeoClue found it
//! to a region's width, fifty kilometres, as a time zone needs no nearer.
//! Where `BeaconDB` knows none of the Wi-Fi in reach, it places the laptop
//! from the network's address to about 25 km, and that's the fix the
//! laptops get at home, which a rule of ten never once trusted. A circle
//! that wide can cross a zone's line, so past ten kilometres it moves the
//! clock only where whereabouts found every point on land round the circle
//! keeping the zone's time (`zone_holds`), and within ten, unless it found
//! one that doesn't. WARP's exit stands in for the network's address, so
//! such a fix may be of the exit's region rather than the laptop's; should
//! it name another zone, Put It Back holds Joel's until he moves. A
//! fix wider than a region, a country's width, never moves the clock.
//!
//! Each move is written down (`moved.json` in the state folder) for the
//! launcher's Now to say, and `put-back` sets the clock back to the zone it
//! moved from. A zone put back is held (`held.json`) until the laptop is
//! found in some other zone, so the clock isn't moved again while Joel is
//! still where he said no.
//!
//!     clock-follows follow <whereabouts' socket> <state folder> <timedatectl>
//!     clock-follows put-back <state folder> <timedatectl>
//!
//! Run bare, or as `show`, it says what its state folder holds: the
//! clock's last move and whether it follows, in Joel's words or, with
//! `--json`, as JSON (docs/commands.md; show.rs says it). It takes that shape through the terminal kit,
//! which marks a failure's lines as errors for the journal.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use terminal_kit::{Exit, Failure, Help};

mod show;

use show::{said, shown};

/// The key whereabouts says where the laptop is by.
const HERE: &str = "here:laptop";

/// How long a socket that refused, or a whereabouts that stopped, waits
/// before it's asked again.
const AGAIN: Duration = Duration::from_secs(2);

/// How near, in metres, GeoClue has to say the laptop is before its zone
/// moves the clock: its own region's width, `GCLUE_LOCATION_ACCURACY_REGION`.
/// Wi-Fi finds a laptop to within a few hundred metres, or `BeaconDB`'s
/// coarser answer to some tens of kilometres; an address alone, to a
/// country's width.
const TRUSTED: f64 = 50_000.0;

/// How near a fix has to be for its zone to move the clock while nobody
/// could say whether it holds round the circle, as for a place Joel named
/// or an older whereabouts: ten kilometres, the rule this was before.
const NEAR: f64 = 10_000.0;

/// What of whereabouts' record the clock needs, each null while unknown,
/// and when it was found, which tells a fix found again from the same.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
struct Fix {
    #[serde(default)]
    at_s: Option<u64>,
    #[serde(default)]
    accuracy_m: Option<f64>,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    zone: Option<String>,
    #[serde(default)]
    zone_holds: Option<bool>,
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
    let trusted = fix.accuracy_m.is_some_and(|metres| {
        metres.is_finite()
            && match fix.zone_holds {
                Some(true) => metres <= TRUSTED,
                Some(false) => false,
                None => metres <= NEAR,
            }
    });
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

/// The last fix decided, kept in the state folder across connections and
/// starts.
struct Decided {
    path: PathBuf,
    last: Option<Fix>,
}

impl Decided {
    fn in_state(state: &Path) -> Decided {
        let path = state.join("decided.json");
        let last = read(&path);
        Decided { path, last }
    }

    /// Whether `fix` differs from the last decided, so is one to decide.
    fn differs(&self, fix: &Fix) -> bool {
        self.last.as_ref() != Some(fix)
    }

    /// Keeps `fix` as the last decided, once deciding it went through, so
    /// one timedated refused is decided again on the next connection.
    fn keep(&mut self, fix: Fix) {
        if let Err(why) = write(&self.path, &fix) {
            eprintln!("<4>the fix decided couldn't be kept: {why}");
        }
        self.last = Some(fix);
    }
}

/// Watches where the laptop is for as long as the program runs, deciding
/// each time it's told, and connecting again whenever whereabouts stops.
/// Why it couldn't is said once, and again only once it's something else,
/// so a socket missing for an hour says so once rather than each try.
fn follow(socket: &Path, state: &Path, timedatectl: &str) -> io::Result<()> {
    let mut kept = Decided::in_state(state);
    let mut said: Option<String> = None;
    loop {
        let why = watched(socket, state, timedatectl, &mut kept)
            .err()
            .map(|why| why.to_string());
        if why.is_some() && why != said {
            eprintln!(
                "<4>where the laptop is couldn't be watched: {}",
                why.as_deref().unwrap_or_default()
            );
        }
        said = why;
        std::thread::sleep(AGAIN);
    }
}

/// One connection's watch, until whereabouts hangs up.
fn watched(socket: &Path, state: &Path, timedatectl: &str, kept: &mut Decided) -> io::Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(
        stream,
        r#"{{"id":"here","ask":"watch","kind":"here","keys":["{HERE}"]}}"#
    )?;
    for line in BufReader::new(stream).lines() {
        for fix in fixes(&line?) {
            if !kept.differs(&fix) {
                continue;
            }
            // A move that fails, as when timedated refuses, is said and
            // isn't kept, so the same fix is decided anew as it comes again.
            match decided(&fix, state, timedatectl) {
                // Said each time, never where, so what was heard can be
                // counted without the journal keeping where Joel is.
                Ok(()) => {
                    eprintln!("<7>where the laptop is was decided");
                    kept.keep(fix);
                }
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

/// Where the account's unit keeps its state, `%S/clock-follows`, for a
/// run by hand that names none.
fn kept_state() -> Option<PathBuf> {
    let home = || std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"));
    let states = std::env::var_os("XDG_STATE_HOME")
        .filter(|states| !states.is_empty())
        .map(PathBuf::from)
        .or_else(home)?;
    Some(states.join("clock-follows"))
}

fn help() -> Help {
    Help::new(
        "clock-follows",
        "moves the clock's zone to where the laptop is",
    )
    .subcommand(
        Help::new("show", "where the clock last moved, and if it follows")
            .example("clock-follows show", "the same as bare clock-follows")
            .argument("[state]", "the state folder, if not the account's"),
    )
    .subcommand(
        Help::new("follow", "watches whereabouts and sets the zone")
            .example(
                "clock-follows follow SOCKET STATE PROGRAM",
                "as its service runs it",
            )
            .argument("<socket>", "where whereabouts are told")
            .argument("<state>", "the folder its moves are kept in")
            .argument("<program>", "what sets the zone")
            .exit(Exit::Failed, "the zone couldn't be set, or its state kept"),
    )
    .subcommand(
        Help::new("put-back", "puts the zone back where it was")
            .example(
                "clock-follows put-back STATE PROGRAM",
                "the zone before it moved",
            )
            .argument("<state>", "the folder its moves are kept in")
            .argument("<program>", "what sets the zone")
            .exit(Exit::Failed, "the zone couldn't be set"),
    )
    .example(
        "clock-follows",
        "where the clock last moved, and if it follows you",
    )
    .example("clock-follows --json", "the same, as JSON")
    .exit(Exit::Failed, "the zone couldn't be set, or its state kept")
    .exit(
        Exit::Config,
        "no state folder named, and no XDG_STATE_HOME or HOME",
    )
}

fn asked_wrongly(what: &str) -> Failure {
    Failure::new(Exit::Usage, what).try_this("clock-follows --help")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let args: Vec<&str> = context.args.iter().map(String::as_str).collect();
        let failed = |what: &'static str| {
            move |error: io::Error| Failure::new(Exit::Failed, what).because(error.to_string())
        };
        match args.as_slice() {
            [] | ["show"] | ["show", _] => {
                let state = args
                    .get(1)
                    .map(PathBuf::from)
                    .or_else(kept_state)
                    .ok_or_else(|| {
                        Failure::new(Exit::Config, "there's no state folder to read")
                            .because("neither XDG_STATE_HOME nor HOME is set")
                            .try_this("clock-follows show STATE")
                    })?;
                if context.format().is_json() {
                    context.answer(&shown(&state))
                } else {
                    context.say(&said(&state, now(), context))
                }
                .map_err(failed("what it holds couldn't be said"))
            }
            ["follow", socket, state, timedatectl] => fs::create_dir_all(state)
                .and_then(|()| follow(Path::new(socket), Path::new(state), timedatectl))
                .map_err(failed("the clock couldn't follow the laptop")),
            ["put-back", state, timedatectl] => put_back(Path::new(state), timedatectl)
                .map_err(failed("the clock couldn't be put back")),
            ["show", ..] => Err(asked_wrongly("show takes one state folder at most")),
            ["follow", ..] => Err(asked_wrongly(
                "follow takes whereabouts' socket, a state folder and timedatectl",
            )),
            ["put-back", ..] => Err(asked_wrongly(
                "put-back takes a state folder and timedatectl",
            )),
            [word, ..] => Err(terminal_kit::flags::unknown(
                "command",
                word,
                &["show", "follow", "put-back"],
            )),
        }
    })
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
            zone_holds: Some(true),
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
    fn only_a_fix_that_differs_from_the_last_decided_is_decided_again() {
        let state = std::env::temp_dir().join(format!("clock-follows-{}", std::process::id()));
        fs::create_dir_all(&state).unwrap();
        let here = fix(300.0, Some("America/Edmonton"));
        let mut decided = Decided::in_state(&state);
        assert!(decided.differs(&here), "the first is decided");
        // One whose deciding failed isn't kept, so is decided again.
        assert!(
            Decided::in_state(&state).differs(&here),
            "one not kept is decided again"
        );
        decided.keep(here.clone());
        // A change only to the clock's zone, as Joel setting it by hand
        // makes, says the same fix.
        assert!(!decided.differs(&here), "the same is not");
        // Nor is it after a connection again or a restart.
        assert!(
            !Decided::in_state(&state).differs(&here),
            "nor after a start"
        );
        let later = Fix {
            at_s: Some(1_790_000_600),
            ..here
        };
        assert!(
            Decided::in_state(&state).differs(&later),
            "one found later is"
        );
        fs::remove_dir_all(&state).unwrap();
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
    fn a_fix_to_beacondbs_width_in_another_zone_moves_the_clock() {
        // BeaconDB places a laptop from the network's address to about
        // 25 km where it knows none of the Wi-Fi in reach.
        assert_eq!(
            decide(
                &fix(26_000.0, Some("America/Halifax")),
                "America/Toronto",
                None
            ),
            Decision::Move("America/Halifax".into()),
            "a fix to 26 km whose zone holds round its circle"
        );
    }

    #[test]
    fn a_fix_whose_circle_crosses_a_zones_line_never_moves_the_clock() {
        let crossed = Fix {
            zone_holds: Some(false),
            ..fix(26_000.0, Some("America/Winnipeg"))
        };
        assert_eq!(
            decide(&crossed, "America/Toronto", None),
            Decision::Stay,
            "a fix to 26 km whose circle keeps another time"
        );
        let near = Fix {
            zone_holds: Some(false),
            ..fix(3_000.0, Some("America/Winnipeg"))
        };
        assert_eq!(
            decide(&near, "America/Toronto", None),
            Decision::Stay,
            "however near, where its edge keeps another time"
        );
    }

    #[test]
    fn a_fix_unsaid_round_its_circle_moves_the_clock_only_when_near() {
        let named = Fix {
            zone_holds: None,
            ..fix(1_000.0, Some("America/Edmonton"))
        };
        assert_eq!(
            decide(&named, "America/Toronto", None),
            Decision::Move("America/Edmonton".into()),
            "a town Joel named"
        );
        let wide = Fix {
            zone_holds: None,
            ..fix(26_000.0, Some("America/Edmonton"))
        };
        assert_eq!(
            decide(&wide, "America/Toronto", None),
            Decision::Stay,
            "a fix to 26 km with nothing said of its circle"
        );
    }

    #[test]
    fn a_fix_wider_than_a_region_never_moves_the_clock() {
        // GeoClue's width where it knows only the country.
        assert_eq!(
            decide(&fix(300_000.0, Some("Asia/Tokyo")), "America/Toronto", None),
            Decision::Stay,
            "a country wide"
        );
        assert_eq!(
            decide(&fix(50_001.0, Some("Asia/Tokyo")), "America/Toronto", None),
            Decision::Stay,
            "past a region's width"
        );
        assert_eq!(
            decide(&fix(f64::NAN, Some("Asia/Tokyo")), "America/Toronto", None),
            Decision::Stay,
            "a width that's no number"
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
        assert!(
            put_back(&state, "false").is_err(),
            "a zone timedated refuses isn't put back"
        );
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
        assert!(
            !state.join("moved.json").exists(),
            "a move put back is let go"
        );
        fs::remove_dir_all(&state).unwrap();
    }
}
