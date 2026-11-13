//! What a file shows of itself in the launcher's chooser of files
//! (FileChooser.qml), asked as a file is selected, so the drawer beside it
//! can show a preview with its facts.
//!
//! `file-preview show PATH` prints one line of JSON about PATH: its `size`
//! in bytes and when it last `changed`, in milliseconds since the epoch,
//! always; then what it is. A picture says its `kind`, by its ending, and
//! its width and height (`across`, `down`), and is its own `picture`. A PDF
//! says how many `pages` it has and a video how many `seconds` it runs,
//! each with a `picture` made of its first page or of a frame a tenth of
//! the way in, and that picture's width and height. A file of text gives
//! its first whole `lines` and the `count` of them all, or, past 64 MB,
//! of those in its first 64 MB with `more` true; a folder how many
//! `items` it shows and the `names` of its first; and anything else only
//! its facts, which is all a FIFO, a socket or a device says, as none of
//! them is opened.
//!
//! `file-preview items FOLDER` prints one line of JSON saying, for each
//! folder inside FOLDER, how many items it shows, by its path.
//!
//! `file-preview serve` answers both, asked as lines on its input, `show
//! PATH` or `items FOLDER`, each with one line of JSON on its output, in
//! the order asked, `{"error": ...}` for one it can't, until its input
//! ends. The chooser keeps one running for as long as it's asked, so
//! nothing is started for each file selected or folder read.
//!
//! A picture made of a PDF or a video is kept under the cache, named by the
//! file's path, size and change time, so it's made once for each version
//! of the file. Each one's change time is when it was last shown, and once
//! they pass 128 MB in all the longest unshown go first. The programs that
//! make them, poppler's `pdfinfo` and `pdftoppm` and `FFmpeg`'s `ffprobe`
//! and `ffmpeg`, are found on the path.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit, save that its text is already a line of JSON,
//! since that's what the chooser reads; `--json` gives `show` and `items`
//! the schema an answer names, and `serve` keeps its lines as they are.

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::UNIX_EPOCH;

use serde_json::{Map, Value, json};
use terminal_kit::{Context, Exit, Failure, Help};

const USAGE: &str =
    "usage: file-preview show PATH | file-preview items FOLDER | file-preview serve";

/// How wide a picture made of a PDF or a video is drawn, in pixels: the
/// drawer at its widest, 520, at a scale of 2.
const MADE_WIDE: u32 = 1040;
/// How much of a file is read to tell whether it's text, and its lines.
const READ: usize = 4096;
/// How many of its lines a file of text shows, and of its items a folder.
const LINES: usize = 16;
const NAMES: usize = 8;
/// How much of a file of text is read to count its lines, so a log of
/// gigabytes never holds up the file selected after it.
const COUNTED_MOST: u64 = 64 * 1024 * 1024;
/// How much the pictures made of files may hold in all.
const KEPT_MOST: u64 = 128 * 1024 * 1024;
/// The most of one line shown, in characters, before it's cut.
const LINE_WIDE: usize = 160;

const PICTURES: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];
const VIDEOS: &[&str] = &["mp4", "m4v", "mkv", "webm", "mov", "avi"];

fn help() -> Help {
    Help::new("file-preview", "what a file shows of itself in the chooser")
        .command("show PATH", "a line of JSON: its facts and preview")
        .command("items FOLDER", "how many items each folder inside shows")
        .command("serve", "answers show and items, asked a line each")
        .example("file-preview show notes.md", "its size, change and lines")
        .example("file-preview items ~", "each folder in home, counted")
        .exit(Exit::Failed, "it couldn't be read, or its answer said")
        .exit(Exit::NotFound, "there's nothing at PATH")
}

