//! Capture's Recent's rows, made ready for the shell (ready-rows.md, the
//! ready-row form): each thing taken in the folders a watch names,
//! newest first, said as launcher/Captured.js and launcher/Recent.js said
//! it in the shell. Its side says when it was taken, and its folder too
//! where that isn't the usual one; its drawer says where it is and what
//! kind of file, shows the picture, then when it was taken, its size, and
//! a picture's dimensions or a recording's length once file-preview has
//! said them, which is asked only while the row is ahead (`shown`). Where
//! file-preview isn't answering, the fact it would have said says so.
//! What the shell attaches to a row, its acts, it makes by its path
//! (launcher/Capture.qml).

use ready_rows::numbers::Numbers;
use ready_rows::time::{Form, at};
use serde_json::{Value, json};

/// A number keeps to its unit, and a dimension's two sides to the ×.
const KEPT: &str = "\u{a0}";
/// SI's steps, largest first, each a thousand of the one after it.
const UNITS: [(f64, &str); 4] = [(1e12, "TB"), (1e9, "GB"), (1e6, "MB"), (1e3, "kB")];

/// One thing taken, as the folders' list says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taken {
    pub path: String,
    /// When it was taken, in milliseconds since 1970.
    pub at: i64,
    /// How big it is, in bytes.
    pub size: u64,
}

/// The words Capture's rows are said in, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// "in {folder}".
    pub within: String,
    pub picture: String,
    pub video: String,
    /// What a fact file-preview would have said says while it isn't
    /// answering.
    pub silent: String,
    /// The facts' names: Taken, Size, Dimensions and Length.
    pub taken: String,
    pub size: String,
    pub dimensions: String,
    pub length: String,
    /// The glyph a file that's neither a picture nor a video wears.
    pub file: String,
}

impl Words {
    /// Its words from the vocabulary, or which one is missing.
    ///
    /// # Errors
    ///
    /// The id of the first word the vocabulary doesn't hold.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let of = |id: &str| {
            vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id))
                .ok_or_else(|| id.to_owned())
        };
        let text = |value: &Value, id: &str| value.as_str().map(str::to_owned).ok_or(id.to_owned());
        let recent = of("capture:recent")?;
        let said = |field: &str| text(&recent["said"][field], "capture:recent");
        Ok(Words {
            within: said("in")?,
            picture: said("picture")?,
            video: said("video")?,
            silent: said("silent")?,
            taken: text(&of("capture:taken")?["name"], "capture:taken")?,
            size: text(&of("file-chooser:size")?["name"], "file-chooser:size")?,
            dimensions: text(&of("capture:dimensions")?["name"], "capture:dimensions")?,
            length: text(&of("capture:length")?["name"], "capture:length")?,
            file: text(&of("path:file")?["glyph"], "path:file")?,
        })
    }
}

/// Each thing listed, from the list's lines: when it was taken in
/// seconds, its size, then its path, each apart by a tab.
#[must_use]
pub fn listed<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<Taken> {
    lines
        .into_iter()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let seconds: f64 = parts.next()?.parse().ok()?;
            let size = parts.next()?.parse().ok()?;
            let path = parts.next().filter(|path| path.starts_with('/'))?;
            #[expect(clippy::cast_possible_truncation, reason = "a time in ms fits an i64")]
            let at = (seconds * 1000.0).round() as i64;
            Some(Taken {
                path: path.to_owned(),
                at,
                size,
            })
        })
        .collect()
}

/// The kind of thing a file is by its ending: a picture, a video, or
/// neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Picture,
    Video,
    Other,
}

fn kind(path: &str) -> Kind {
    let ending = path.rsplit_once('.').map(|(_, end)| end.to_lowercase());
    match ending.as_deref() {
        Some("png" | "jpg" | "jpeg" | "webp" | "avif" | "gif") => Kind::Picture,
        Some("mp4" | "mkv" | "webm" | "mov") => Kind::Video,
        _ => Kind::Other,
    }
}

