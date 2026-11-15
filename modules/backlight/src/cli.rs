//! What backlight is asked on its command line: its help, and the words its
//! two units start it with, read into what it does before anything is
//! touched, so a unit written wrongly says so at once.

use std::fs;
use std::path::{Path, PathBuf};

use terminal_kit::{Exit, Failure, Help};

use crate::curve::Curve;
use crate::evening::Evening;
use crate::warmth::WHITE;

/// The night light's white unless the command line gives another: GNOME's
/// Night Light's, gentle enough that code's colours still read (VKCF3Z,
/// 2026-11-08).
pub const NIGHT_WHITE: u16 = 4000;

/// The flag the root unit starts it with, to keep the open level.
const KEEPS: &str = "--keeps-the-open-level";

/// What `backlight --help` says.
pub fn help() -> Help {
    Help::new(
        "backlight",
        "sets the screen's brightness for the light in the room, and learns from what's picked",
    )
    .example("backlight standard 4000 21:30-05:00", "as its unit runs it")
    .example(
        "backlight standard",
        "the night light at 4000 K, and no evening",
    )
    .example(
        "backlight --keeps-the-open-level saves-while-open",
        "as root, keeping the panel's saved level",
    )
    .argument(
        "standard",
        "the curve learning starts from: a lux and a share of the brightest on each line",
    )
    .argument(
        "[night-white]",
        "the night light's white, 1000 to 6500 K; 4000 when left out",
    )
    .argument(
        "[evening]",
        "when the screen dims, as 21:30-05:00, or off, which it is when left out",
    )
    .option(
        "--keeps-the-open-level SAVER",
        "keep the panel's saved level current while the lid is open, through SAVER",
    )
    .exit(Exit::Failed, "the screen, a bus or the sensor went away")
    .exit(Exit::Config, "its standard curve couldn't be read")
}

/// What a start asks it to do.
#[derive(Debug, PartialEq)]
pub enum Asked {
    /// Keep the panel's saved level current through the saver, as root.
    Keep { saver: PathBuf },
    /// Set the screen for the light, from the standard curve in its file.
    Set {
        standard: PathBuf,
        night_white: u16,
        evening: Option<Evening>,
    },
}

/// What the words a start is given ask for.
///
/// # Errors
///
/// Asked wrongly, 64: no standard curve, a word too many, a flag it doesn't
/// take, or a night white or evening it can't read.
pub fn asked(args: &[String]) -> Result<Asked, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("backlight --help");
    if let [first, rest @ ..] = args
        && first == KEEPS
    {
        return match rest {
            [saver] if !saver.starts_with('-') => Ok(Asked::Keep {
                saver: PathBuf::from(saver),
            }),
            [] => Err(wrongly(format!("{KEEPS} needs the saver to run"))),
            [saver] => Err(wrongly(format!("{KEEPS} needs a saver, not {saver}"))),
            [_, more, ..] => Err(wrongly(format!(
                "{KEEPS} takes one saver, and {more} is another"
            ))),
        };
    }
    if let Some(flag) = args.iter().find(|arg| arg.starts_with('-')) {
        return Err(if flag == KEEPS {
            wrongly(format!("{KEEPS} is asked alone, with its saver"))
        } else {
            terminal_kit::flags::unknown("flag", flag, &[KEEPS]).try_this("backlight --help")
        });
    }
    let (standard, white, evening) = match args {
        [] => return Err(wrongly("backlight takes its standard curve".into())),
        [standard] => (standard, None, None),
        [standard, white] => (standard, Some(white), None),
        [standard, white, evening] => (standard, Some(white), Some(evening)),
        [_, _, _, more, ..] => {
            return Err(wrongly(format!(
                "backlight takes three words at most, and {more} is a fourth"
            )));
        }
    };
    let night_white = match white {
        None => NIGHT_WHITE,
        Some(said) => said
            .parse::<u16>()
            .ok()
            .filter(|kelvin| (1000..=WHITE).contains(kelvin))
            .ok_or_else(|| {
                wrongly(format!(
                    "{said} is no night white between 1000 and {WHITE} K"
                ))
            })?,
    };
    let evening = match evening.map(String::as_str) {
        None | Some("off") => None,
        Some(said) => Some(
            Evening::read(said)
                .ok_or_else(|| wrongly(format!("{said} is no evening, as 21:30-05:00")))?,
        ),
    };
    Ok(Asked::Set {
        standard: PathBuf::from(standard),
        night_white,
        evening,
    })
}