fn main() -> ExitCode {
    terminal_kit::main(
        &help(),
        env!("CARGO_PKG_VERSION"),
        |context| match context.args.as_slice() {
            [verb] if verb == "serve" => serve(io::stdin().lock(), &mut io::stdout().lock()),
            [verb, path] if verb == "show" => {
                let path = Path::new(path);
                let said = shown(path).map_err(|error| {
                    let exit = if path.symlink_metadata().is_err() {
                        Exit::NotFound
                    } else {
                        Exit::Failed
                    };
                    Failure::new(exit, format!("{} can't be shown", path.display())).because(error)
                })?;
                said_as(context, said, "file-preview.show/1")
            }
            [verb, folder] if verb == "items" => said_as(
                context,
                items_inside(Path::new(folder)),
                "file-preview.items/1",
            ),
            [verb, ..] if ["serve", "show", "items"].contains(&verb.as_str()) => Err(
                asked_wrongly("show takes a PATH, items a FOLDER and serve nothing"),
            ),
            [verb, ..] => Err(terminal_kit::flags::unknown(
                "command",
                verb,
                &["show", "items", "serve"],
            )),
            [] => Err(asked_wrongly(
                "file-preview needs a command: show, items or serve",
            )),
        },
    )
}

fn asked_wrongly(what: &str) -> Failure {
    Failure::new(Exit::Usage, what).try_this("file-preview --help")
}

/// Says `said` as its line of JSON, with its `schema` when JSON was asked.
fn said_as(context: &Context, mut said: Value, schema: &str) -> Result<(), Failure> {
    let unsaid = |error: io::Error| {
        Failure::new(Exit::Failed, "it couldn't be said").because(error.to_string())
    };
    if context.format().is_json() {
        said["schema"] = json!(schema);
        return context.answer(&said).map_err(unsaid);
    }
    writeln!(io::stdout().lock(), "{said}").map_err(unsaid)
}

/// Answers each line asked with one of JSON, until what's asked ends.
fn serve(asked: impl io::BufRead, out: &mut impl Write) -> Result<(), Failure> {
    let broke = |error: io::Error| {
        Failure::new(Exit::Failed, "serving broke off").because(error.to_string())
    };
    for line in asked.lines() {
        let said = answer(&line.map_err(broke)?);
        writeln!(out, "{said}")
            .and_then(|()| out.flush())
            .map_err(broke)?;
    }
    Ok(())
}

fn answer(line: &str) -> Value {
    match line.split_once(' ') {
        Some(("show", path)) => {
            shown(Path::new(path)).unwrap_or_else(|error| json!({ "error": error }))
        }
        Some(("items", folder)) => items_inside(Path::new(folder)),
        _ => json!({ "error": USAGE }),
    }
}

/// What a file or a folder shows of itself.
fn shown(path: &Path) -> Result<Value, String> {
    let facts = fs::metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let changed = facts
        .modified()
        .ok()
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .unwrap_or_default();
    let mut said = Map::new();
    said.insert("size".into(), json!(facts.len()));
    // Milliseconds since 1970 fit a u64 for some half a billion years.
    let millis = u64::try_from(changed.as_millis()).unwrap_or(u64::MAX);
    said.insert("changed".into(), json!(millis));
    if facts.is_dir() {
        let (count, names) = listed(path);
        said.insert("items".into(), json!(count));
        said.insert("names".into(), json!(names));
        return Ok(Value::Object(said));
    }
    // Only a file is opened: a FIFO, a socket or a device would hold the
    // read open, and every ask behind it, until something wrote to it.
    if !facts.is_file() {
        return Ok(Value::Object(said));
    }
    let ending = ending_of(path);
    if !ending.is_empty() {
        said.insert("kind".into(), json!(ending.to_uppercase()));
    }
    let version = Version {
        path,
        size: facts.len(),
        changed: changed.as_nanos(),
    };
    if PICTURES.contains(&ending.as_str()) {
        if let Ok(size) = imagesize::size(path) {
            said.insert("across".into(), json!(size.width));
            said.insert("down".into(), json!(size.height));
            said.insert("picture".into(), json!(path));
        }
    } else if ending == "pdf" {
        if let Some(pages) = pdf_pages(path) {
            said.insert("pages".into(), json!(pages));
        }
        picture_of(&mut said, made(&version, first_page));
    } else if VIDEOS.contains(&ending.as_str()) {
        let seconds = video_seconds(path);
        if let Some(seconds) = seconds {
            said.insert("seconds".into(), json!(seconds));
        }
        let at = seconds.unwrap_or(0.0) / 10.0;
        picture_of(&mut said, made(&version, |from, to| frame_at(from, to, at)));
    } else if let Some(lines) = text_lines(path) {
        said.insert("lines".into(), json!(lines));
        if let Some((count, whole)) = lines_in(path, COUNTED_MOST) {
            said.insert("count".into(), json!(count));
            if !whole {
                said.insert("more".into(), json!(true));
            }
        }
    }
    Ok(Value::Object(said))
}

