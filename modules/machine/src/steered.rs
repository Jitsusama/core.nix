//! What the backlight program steers the built-in screen to, as it says it
//! in three files it writes whole (modules/backlight): the level it last
//! settled, the light it heard, what its curve asks for that light and
//! whether it follows the room, in `$XDG_RUNTIME_DIR/backlight-now`, with
//! the curve's points from `$XDG_STATE_HOME/backlight/learned`
//! (`brightness:built-in`); and the screen's warmth and night light, in
//! `$XDG_RUNTIME_DIR/backlight-warmth` (`warmth:built-in`). Read and
//! parsed here, they reach the shell over the socket it already holds, as
//! a record told only when it changes, so it reads no file of its own.

use std::path::Path;

use serde_json::{Value, json};

use crate::Root;

/// A fact a line in `backlight-now`: its name, a tab, its value.
const TAB: char = '\t';
/// A line of the curve's opening with this is no point.
const COMMENT: char = '#';
/// The curve's levels lie between these, and its lux are never below
/// none, as the backlight program's `Curve::through` keeps them.
const DIMMEST: f64 = 0.02;
const BRIGHTEST: f64 = 1.0;
const NO_LUX: f64 = 0.0;

/// Where the backlight program writes, as paths on the machine.
#[derive(Clone, Debug)]
pub struct Files {
    pub now: String,
    pub warmth: String,
    pub learned: String,
}

impl Files {
    /// Where they are for a session with this runtime folder and state
    /// folder, or, with no state folder said, under its home's
    /// `.local/state`, as the backlight program takes it.
    pub fn of(
        runtime: Option<String>,
        state: Option<String>,
        home: Option<String>,
    ) -> Option<Self> {
        let said = |folder: Option<String>| folder.filter(|folder| !folder.is_empty());
        let runtime = said(runtime)?;
        let state =
            said(state).or_else(|| said(home).map(|home| format!("{home}/.local/state")))?;
        Some(Self {
            now: format!("{runtime}/backlight-now"),
            warmth: format!("{runtime}/backlight-warmth"),
            learned: format!("{state}/backlight/learned"),
        })
    }

    /// Where they are for this process's session.
    pub fn of_session() -> Option<Self> {
        let var = |name: &str| std::env::var(name).ok();
        Self::of(var("XDG_RUNTIME_DIR"), var("XDG_STATE_HOME"), var("HOME"))
    }

    /// The folders they're in, each once, for a watch on them.
    pub fn folders(&self) -> Vec<String> {
        let mut folders: Vec<String> = [&self.now, &self.warmth, &self.learned]
            .iter()
            .filter_map(|file| Path::new(file).parent())
            .map(|folder| folder.to_string_lossy().into_owned())
            .collect();
        folders.sort();
        folders.dedup();
        folders
    }

    /// Whether a file of this name, in a folder watched, is one of them.
    pub fn names(&self, name: &str) -> bool {
        [&self.now, &self.warmth, &self.learned]
            .iter()
            .any(|file| Path::new(file).file_name().is_some_and(|leaf| leaf == name))
    }
}

/// The built-in screen's brightness, as the backlight program last
/// settled it and the curve it follows, or none where it says neither: it
/// isn't running, or has nothing to steer.
pub fn brightness(root: &Root, files: &Files) -> Option<Value> {
    let now = root.read(&files.now).and_then(|text| settled(&text));
    let curve = root
        .read(&files.learned)
        .map(|text| curve(&text))
        .unwrap_or_default();
    if now.is_none() && curve.is_empty() {
        return None;
    }
    let fact = |name: &str| now.as_ref().map_or(Value::Null, |now| now[name].clone());
    Some(json!({
        "key": "brightness:built-in",
        "kind": "brightness",
        "level_ratio": fact("level_ratio"),
        "lux": fact("lux"),
        "asks_ratio": fact("asks_ratio"),
        "following": fact("following"),
        "curve": curve,
    }))
}

