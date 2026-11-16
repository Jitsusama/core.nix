//! clipboard-history's words as the terminal kit takes them: its help,
//! and the settings its unit starts it with. A start asked wrongly ends
//! 64 before anything of the clipboard is heard.

use std::path::PathBuf;

use terminal_kit::{Exit, Failure, Help, flags};

/// The flags it takes, for naming the nearest when one is mistyped.
const FLAGS: [&str; 8] = [
    "--socket",
    "--vocabulary",
    "--keep",
    "--largest",
    "--pictures",
    "--largest-picture",
    "--room",
    "--emoji",
];

/// What clipboard-history says it is, does and ends with, as `--help`
/// shows it.
pub fn help() -> Help {
    Help::new(
        "clipboard-history",
        "hears the clipboard, and keeps its copies for the launcher",
    )
    .example(
        "clipboard-history --socket c.sock --vocabulary w.json",
        "keeps 200 copies, and no pictures",
    )
    .example(
        "clipboard-history --socket c.sock --vocabulary w.json \\\n  --keep 0",
        "keeps none, saying only what the clipboard holds",
    )
    .option("--socket PATH", "where the shell connects")
    .option("--vocabulary FILE", "the launcher's words.json")
    .option("--keep N", "how many copies it keeps; 200")
    .option("--largest BYTES", "the largest text kept; 1 MiB")
    .option("--pictures DIR", "where pictures are kept; none without")
    .option("--largest-picture BYTES", "the largest picture; 64 MiB")
    .option("--room BYTES", "the room its pictures take; 256 MiB")
    .option("--emoji LIST", "the launcher's emoji, served as rows")
    .exit(Exit::Failed, "it lost Wayland, or couldn't listen")
    .exit(Exit::Config, "its words or emoji couldn't be read")
}

/// What the unit starts it with.
#[derive(Debug, PartialEq)]
pub struct Settings {
    pub socket: PathBuf,
    pub vocabulary: PathBuf,
    pub keep: usize,
    pub largest: usize,
    pub pictures: Option<PathBuf>,
    pub largest_picture: usize,
    pub room: usize,
    pub emoji: Option<PathBuf>,
}

/// The settings its words give, each left out at its default.
///
/// # Errors
///
/// Asked wrongly, 64: a flag it doesn't take, named with the nearest it
/// does, one without its value or with a value it can't read, or no
/// socket or words. A value comes as the next word or joined with `=`.
pub fn settings(args: &[String]) -> Result<Settings, Failure> {
    let wrongly =
        |what: String| Failure::new(Exit::Usage, what).try_this("clipboard-history --help");
    let (mut socket, mut vocabulary) = (None, None);
    let mut keep = 200;
    let mut largest = 1 << 20;
    let mut pictures = None;
    let mut largest_picture = 64 << 20;
    let mut room = 256 << 20;
    let mut emoji = None;
    let mut arguments = args.iter();
    while let Some(word) = arguments.next() {
        let (argument, joined) = match word.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value)),
            _ => (word.as_str(), None),
        };
        // A value as the next word mustn't be the next flag, which
        // `--socket --keep 5` would take as its socket; one joined with
        // `=` is meant as written, but never empty.
        let mut value = || {
            joined
                .or_else(|| {
                    arguments
                        .next()
                        .map(String::as_str)
                        .filter(|value| !value.starts_with("--"))
                })
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| wrongly(format!("{argument} needs a value")))
        };
        let count = |value: String, unit: &str| {
            value
                .parse::<usize>()
                .map_err(|_| wrongly(format!("{argument} takes {unit}, not {value}")))
        };
        match argument {
            "--socket" => socket = Some(PathBuf::from(value()?)),
            "--vocabulary" => vocabulary = Some(PathBuf::from(value()?)),
            "--keep" => keep = count(value()?, "a count")?,
            "--largest" => largest = count(value()?, "bytes")?,
            "--pictures" => pictures = Some(PathBuf::from(value()?)),
            "--largest-picture" => largest_picture = count(value()?, "bytes")?,
            "--room" => room = count(value()?, "bytes")?,
            "--emoji" => emoji = Some(PathBuf::from(value()?)),
            other => {
                return Err(
                    flags::unknown("option", other, &FLAGS).try_this("clipboard-history --help")
                );
            }
        }
    }
    let socket = socket.ok_or_else(|| wrongly("--socket names where the shell connects".into()))?;
    let vocabulary =
        vocabulary.ok_or_else(|| wrongly("--vocabulary names the launcher's words".into()))?;
    if largest_picture > room {
        return Err(wrongly(
            "--largest-picture can't be more than --room".to_owned(),
        ));
    }
    Ok(Settings {
        socket,
        vocabulary,
        keep,
        largest,
        pictures,
        largest_picture,
        room,
        emoji,
    })
}