/// A file's ending, lowercase, or nothing: "Plan.MD" ends in md, and
/// ".bashrc" in nothing, being only hidden.
fn ending_of(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    match name.rfind('.') {
        Some(at) if at > 0 => name[at + 1..].to_lowercase(),
        _ => String::new(),
    }
}

/// How many items a folder shows, its hidden ones left out as the chooser
/// leaves them, and the names of its first: its folders, then its files,
/// each by name, a folder's ending in a slash.
fn listed(folder: &Path) -> (usize, Vec<String>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return (0, Vec::new());
    };
    let mut shown: Vec<(bool, String)> = entries
        .filter_map(Result::ok)
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let inside = entry.path().is_dir();
            (!inside, name)
        })
        .filter(|(_, name)| !name.starts_with('.'))
        .collect();
    shown.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    let names = shown
        .iter()
        .take(NAMES)
        .map(|(file, name)| {
            if *file {
                name.clone()
            } else {
                format!("{name}/")
            }
        })
        .collect();
    (shown.len(), names)
}

/// How many items each folder inside a folder shows, by its path.
fn items_inside(folder: &Path) -> Value {
    let mut counted = Map::new();
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                counted.insert(path.to_string_lossy().into_owned(), json!(listed(&path).0));
            }
        }
    }
    Value::Object(counted)
}

/// A file's first whole lines, if it's text: no NUL in what's read and
/// UTF-8 throughout, a character cut at the end of what's read aside. A
/// line the read ends inside is left out, tabs read as four spaces, and
/// anything else that would draw nothing is dropped.
fn text_lines(path: &Path) -> Option<Vec<String>> {
    let mut read = Vec::with_capacity(READ);
    fs::File::open(path)
        .ok()?
        .take(READ as u64 + 1)
        .read_to_end(&mut read)
        .ok()?;
    lines_of(&read)
}

/// How many lines a file of text has, read through once, and whether
/// that's all of them: its line ends, and one more for a last line with
/// none; past `most` bytes, only the whole lines in what was read.
fn lines_in(path: &Path, most: u64) -> Option<(u64, bool)> {
    let mut file = fs::File::open(path).ok()?.take(most + 1);
    let mut buffer = vec![0; 64 * 1024];
    let mut count = 0;
    let mut read_in_all = 0;
    let mut last = b'\n';
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        read_in_all += read as u64;
        count += counted_ends(&buffer[..read]);
        last = buffer[read - 1];
    }
    if read_in_all > most {
        return Some((count, false));
    }
    Some((if last == b'\n' { count } else { count + 1 }, true))
}

fn counted_ends(read: &[u8]) -> u64 {
    read.iter().filter(|each| **each == b'\n').map(|_| 1).sum()
}

fn lines_of(read: &[u8]) -> Option<Vec<String>> {
    let whole = read.len() <= READ;
    let read = &read[..read.len().min(READ)];
    if read.contains(&0) {
        return None;
    }
    let text = match std::str::from_utf8(read) {
        Ok(text) => text,
        // A character cut by the end of the read is no sign of anything else.
        Err(error) if !whole && error.error_len().is_none() => {
            std::str::from_utf8(&read[..error.valid_up_to()]).ok()?
        }
        Err(_) => return None,
    };
    let mut lines: Vec<&str> = text.lines().collect();
    if !whole && !text.ends_with('\n') {
        lines.pop();
    }
    Some(
        lines
            .into_iter()
            .take(LINES)
            .map(|line| {
                let shown: String = line
                    .replace('\t', "    ")
                    .chars()
                    .filter(|each| !each.is_control())
                    .take(LINE_WIDE)
                    .collect();
                shown.trim_end().to_owned()
            })
            .collect(),
    )
}