/// A folder as Joel reads it, from home as ~.
fn from_home(folder: &str, home: &str) -> String {
    match folder.strip_prefix(home) {
        Some("") => "~".to_owned(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => folder.to_owned(),
    }
}

/// How big a file is, as its row and drawer say it: "512 B", "1.50 kB",
/// "1.00 MB" for what would round to 1000 kB.
#[must_use]
pub fn weighed(bytes: u64) -> String {
    let numbers = Numbers::default();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a size is said to three figures"
    )]
    let bytes = bytes as f64;
    let Some(at) = UNITS.iter().position(|(unit, _)| bytes >= *unit) else {
        return format!("{bytes}{KEPT}B");
    };
    let (unit, name) = UNITS[at];
    let scaled = bytes / unit;
    let said = numbers.significant(scaled);
    if said
        .replace(',', "")
        .parse::<f64>()
        .is_ok_and(|figure| figure >= 1000.0)
        && at > 0
    {
        let (_, next) = UNITS[at - 1];
        return format!("{}{KEPT}{next}", numbers.significant(scaled / 1000.0));
    }
    format!("{said}{KEPT}{name}")
}

/// How long a recording lasts: "0:42", "12:05", "1:02:03".
#[must_use]
pub fn lasting(seconds: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a length"
    )]
    let whole = seconds.max(0.0).round() as u64;
    let (hours, minutes, rest) = (whole / 3600, whole % 3600 / 60, whole % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{rest:02}")
    } else {
        format!("{minutes}:{rest:02}")
    }
}

/// Where the rows are, and what's usual of them.
pub struct Placing<'a> {
    /// Joel's home, said as ~.
    pub home: &'a str,
    /// The folder most things are taken into, which a row needn't name.
    pub usual: &'a str,
}

/// One row, `shown` being what file-preview said of it where it's ahead
/// (`None` where it isn't), and whether file-preview is answering.
#[must_use]
pub fn row(
    one: &Taken,
    shown: Option<&Value>,
    answering: bool,
    placing: &Placing,
    words: &Words,
) -> Value {
    let (folder, name) = one.path.rsplit_once('/').unwrap_or(("", one.path.as_str()));
    let within = from_home(folder, placing.home);
    let mut side = vec![at(one.at, Form::Came)];
    if within != from_home(placing.usual, placing.home) {
        side.push(json!({"said": within, "keep": 1}));
    }
    let glyph = match kind(&one.path) {
        Kind::Picture => "picture",
        Kind::Video => "film",
        Kind::Other => words.file.as_str(),
    };
    json!({
        "key": format!("row:taken:{}", one.path),
        "thing": format!("file:{}", one.path),
        "place": "taken",
        "kind": "file",
        "name": name,
        "glyph": glyph,
        "hue": "program",
        "side": side,
        "drawer": drawer(one, name, &within, shown, answering, words),
        "held": {"path": one.path, "at": one.at, "size": one.size},
    })
}

/// Its drawer: the folder it's in, then what kind of file it is, above;
/// its name; the picture; then when it was taken, its size, and what
/// file-preview said of it, or that it isn't answering.
fn drawer(
    one: &Taken,
    name: &str,
    within: &str,
    shown: Option<&Value>,
    answering: bool,
    words: &Words,
) -> Value {
    let kind = kind(&one.path);
    let said = match kind {
        Kind::Picture => words.picture.as_str(),
        Kind::Video => words.video.as_str(),
        Kind::Other => "",
    };
    let mut items = vec![json!({
        "said": words.within.replace("{folder}", within), "keep": 2, "path": within,
    })];
    if !said.is_empty() {
        items.push(json!({"said": said, "keep": 1}));
    }
    let size = shown
        .and_then(|shown| shown["size"].as_u64())
        .unwrap_or(one.size);
    let mut facts = vec![
        json!({"name": words.taken, "at": one.at, "as": Form::Moment.name()}),
        json!({"name": words.size, "said": weighed(size)}),
    ];
    let own = shown.is_some_and(|shown| shown["picture"] == one.path.as_str());
    let across = shown
        .and_then(|shown| shown["across"].as_u64())
        .filter(|across| *across > 0);
    if let (true, Some(across)) = (own, across) {
        let down = shown.and_then(|shown| shown["down"].as_u64()).unwrap_or(0);
        facts.push(json!({
            "name": words.dimensions, "said": format!("{across}{KEPT}\u{d7}{KEPT}{down}"),
        }));
    } else if let Some(seconds) = shown.and_then(|shown| shown["seconds"].as_f64()) {
        facts.push(json!({"name": words.length, "said": lasting(seconds)}));
    } else if !answering && kind != Kind::Other {
        let id = if kind == Kind::Picture {
            &words.dimensions
        } else {
            &words.length
        };
        facts.push(json!({"name": id, "said": words.silent}));
    }
    let mut drawn = json!({
        "eyebrowItems": items,
        "eyebrowNames": true,
        "title": name,
        "facts": facts,
    });
    if let Some(preview) = preview(one, shown) {
        drawn["preview"] = preview;
    }
    drawn
}