#[cfg(test)]
mod tests {
    use super::{Settings, help, settings};
    use std::path::{Path, PathBuf};
    use terminal_kit::{Environment, Exit};

    /// The exit and the conversation the kit ends with given `line`, a
    /// unit's words split as systemd splits them, and the settings the
    /// body read from them.
    fn read(line: &str) -> (Exit, String, Option<Settings>) {
        let mut read = None;
        let ending = terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[]),
            line.split_whitespace().map(str::to_owned).collect(),
            |context| {
                read = Some(settings(&context.args)?);
                Ok(())
            },
        );
        (ending.exit, ending.stderr, read)
    }

    #[test]
    fn each_of_the_units_two_lines_reads_as_it_did() {
        let (exit, said, kept) = read(
            "--socket /run/user/1000/clipboard-history.sock --vocabulary /nix/w.json \
             --keep 50 --pictures /run/user/1000/clipboard-history --emoji /nix/emoji.json",
        );
        assert_eq!(
            exit,
            Exit::Done,
            "the keeping unit's words were refused: {said}"
        );
        assert_eq!(
            kept,
            Some(Settings {
                socket: PathBuf::from("/run/user/1000/clipboard-history.sock"),
                vocabulary: PathBuf::from("/nix/w.json"),
                keep: 50,
                largest: 1 << 20,
                pictures: Some(PathBuf::from("/run/user/1000/clipboard-history")),
                largest_picture: 64 << 20,
                room: 256 << 20,
                emoji: Some(PathBuf::from("/nix/emoji.json")),
            }),
            "the keeping unit's settings weren't read as written"
        );
        let (exit, said, none) = read("--socket c.sock --vocabulary w.json --keep 0");
        assert_eq!(
            exit,
            Exit::Done,
            "the unit keeping none was refused: {said}"
        );
        assert!(
            none.is_some_and(|none| none.keep == 0 && none.pictures.is_none()),
            "--keep 0 kept something"
        );
    }

    #[test]
    fn a_value_may_be_joined_to_its_flag_with_an_equals() {
        let (exit, said, joined) = read("--socket=c.sock --vocabulary=w.json --keep=5");

        assert_eq!(exit, Exit::Done, "the joined values were refused: {said}");
        assert!(
            joined.is_some_and(|joined| joined.keep == 5
                && joined.socket == Path::new("c.sock")
                && joined.vocabulary == Path::new("w.json")),
            "the joined values weren't read as written"
        );
    }

    #[test]
    fn a_start_it_cant_read_ends_64_before_hearing_anything() {
        for (line, why) in [
            ("--vocabulary w.json", "--socket names"),
            ("--socket c.sock", "--vocabulary names"),
            (
                "--socket c.sock --vocabulary w.json --keep lots",
                "takes a count",
            ),
            (
                "--socket c.sock --vocabulary w.json --room",
                "--room needs a value",
            ),
            ("--socket --keep 5", "--socket needs a value"),
            ("--socket= --vocabulary w.json", "--socket needs a value"),
            (
                "--socket c.sock --vocabulary w.json --colour",
                "no option called --colour",
            ),
            (
                "--socket c.sock --vocabulary w.json --kep 5",
                "did you mean --keep?",
            ),
            (
                "--socket c.sock --vocabulary w.json --largest-picture 9 --room 8",
                "more than --room",
            ),
        ] {
            let (exit, said, read) = read(line);
            assert_eq!(exit, Exit::Usage, "{line} wasn't refused with 64: {said}");
            assert!(read.is_none(), "{line} was read as {read:?}");
            assert!(
                said.contains(why) && said.contains("clipboard-history --help"),
                "{line}'s refusal didn't say {why} and name the help: {said}"
            );
        }
    }
}