/// One version of a file, which a picture made of it is kept for.
struct Version<'a> {
    path: &'a Path,
    size: u64,
    changed: u128,
}

impl Version<'_> {
    /// Its name under the cache, the same for the same path, size and
    /// change time: FNV-1a over the three, which is stable from build to
    /// build as Rust's own hasher isn't.
    fn name(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let said = format!("{}\0{}\0{}", self.path.display(), self.size, self.changed);
        for byte in said.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{hash:016x}.png")
    }
}

/// Where pictures made of files are kept.
fn kept_in() -> Option<PathBuf> {
    let cache = env::var_os("XDG_CACHE_HOME")
        .filter(|cache| !cache.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(cache.join("open-with").join("previews"))
}

/// The picture kept for this version of a file, made by `make` if it
/// isn't already: written beside where it goes and then moved there, so
/// one half made is never shown.
fn made(version: &Version, make: impl Fn(&Path, &Path) -> bool) -> Option<PathBuf> {
    let folder = kept_in()?;
    let kept = folder.join(version.name());
    if kept.is_file() {
        // Shown again, so it's the last to go.
        let _ = fs::File::options()
            .append(true)
            .open(&kept)
            .and_then(|file| file.set_modified(std::time::SystemTime::now()));
        return Some(kept);
    }
    fs::create_dir_all(&folder).ok()?;
    let making = folder.join(format!(".{}.{}.png", version.name(), std::process::id()));
    if make(version.path, &making) && making.is_file() && fs::rename(&making, &kept).is_ok() {
        pruned(&folder, KEPT_MOST);
        return Some(kept);
    }
    let _ = fs::remove_file(&making);
    None
}

/// Removes the pictures kept longest unshown until those left hold no
/// more than `most` bytes, the one just made being the newest, so it's
/// kept. One being made, its name starting with a dot, is left alone.
fn pruned(folder: &Path, most: u64) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let mut kept: Vec<(std::time::SystemTime, u64, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|entry| {
            let facts = entry.metadata().ok()?;
            facts
                .is_file()
                .then(|| Some((facts.modified().ok()?, facts.len(), entry.path())))?
        })
        .collect();
    kept.sort();
    let mut held: u64 = kept.iter().map(|(_, size, _)| size).sum();
    for (_, size, path) in kept.iter().take(kept.len().saturating_sub(1)) {
        if held <= most {
            break;
        }
        if fs::remove_file(path).is_ok() {
            held -= size;
        }
    }
}

/// Says the picture made, and its width and height, if one was.
fn picture_of(said: &mut Map<String, Value>, picture: Option<PathBuf>) {
    let Some(picture) = picture else {
        return;
    };
    let Ok(size) = imagesize::size(&picture) else {
        return;
    };
    said.insert("picture".into(), json!(picture));
    said.insert("across".into(), json!(size.width));
    said.insert("down".into(), json!(size.height));
}

/// Runs a program, quietly, saying whether it did what it was asked.
fn ran(command: &mut Command) -> Option<String> {
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn pdf_pages(path: &Path) -> Option<u64> {
    let said = ran(Command::new("pdfinfo").arg("--").arg(path))?;
    said.lines()
        .find_map(|line| line.strip_prefix("Pages:"))
        .and_then(|pages| pages.trim().parse().ok())
}

/// A PDF's first page, drawn as wide as the drawer can show it.
fn first_page(from: &Path, to: &Path) -> bool {
    // pdftoppm names what it writes itself, adding .png to the name given.
    let Some(stem) = to.to_str().and_then(|to| to.strip_suffix(".png")) else {
        return false;
    };
    ran(Command::new("pdftoppm")
        .args(["-png", "-singlefile", "-f", "1", "-l", "1"])
        .args(["-scale-to-x", &MADE_WIDE.to_string(), "-scale-to-y", "-1"])
        .arg(from)
        .arg(stem))
    .is_some()
}

fn video_seconds(path: &Path) -> Option<f64> {
    let said = ran(Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path))?;
    said.trim()
        .parse()
        .ok()
        .filter(|seconds: &f64| seconds.is_finite())
}