/// What `backlight-now` says, or none while it says nothing, as when the
/// program has stopped and emptied it, or says it in a way the program
/// never writes. `following` is the program's: light heard and the lid
/// open, the curve setting the level. The lux and what the curve asks for
/// them are null while no light is heard.
fn settled(text: &str) -> Option<Value> {
    let fact = |name: &str| {
        text.lines()
            .filter_map(|line| line.split_once(TAB))
            .find(|(said, _)| *said == name)
            .map(|(_, value)| value)
    };
    let level = figure(fact("level")?)?;
    let following = match fact("following")? {
        "yes" => true,
        "no" => false,
        _ => return None,
    };
    Some(json!({
        "level_ratio": level,
        "lux": fact("lux").and_then(figure),
        "asks_ratio": fact("asks").and_then(figure),
        "following": following,
    }))
}

/// The curve's points, dimmest first, each kept where the program keeps
/// it; or none for a file it would refuse, any line of it not two
/// figures, as it falls back to its standard curve then.
fn curve(text: &str) -> Vec<Value> {
    let points: Option<Vec<(f64, f64)>> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(COMMENT))
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields[..] {
                [lux, level] => Some((figure(lux)?, figure(level)?)),
                _ => None,
            }
        })
        .collect();
    let mut points: Vec<(f64, f64)> = points
        .unwrap_or_default()
        .into_iter()
        .map(|(lux, level)| (lux.max(NO_LUX), level.clamp(DIMMEST, BRIGHTEST)))
        .collect();
    points.sort_by(|one, other| one.0.total_cmp(&other.0));
    points
        .into_iter()
        .map(|(lux, level)| json!({"lux": lux, "level_ratio": level}))
        .collect()
}