/// What the drawer previews: file-preview's picture of it, a recording's
/// frame, or, before it's said, a picture as it is; its path and size,
/// never its bytes.
fn preview(one: &Taken, shown: Option<&Value>) -> Option<Value> {
    if let Some(picture) = shown.and_then(|shown| shown["picture"].as_str()) {
        let side = |field: &str| shown.and_then(|shown| shown[field].as_u64()).unwrap_or(0);
        return Some(json!({"picture": picture, "across": side("across"), "down": side("down")}));
    }
    (kind(&one.path) == Kind::Picture).then(|| json!({"picture": one.path, "across": 0, "down": 0}))
}

/// Every row, newest first, of the things taken in `folders`.
#[must_use]
pub fn rows(
    taken: &[Taken],
    folders: &[String],
    most: usize,
    shown: &dyn Fn(&str) -> (Option<Value>, bool),
    placing: &Placing,
    words: &Words,
) -> Vec<Value> {
    let mut within: Vec<&Taken> = taken
        .iter()
        .filter(|one| {
            one.path
                .rsplit_once('/')
                .is_some_and(|(folder, _)| folders.iter().any(|wanted| wanted == folder))
        })
        .collect();
    within.sort_by(|one, other| {
        other
            .at
            .cmp(&one.at)
            .then_with(|| one.path.cmp(&other.path))
    });
    within
        .into_iter()
        .take(most)
        .map(|one| {
            let (said, answering) = shown(&one.path);
            row(one, said.as_ref(), answering, placing, words)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words {
            within: "in {folder}".into(),
            picture: "picture".into(),
            video: "video".into(),
            silent: "not answering".into(),
            taken: "Taken".into(),
            size: "Size".into(),
            dimensions: "Dimensions".into(),
            length: "Length".into(),
            file: "file".into(),
        }
    }

    const PLACING: Placing = Placing {
        home: "/home/joel",
        usual: "/home/joel/Pictures/Screenshots",
    };

    fn one(path: &str, at: i64) -> Taken {
        Taken {
            path: path.into(),
            at,
            size: 1536,
        }
    }

    #[test]
    fn the_list_says_when_how_big_and_where_each_is() {
        let lines = [
            "1700000000.5\t1536\t/home/joel/Pictures/Screenshots/a.png",
            "junk",
            "1\t2\trel",
        ];
        assert_eq!(
            listed(lines),
            vec![one(
                "/home/joel/Pictures/Screenshots/a.png",
                1_700_000_000_500
            )],
            "only a whole line with a path is a thing taken"
        );
    }

    #[test]
    fn sizes_keep_three_figures_and_lengths_their_clock() {
        assert_eq!(weighed(512), "512\u{a0}B", "under a kilobyte, in bytes");
        assert_eq!(weighed(1536), "1.54\u{a0}kB", "three figures, zeros kept");
        assert_eq!(
            weighed(999_600),
            "1.00\u{a0}MB",
            "what rounds to 1000 kB is a megabyte"
        );
        assert_eq!(lasting(42.4), "0:42", "under a minute");
        assert_eq!(lasting(3723.0), "1:02:03", "past an hour");
    }

    #[test]
    fn a_picture_says_when_and_its_folder_only_where_it_isnt_the_usual() {
        let none = |_: &str| (None, true);
        let taken = [
            one("/home/joel/Pictures/Screenshots/old.png", 10),
            one("/home/joel/Videos/Recordings/new.mp4", 20),
            one("/home/joel/elsewhere/skip.png", 30),
        ];
        let folders = [
            "/home/joel/Pictures/Screenshots".to_owned(),
            "/home/joel/Videos/Recordings".to_owned(),
        ];
        let made = rows(&taken, &folders, 9, &none, &PLACING, &words());
        let keys: Vec<&str> = made.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            [
                "row:taken:/home/joel/Videos/Recordings/new.mp4",
                "row:taken:/home/joel/Pictures/Screenshots/old.png"
            ],
            "newest first, only from the folders asked"
        );
        assert_eq!(
            made[0]["side"],
            json!([{"at": 20, "as": "came"}, {"said": "~/Videos/Recordings", "keep": 1}])
        );
        assert_eq!(
            made[1]["side"],
            json!([{"at": 10, "as": "came"}]),
            "the usual folder isn't said"
        );
        assert_eq!(
            made[1]["glyph"], "picture",
            "a picture wears a picture's mark"
        );
        assert_eq!(made[0]["glyph"], "film", "a video a film's");
        assert_eq!(
            rows(&taken, &folders, 1, &none, &PLACING, &words()).len(),
            1,
            "at most as many as asked"
        );
    }

    #[test]
    fn a_drawer_says_where_and_what_then_its_facts_once_file_preview_has() {
        let picture = one("/home/joel/Pictures/Screenshots/a.png", 10);
        let before = row(&picture, None, true, &PLACING, &words());
        assert_eq!(
            before["drawer"],
            json!({
                "eyebrowItems": [
                    {
                        "said": "in ~/Pictures/Screenshots", "keep": 2,
                        "path": "~/Pictures/Screenshots"
                    },
                    {"said": "picture", "keep": 1}
                ],
                "eyebrowNames": true,
                "title": "a.png",
                "facts": [
                    {"name": "Taken", "at": 10, "as": "moment"},
                    {"name": "Size", "said": "1.54\u{a0}kB"}
                ],
                "preview": {
                    "picture": "/home/joel/Pictures/Screenshots/a.png", "across": 0, "down": 0
                }
            }),
            "before file-preview says, the picture as it is and no dimensions"
        );
        let said = json!({"picture": picture.path, "across": 1920, "down": 1080, "size": 2048});
        let after = row(&picture, Some(&said), true, &PLACING, &words());
        assert_eq!(
            after["drawer"]["facts"][1]["said"], "2.05\u{a0}kB",
            "its size as file-preview read it"
        );
        assert_eq!(
            after["drawer"]["facts"][2],
            json!({"name": "Dimensions", "said": "1920\u{a0}\u{d7}\u{a0}1080"})
        );
        assert_eq!(
            after["drawer"]["preview"],
            json!({"picture": picture.path, "across": 1920, "down": 1080})
        );
        let silent = row(&picture, None, false, &PLACING, &words());
        assert_eq!(
            silent["drawer"]["facts"][2],
            json!({"name": "Dimensions", "said": "not answering"}),
            "where file-preview isn't answering, the fact it would say says so"
        );
        let video = one("/home/joel/Videos/Recordings/b.mp4", 10);
        let frame =
            json!({"picture": "/run/frame.png", "across": 640, "down": 360, "seconds": 42.0});
        let recorded = row(&video, Some(&frame), true, &PLACING, &words());
        assert_eq!(
            recorded["drawer"]["facts"][2],
            json!({"name": "Length", "said": "0:42"}),
            "a video's length"
        );
        assert_eq!(
            recorded["drawer"]["preview"]["picture"], "/run/frame.png",
            "and its frame"
        );
        assert!(
            row(&video, None, true, &PLACING, &words())["drawer"]
                .get("preview")
                .is_none(),
            "no frame yet, no preview"
        );
    }
}