/// A video's frame `at` seconds in, no wider than the drawer can show it
/// and never widened.
fn frame_at(from: &Path, to: &Path, at: f64) -> bool {
    ran(Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-ss", &format!("{at:.3}"), "-i"])
        .arg(from)
        .args(["-frames:v", "1", "-vf"])
        .arg(format!("scale='min({MADE_WIDE},iw)':-2"))
        .args(["-f", "image2", "-c:v", "png"])
        .arg(to))
    .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = env::temp_dir().join(format!("file-preview-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn text_shows_its_first_whole_lines() {
        let lines = lines_of(b"# Plan\n\n\tone\x07 thing  \nlast").unwrap();
        assert_eq!(lines, ["# Plan", "", "    one thing", "last"]);
    }

    #[test]
    fn text_cut_short_leaves_out_the_line_it_was_cut_in() {
        let mut read = "a line\n".repeat(600).into_bytes();
        read.truncate(READ + 1);
        let lines = lines_of(&read).unwrap();
        assert_eq!(lines.len(), LINES);
        assert!(lines.iter().all(|line| line == "a line"));
    }

    #[test]
    fn a_character_cut_by_the_read_is_still_text() {
        let mut read = vec![b'a'; READ - 1];
        read.extend("é\n".as_bytes());
        assert!(lines_of(&read).is_some());
    }

    #[test]
    fn text_counts_all_its_lines_past_what_it_shows() {
        let folder = folder("counted");
        let long = folder.join("long.md");
        fs::write(&long, "a line\n".repeat(20_000)).unwrap();
        let said = shown(&long).unwrap();
        assert_eq!(said["count"], 20_000);
        assert_eq!(said["lines"].as_array().unwrap().len(), LINES);
        let unended = folder.join("unended.txt");
        fs::write(&unended, "one\ntwo").unwrap();
        assert_eq!(shown(&unended).unwrap()["count"], 2);
        let empty = folder.join("empty.txt");
        fs::write(&empty, "").unwrap();
        assert_eq!(shown(&empty).unwrap()["count"], 0);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn text_past_the_bound_counts_what_was_read_and_says_theres_more() {
        let folder = folder("bounded");
        let long = folder.join("long.log");
        fs::write(&long, "a line\n".repeat(100)).unwrap();
        // 100 bytes hold 14 whole lines of 7, and a part of the 15th.
        assert_eq!(lines_in(&long, 100), Some((14, false)));
        assert_eq!(lines_in(&long, 700), Some((100, true)));
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn serving_answers_each_line_with_one_and_a_bad_one_with_an_error() {
        let folder = folder("served");
        let file = folder.join("notes.md");
        fs::write(&file, "one\ntwo\n").unwrap();
        let asked = format!(
            "show {}\nitems {}\nshow {}/gone\nlook at it\n",
            file.display(),
            folder.display(),
            folder.display()
        );
        let mut out = Vec::new();
        serve(asked.as_bytes(), &mut out).expect("serving answers every line, even a wrong one");
        let said: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(said.len(), 4);
        assert_eq!(said[0]["count"], 2);
        assert_eq!(said[1], json!({}));
        assert!(said[2]["error"].as_str().unwrap().contains("gone"));
        assert_eq!(said[3]["error"], USAGE);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn the_kept_pictures_lose_the_longest_unshown_first_past_their_cap() {
        let folder = folder("pruned");
        let at = |seconds| UNIX_EPOCH + std::time::Duration::from_secs(seconds);
        for (name, shown) in [
            ("old.png", 1),
            ("older.png", 0),
            ("new.png", 3),
            ("mid.png", 2),
        ] {
            let path = folder.join(name);
            fs::write(&path, [0; 10]).unwrap();
            fs::File::options()
                .append(true)
                .open(&path)
                .unwrap()
                .set_modified(at(shown))
                .unwrap();
        }
        fs::write(folder.join(".being-made.png"), [0; 10]).unwrap();
        pruned(&folder, 20);
        let mut left: Vec<String> = fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, [".being-made.png", "mid.png", "new.png"]);
        // However small the cap, the one just made, the newest, is kept.
        pruned(&folder, 0);
        assert!(folder.join("new.png").is_file());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_fifo_says_only_its_facts_and_is_never_opened() {
        // Opened, it would wait for a writer that never comes, so it's asked on a thread of
        // its own and given SAID_WITHIN, and the case fails rather than hangs.
        const SAID_WITHIN: std::time::Duration = std::time::Duration::from_secs(2);
        let folder = folder("fifo");
        let fifo = folder.join("pipe.txt");
        let made = Command::new("mkfifo").arg(&fifo).status();
        assert!(
            made.is_ok_and(|made| made.success()),
            "mkfifo made {}",
            fifo.display()
        );
        let (tell, told) = std::sync::mpsc::channel();
        let asked = fifo.clone();
        std::thread::spawn(move || tell.send(shown(&asked).ok()));
        let said = told
            .recv_timeout(SAID_WITHIN)
            .expect("a FIFO is said without being opened")
            .expect("a FIFO is said");
        let keys: Vec<&String> = said.as_object().expect("its facts").keys().collect();
        assert_eq!(keys, ["changed", "size"], "only its facts: {said}");
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn what_holds_a_nul_or_isnt_utf8_is_not_text() {
        assert_eq!(lines_of(b"PK\x03\x04\0\0"), None);
        assert_eq!(lines_of(b"caf\xe9 au lait"), None);
    }

    #[test]
    fn a_folder_counts_what_it_shows_and_names_its_folders_first() {
        let folder = folder("listed");
        for name in ["b.md", "A.txt", ".hidden"] {
            fs::write(folder.join(name), "").unwrap();
        }
        fs::create_dir(folder.join("zeta")).unwrap();
        let (count, names) = listed(&folder);
        assert_eq!(count, 3);
        assert_eq!(names, ["zeta/", "A.txt", "b.md"]);
        let counted = items_inside(folder.parent().unwrap());
        assert_eq!(counted[folder.to_str().unwrap()], json!(3));
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_picture_says_its_size_and_is_its_own_picture() {
        let folder = folder("picture");
        let path = folder.join("Wide.PNG");
        // The smallest PNG: its signature and a header 3 wide and 2 down.
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend([0, 0, 0, 3, 0, 0, 0, 2, 8, 2, 0, 0, 0, 0, 0, 0, 0]);
        fs::write(&path, png).unwrap();
        let said = shown(&path).unwrap();
        assert_eq!(said["kind"], "PNG");
        assert_eq!(
            (said["across"].clone(), said["down"].clone()),
            (json!(3), json!(2))
        );
        assert_eq!(said["picture"], json!(path));
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn anything_else_says_only_its_facts() {
        let folder = folder("other");
        let path = folder.join("archive.zip");
        fs::write(&path, b"PK\x03\x04\0\0\0").unwrap();
        let said = shown(&path).unwrap();
        let keys: Vec<&String> = said.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["changed", "kind", "size"]);
        assert_eq!(said["size"], 7);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_version_is_named_by_its_path_size_and_change() {
        let path = Path::new("/home/a/plan.pdf");
        let one = Version {
            path,
            size: 10,
            changed: 5,
        };
        let again = Version {
            path,
            size: 10,
            changed: 5,
        };
        let changed = Version {
            path,
            size: 10,
            changed: 6,
        };
        assert_eq!(one.name(), again.name());
        assert_ne!(one.name(), changed.name());
    }

    #[test]
    fn hidden_files_have_no_ending() {
        assert_eq!(ending_of(Path::new("/a/.bashrc")), "");
        assert_eq!(ending_of(Path::new("/a/Plan.MD")), "md");
        assert_eq!(ending_of(Path::new("/a/notes")), "");
    }
}