/// A figure as the program writes one, or none for anything else.
fn figure(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

/// The built-in screen's warmth, or none where nothing warms it: no file,
/// or `{}`, which the program leaves as it stops. A program older than
/// the night light says only the warmth and whether it follows the room.
pub fn warmth(root: &Root, files: &Files) -> Option<Value> {
    let said: Value = serde_json::from_str(&root.read(&files.warmth)?).ok()?;
    // Each figure as it was written, so a whole number stays one.
    let number = |name: &str| Some(&said[name]).filter(|value| value.is_number()).cloned();
    let kelvin = number("kelvin")?;
    let follows = said["followsTheRoom"].as_bool()?;
    let number = |name: &str| number(name).unwrap_or(Value::Null);
    Some(json!({
        "key": "warmth:built-in",
        "kind": "warmth",
        "kelvin": kelvin,
        "follows_the_room": follows,
        "sees_colour": said["seesColour"].as_bool().unwrap_or(false),
        "room_white_kelvin": number("roomWhite"),
        "night_light": said["nightLight"].as_bool().unwrap_or(false),
        "night_white_kelvin": number("nightWhite"),
        // The night under way's, or the next's, in milliseconds since the
        // epoch, for the shell to say by its clock.
        "sunset_ms": number("sunset"),
        "sunrise_ms": number("sunrise"),
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn session(test: &str) -> (Root, Files) {
        let folder =
            std::env::temp_dir().join(format!("machine-steered-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("run/user/1000")).unwrap();
        fs::create_dir_all(folder.join("home/joel/.local/state/backlight")).unwrap();
        let files = Files::of(
            Some("/run/user/1000".into()),
            None,
            Some("/home/joel".into()),
        )
        .unwrap();
        (Root::new(folder), files)
    }

    fn write(root: &Root, path: &str, text: &str) {
        fs::write(root.path(path), text).unwrap();
    }

    #[test]
    fn the_files_are_the_sessions() {
        let files = Files::of(Some("/run/user/1000".into()), Some("/state".into()), None).unwrap();
        assert_eq!(files.now, "/run/user/1000/backlight-now");
        assert_eq!(files.warmth, "/run/user/1000/backlight-warmth");
        assert_eq!(files.learned, "/state/backlight/learned");
        assert_eq!(files.folders(), ["/run/user/1000", "/state/backlight"]);
        assert!(files.names("learned") && files.names("backlight-now"));
        assert!(
            !files.names("backlight-now.writing"),
            "a file being written isn't one"
        );
        assert!(
            Files::of(None, Some("/state".into()), None).is_none(),
            "no runtime folder"
        );
    }

    #[test]
    fn the_level_settled_says_the_light_heard_and_what_the_curve_asks() {
        let (root, files) = session("settled");
        write(
            &root,
            &files.now,
            "level\t0.380\nlux\t28\nasks\t0.950\nfollowing\tyes\n",
        );
        let said = brightness(&root, &files).unwrap();
        assert_eq!(said["key"], "brightness:built-in");
        assert_eq!(said["level_ratio"], json!(0.38));
        assert_eq!(said["lux"], json!(28.0));
        assert_eq!(said["asks_ratio"], json!(0.95));
        assert_eq!(said["following"], json!(true));
        assert_eq!(said["curve"], json!([]), "no curve learned yet");
        // With no light heard, it says the level alone.
        write(&root, &files.now, "level\t0.600\nfollowing\tno\n");
        let said = brightness(&root, &files).unwrap();
        assert_eq!(
            (&said["lux"], &said["asks_ratio"]),
            (&Value::Null, &Value::Null)
        );
        assert_eq!(said["following"], json!(false));
    }

    #[test]
    fn a_level_said_otherwise_says_nothing() {
        let (root, files) = session("unsaid");
        for text in [
            "",
            "level\t0.600\nasks\t0.950\n",
            "level\tbright\nfollowing\tyes\n",
            "level\t0.6\nfollowing\tmaybe\n",
        ] {
            write(&root, &files.now, text);
            assert_eq!(brightness(&root, &files), None, "{text:?}");
        }
    }

    #[test]
    fn the_curve_is_dimmest_first_and_kept_where_the_program_keeps_it() {
        let (root, files) = session("curve");
        write(
            &root,
            &files.learned,
            "# the curve\n28 0.950\n\n6 0.377\n-5 0.010\n100 1.500\n",
        );
        let said = brightness(&root, &files).unwrap();
        assert_eq!(said["level_ratio"], Value::Null, "nothing settled yet");
        assert_eq!(
            said["curve"],
            json!([
                {"lux": 0.0, "level_ratio": 0.02},
                {"lux": 6.0, "level_ratio": 0.377},
                {"lux": 28.0, "level_ratio": 0.95},
                {"lux": 100.0, "level_ratio": 1.0},
            ])
        );
        // A line the program would refuse leaves none of it.
        for text in ["28 0.950\n6\n", "0x10 0.5\n", "28 0.9 1\n"] {
            write(&root, &files.learned, text);
            assert_eq!(brightness(&root, &files), None, "{text:?}");
        }
    }

    #[test]
    fn the_warmth_says_the_night_light_and_the_room() {
        let (root, files) = session("warmth");
        write(
            &root,
            &files.warmth,
            r#"{"kelvin": 4200, "followsTheRoom": true, "seesColour": true, "roomWhite": 5100,
                "nightLight": true, "nightWhite": 3400, "sunset": 1762815600000,
                "sunrise": 1762863000000}"#,
        );
        let said = warmth(&root, &files).unwrap();
        assert_eq!(said["key"], "warmth:built-in");
        assert_eq!(said["kelvin"], json!(4200));
        assert_eq!(said["follows_the_room"], json!(true));
        assert_eq!(said["sees_colour"], json!(true));
        assert_eq!(said["room_white_kelvin"], json!(5100));
        assert_eq!(said["night_light"], json!(true));
        assert_eq!(said["night_white_kelvin"], json!(3400));
        assert_eq!(said["sunset_ms"], json!(1_762_815_600_000u64));
        assert_eq!(said["sunrise_ms"], json!(1_762_863_000_000u64));
        // A program before the night light says the warmth alone.
        write(
            &root,
            &files.warmth,
            r#"{"kelvin": 6500, "followsTheRoom": false}"#,
        );
        let said = warmth(&root, &files).unwrap();
        assert_eq!(said["night_light"], json!(false));
        assert_eq!(said["sunset_ms"], Value::Null);
        // One stopped leaves `{}`, which warms nothing.
        write(&root, &files.warmth, "{}\n");
        assert_eq!(warmth(&root, &files), None);
    }
}