/// The standard curve kept in `path`.
///
/// # Errors
///
/// 78 when the file can't be read or holds no curve.
pub fn standard_curve(path: &Path) -> Result<Curve, Failure> {
    let unread = || Failure::new(Exit::Config, "backlight can't read its standard curve");
    let text = fs::read_to_string(path)
        .map_err(|error| unread().because(format!("{}: {error}", path.display())))?;
    Curve::read(&text).ok_or_else(|| unread().because(format!("{} holds no curve", path.display())))
}

#[cfg(test)]
mod tests {
    use super::{Asked, NIGHT_WHITE, asked, help, standard_curve};
    use crate::evening::Evening;
    use std::fs;
    use std::path::PathBuf;
    use terminal_kit::{Environment, Exit};

    fn words(said: &[&str]) -> Vec<String> {
        said.iter().map(|word| (*word).to_owned()).collect()
    }

    /// How the kit ends backlight started with `said`, its body reading
    /// only what the words ask.
    fn started(said: &[&str]) -> (Exit, String) {
        let ending = terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[]),
            words(said),
            |context| asked(&context.args).map(drop),
        );
        (ending.exit, ending.stderr)
    }

    #[test]
    fn its_units_words_are_read_as_they_were() {
        let set = |night_white, evening: Option<&str>| Asked::Set {
            standard: PathBuf::from("/nix/store/x-backlight-standard"),
            night_white,
            evening: evening.and_then(Evening::read),
        };
        for (said, meant) in [
            (
                &["/nix/store/x-backlight-standard", "3400", "21:30-05:00"][..],
                set(3400, Some("21:30-05:00")),
            ),
            (
                &["/nix/store/x-backlight-standard", "3400", "off"],
                set(3400, None),
            ),
            (&["/nix/store/x-backlight-standard"], set(NIGHT_WHITE, None)),
            (
                &[
                    "--keeps-the-open-level",
                    "/nix/store/x/bin/saves-while-open",
                ],
                Asked::Keep {
                    saver: PathBuf::from("/nix/store/x/bin/saves-while-open"),
                },
            ),
        ] {
            assert_eq!(
                asked(&words(said)).ok(),
                Some(meant),
                "{said:?} wasn't read as its unit means it"
            );
            assert_eq!(started(said).0, Exit::Done, "the kit refused {said:?}");
        }
    }

    #[test]
    fn a_start_asked_wrongly_ends_64_naming_its_help() {
        for said in [
            &[][..],
            &["standard", "999"],
            &["standard", "6501"],
            &["standard", "warm"],
            &["standard", "4000", "late"],
            &["standard", "4000", "off", "more"],
            &["--keeps-the-open-level"],
            &["--keeps-the-open-level", "saver", "more"],
            &["standard", "--keeps-the-open-level", "saver"],
            &["--keep", "saver"],
        ] {
            let (exit, refused) = started(said);
            assert_eq!(
                exit,
                Exit::Usage,
                "{said:?} wasn't refused as asked wrongly: {refused}"
            );
            assert!(
                refused.contains("backlight --help"),
                "{said:?}'s refusal didn't name its help: {refused}"
            );
        }
    }

    #[test]
    fn a_flag_written_nearly_right_is_named() {
        let (_, refused) = started(&["--keeps-the-open-levl", "saver"]);
        assert!(
            refused.contains("did you mean --keeps-the-open-level?"),
            "the nearest flag wasn't named: {refused}"
        );
    }

    #[test]
    fn a_standard_curve_it_cant_read_ends_78() {
        let folder = std::env::temp_dir().join(format!("backlight-cli-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("the test's folder can't be made");
        let empty = folder.join("empty");
        fs::write(&empty, "no curve here\n").expect("the empty curve can't be written");
        let kept = folder.join("kept");
        fs::write(&kept, "0 0.05\n1000 0.6\n").expect("the curve can't be written");
        for path in [folder.join("missing"), empty] {
            let failure = standard_curve(&path).expect_err("an unreadable curve was read");
            assert_eq!(
                failure.exit,
                Exit::Config,
                "{} didn't end as a configuration that can't be read",
                path.display()
            );
        }
        assert!(
            standard_curve(&kept).is_ok(),
            "a curve kept as its unit writes it wasn't read"
        );
        fs::remove_dir_all(&folder).expect("the test's folder can't be removed");
    }
}
