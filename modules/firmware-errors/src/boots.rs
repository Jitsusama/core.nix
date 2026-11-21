//! The boots whose tables the keeping unit kept, each in a folder named
//! for its boot id, and what each says of how the boot before it ended.

use crate::crash::{self, Crash};
use crate::phat::{self, FIRST_UNASKED, Reset, SAID_REASONS, SOFTWARE};
use std::fs;
use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// A boot id's length, as the kernel gives it without its hyphens.
const BOOT_ID: usize = 32;
/// The boot error region's copy, as the keeping unit names it (../keep.sh).
pub const BERT_REGION: &str = "bert-region";
/// The boot error table's copy.
pub const BERT_TABLE: &str = "bert-table";
/// The platform health table's copy.
pub const PHAT_TABLE: &str = "phat-table";
/// The tables the keeping unit copies, by the names it gives them.
pub const TABLES: [&str; 3] = [BERT_REGION, BERT_TABLE, PHAT_TABLE];
/// The file the keeping unit writes the boot's start in, in seconds since
/// 1970, as /proc/stat gives it.
pub const BEGAN: &str = "began";

/// One kept boot, and what its tables say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// Its boot id, without hyphens, as journalctl names boots.
    pub boot: String,
    /// When it began, in seconds since 1970, as the keeping wrote it; none
    /// in a folder kept before the keeping wrote it.
    pub began: Option<i64>,
    /// When its tables were first written, in seconds since 1970, which
    /// orders a folder that doesn't say when its boot began. It isn't when
    /// the boot began: a boot's first run of the keeping may come at a
    /// switch, and a folder kept before the keeping left whole tables alone
    /// had them written again at each switch.
    pub written: Option<i64>,
    /// Whether its health table was kept and read, whether or not it names a
    /// reset; one kept that can't be read is in `unread` instead.
    pub health: bool,
    /// The health table's reset reason, where it gave one.
    pub reset: Option<Reset>,
    /// What the boot error region holds, where it gave one.
    pub crash: Option<Crash>,
    /// Each file in its folder that couldn't be read, by name, and why;
    /// the rest of it is still read.
    pub unread: Vec<(&'static str, String)>,
}

/// How bad an end is, for the mark its row wears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    /// The firmware or the watchdog reset the machine for an error.
    Reset,
    /// It ended some other way than asked: a fault, lost power, heat, or
    /// an error kept that can't be read.
    Unasked,
    /// Nothing says it ended badly.
    Plain,
}

/// How the boot before a kept one ended, as its tables say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum End {
    /// A Crash Log names a core's timeout.
    CoreTimeout,
    /// The firmware kept a Crash Log naming something else, or nothing.
    CrashLog,
    /// The watchdog says it reset the machine.
    Watchdog,
    /// Intel's global reset cause names why the controller reset it.
    GlobalReset(Vec<String>),
    /// The health table's reason, by its name and words, and its weight.
    Reason(&'static str, &'static str, Weight),
    /// The region holds error entries, none of them a Crash Log.
    OtherError,
    /// The region holds what the decoder can't read.
    Unreadable,
    /// A table kept in its folder couldn't be read, or the folder itself.
    Unread,
    /// Nothing kept says why.
    Unknown,
}

impl End {
    /// Its name, as a program matches it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::CoreTimeout => "core-timeout",
            Self::CrashLog => "crash-log",
            Self::Watchdog => "watchdog-reset",
            Self::GlobalReset(_) => "global-reset",
            Self::Reason(name, ..) => name,
            Self::OtherError => "other-error",
            Self::Unreadable => "unreadable",
            Self::Unread => "unread",
            Self::Unknown => "unknown",
        }
    }

    /// It, in words.
    pub fn words(&self) -> String {
        match self {
            Self::CoreTimeout => "a core timed out, and the firmware reset it".to_owned(),
            Self::CrashLog => "the firmware reset it, and kept a Crash Log".to_owned(),
            Self::Watchdog => "the watchdog reset it".to_owned(),
            Self::GlobalReset(causes) => format!("the firmware reset it ({})", causes.join(", ")),
            Self::Reason(_, words, _) => (*words).to_owned(),
            Self::OtherError => "the firmware kept an error that isn't a Crash Log".to_owned(),
            Self::Unreadable => "the firmware kept an error that can't be read".to_owned(),
            Self::Unread => "its kept tables couldn't be read".to_owned(),
            Self::Unknown => "nothing kept says why".to_owned(),
        }
    }

    /// How bad it is.
    pub fn weight(&self) -> Weight {
        match self {
            Self::CoreTimeout | Self::CrashLog | Self::Watchdog | Self::GlobalReset(_) => {
                Weight::Reset
            }
            Self::Reason(.., weight) => *weight,
            Self::OtherError | Self::Unreadable | Self::Unread => Weight::Unasked,
            Self::Unknown => Weight::Plain,
        }
    }
}

impl Kept {
    /// How the boot before this one ended. `named` says whether this
    /// machine's processor is the family whose layout names Intel's reset
    /// cause; `watchdog_reset` whether the watchdog says it reset the
    /// machine, which only the boot running now can know.
    ///
    /// A Crash Log speaks first, being the firmware's own account, then the
    /// watchdog. Heat, lost power and the power button speak before the
    /// rest of Intel's cause, which says less of them, whether the table's
    /// reason or the cause's bits say them. Intel's cause speaks only where
    /// the table's reason doesn't say the end was asked for, so a cause the
    /// firmware left set from an earlier reset can't mark a plain reboot,
    /// and only on the platform whose layout names its bits. A reason that
    /// says the end was asked for speaks last, after an error kept that
    /// isn't a Crash Log, one that can't be read, and a file that can't be.
    pub fn verdict(&self, named: bool, watchdog_reset: bool) -> End {
        if let Some(Crash::Tags(tags)) = &self.crash {
            if tags.iter().any(|tag| tag.starts_with("CORE_TIMEOUT")) {
                return End::CoreTimeout;
            }
            return End::CrashLog;
        }
        if watchdog_reset {
            return End::Watchdog;
        }
        let plain = self.reset.as_ref().and_then(|reset| {
            // A reason not said is asked for where software was the
            // reset's source, as a reboot the system asks for is.
            let unasked = (reset.reason == 0 && reset.source & SOFTWARE == 0)
                || reset.reason >= FIRST_UNASKED;
            if SAID_REASONS.contains(&reset.reason) {
                return reason(reset.reason);
            }
            // The cause is weighed only where the table doesn't say the
            // reboot was asked for, since nothing yet shows the firmware
            // clears it, and only on the platform whose layout names it,
            // since elsewhere a bit may mean anything.
            if unasked {
                if let Some(said) = reset.said_by_cause(named) {
                    return reason(said);
                }
                if named {
                    let causes = reset.global_causes(named);
                    if !causes.is_empty() {
                        return Some(End::GlobalReset(causes));
                    }
                }
            }
            reason(reset.reason)
        });
        if let Some(end) = plain.as_ref().filter(|end| end.weight() != Weight::Plain) {
            return end.clone();
        }
        // An error kept, or a file that can't be read, says more than a
        // reason that says the reboot was asked for.
        match &self.crash {
            Some(Crash::Other) => End::OtherError,
            Some(Crash::Unreadable(_)) => End::Unreadable,
            _ if !self.unread.is_empty() => End::Unread,
            _ => plain.unwrap_or(End::Unknown),
        }
    }
}

/// A reason the specification names, weighed by whether it was asked for.
fn reason(code: u8) -> Option<End> {
    let weight = if code >= FIRST_UNASKED {
        Weight::Unasked
    } else {
        Weight::Plain
    };
    Some(End::Reason(
        phat::reason_name(code)?,
        phat::reason_words(code)?,
        weight,
    ))
}

/// When a file was last written, in seconds since 1970.
fn written(path: &Path) -> Option<i64> {
    let since = fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?;
    i64::try_from(since.as_secs()).ok()
}

/// One boot's folder, read. A file that isn't there is left out; a table
/// that can't be read is said in `unread`, and the rest still read. When
/// the boot began says nothing of how the boot before it ended, so where
/// it can't be read the boot is only left without it.
fn kept_at(folder: &Path, boot: String) -> Kept {
    let mut unread = Vec::new();
    let mut read = |name: &'static str| match fs::read(folder.join(name)) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            unread.push((name, error.to_string()));
            None
        }
    };
    let began = fs::read(folder.join(BEGAN))
        .ok()
        .and_then(|text| String::from_utf8(text).ok()?.trim().parse().ok());
    let written = TABLES
        .iter()
        .filter_map(|name| written(&folder.join(name)))
        .min();
    let health = read(PHAT_TABLE);
    let reset = health.as_deref().and_then(phat::reset_of);
    let crash = read(BERT_REGION).map(|region| crash::crash_of(&region));
    Kept {
        boot,
        began,
        written,
        health: health.is_some(),
        reset,
        crash,
        unread,
    }
}

/// Every boot kept in the state folder, the newest first; none where the
/// folder isn't there. A folder holding none of the tables is no kept
/// boot: the keeping writes only beside a table, and the ageing may leave
/// a folder whose tables it took. One that can't be searched may hold
/// them, so it's listed, its tables said unread, rather than left out.
pub fn kept_in(state: &Path) -> io::Result<Vec<Kept>> {
    let entries = match fs::read_dir(state) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut kept = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let folder = entry.path();
        let holds_a_table = TABLES.iter().any(|table| {
            !matches!(
                fs::symlink_metadata(folder.join(table)),
                Err(error) if error.kind() == io::ErrorKind::NotFound
            )
        });
        if name.len() == BOOT_ID
            && name.bytes().all(|byte| byte.is_ascii_hexdigit())
            && holds_a_table
        {
            kept.push(kept_at(&folder, name));
        }
    }
    kept.sort_by(|a, b| {
        let order = |boot: &Kept| boot.began.or(boot.written);
        order(b).cmp(&order(a)).then_with(|| a.boot.cmp(&b.boot))
    });
    Ok(kept)
}

/// How many of a boot id's digits name a boot, at the fewest, so neither
/// journalctl's index of a boot, as -1 or 0, nor a mistyped flag is taken
/// for the start of one.
pub const SHORTEST_ID: usize = 4;

/// A boot id as any tool writes it, hyphens or not, in either case, as
/// the folders name it; none where it holds anything but hex digits and
/// hyphens, starts with a hyphen, or has fewer than `SHORTEST_ID` digits.
pub fn plain_id(id: &str) -> Option<String> {
    let plain = id
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>()
        .to_ascii_lowercase();
    (!id.starts_with('-')
        && plain.len() >= SHORTEST_ID
        && plain.bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some(plain)
}

/// The kept boots whose id starts with the plain id given.
pub fn matching<'a>(kept: &'a [Kept], plain: &str) -> Vec<&'a Kept> {
    kept.iter()
        .filter(|boot| boot.boot.starts_with(plain))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn reset(source: u8, reason: u8, cause: u32) -> Reset {
        Reset {
            source,
            reason,
            intel: Some([cause, 0, 0, 0]),
        }
    }

    fn kept(reset: Option<Reset>, crash: Option<Crash>) -> Kept {
        Kept {
            boot: "6bccd85d80714583afd805c687f0cdb6".to_owned(),
            began: None,
            written: None,
            health: reset.is_some(),
            reset,
            crash,
            unread: Vec::new(),
        }
    }

    #[test]
    fn a_core_timeout_is_said_first() {
        let tags = vec![
            "CORE_TIMEOUT.MULTIPLE_STUCK_TRANSACTIONS".to_owned(),
            "RESET_CAUSE.GLOBAL_RESET.PMC_FW".to_owned(),
        ];
        let boot = kept(Some(reset(1, 0, 0x400)), Some(Crash::Tags(tags)));
        let end = boot.verdict(true, true);
        assert_eq!((end.name(), end.weight()), ("core-timeout", Weight::Reset));
        assert_eq!(end.words(), "a core timed out, and the firmware reset it");
    }

    #[test]
    fn a_crash_log_the_analyzer_names_nothing_in_is_still_a_crash_log() {
        let boot = kept(Some(reset(1, 0x24, 0)), Some(Crash::Tags(Vec::new())));
        assert_eq!(boot.verdict(true, false), End::CrashLog);
    }

    #[test]
    fn a_global_reset_with_no_crash_log_is_named_by_its_cause_on_panther_lake_alone() {
        let boot = kept(Some(reset(1, 0, 0x40)), Some(Crash::None));
        let end = boot.verdict(true, false);
        assert_eq!(end.words(), "the firmware reset it (MEGBL)");
        assert_eq!(end.weight(), Weight::Reset);
        // Elsewhere the bit may mean anything, so it says nothing.
        let elsewhere = boot.verdict(false, false);
        assert_eq!(
            (elsewhere.clone(), elsewhere.weight()),
            (End::Unknown, Weight::Plain)
        );
    }

    #[test]
    fn heat_power_and_the_button_speak_before_intels_cause() {
        for (reason, name) in [(0x23, "heat"), (0x24, "power-loss"), (0x25, "power-button")] {
            let boot = kept(Some(reset(0b10, reason, 0x2)), None);
            let end = boot.verdict(true, false);
            assert_eq!(
                (end.name(), end.weight()),
                (name, Weight::Unasked),
                "reason {reason:#x} with the power button's override set"
            );
        }
        // Where the table says nothing, the cause's own bits say them.
        for (cause, name) in [
            (1 << 3, "heat"),
            (1 << 27, "heat"),
            (1 << 11, "power-loss"),
            ((1 << 12) | (1 << 10), "power-loss"),
            (1 << 1, "power-button"),
            (1 << 4, "power-button"),
            ((1 << 1) | (1 << 16), "heat"),
        ] {
            let boot = kept(Some(reset(1, 0, cause)), Some(Crash::None));
            let end = boot.verdict(true, false);
            assert_eq!(
                (end.name(), end.weight()),
                (name, Weight::Unasked),
                "cause {cause:#x} with no reason said"
            );
            assert_eq!(
                boot.verdict(false, false),
                End::Unknown,
                "cause {cause:#x} elsewhere"
            );
        }
    }

    #[test]
    fn a_cause_left_set_never_marks_an_asked_reboot() {
        let boot = kept(Some(reset(0b1000, 3, 0x400)), Some(Crash::None));
        let end = boot.verdict(true, false);
        assert_eq!((end.name(), end.weight()), ("warm-reset", Weight::Plain));
        let unasked = kept(Some(reset(0b10, 0x21, 0x400)), None);
        assert_eq!(unasked.verdict(true, false).name(), "global-reset");
        // Nor where software was the source and no reason is said.
        let software = kept(Some(reset(0b1000, 0, 0x400)), Some(Crash::None));
        assert_eq!(software.verdict(true, false).weight(), Weight::Plain);
        // Nor do the bits that say heat, lost power or the button.
        for cause in [1 << 3, 1 << 11, 1 << 1] {
            let end = kept(Some(reset(0b1000, 3, cause)), None).verdict(true, false);
            assert_eq!(
                (end.name(), end.weight()),
                ("warm-reset", Weight::Plain),
                "cause {cause:#x} marked a warm reset"
            );
        }
    }

    #[test]
    fn the_watchdogs_word_is_said_where_no_crash_log_is() {
        let boot = kept(Some(reset(1, 0, 0)), Some(Crash::None));
        assert_eq!(boot.verdict(true, true), End::Watchdog);
        assert_eq!(boot.verdict(true, true).weight(), Weight::Reset);
        assert_eq!(boot.verdict(true, false), End::Unknown);
    }

    #[test]
    fn a_reason_is_weighed_by_whether_it_was_asked_for() {
        let at = |reason| kept(Some(reset(0b10, reason, 0)), None).verdict(true, false);
        assert_eq!(
            (at(0x24).words(), at(0x24).weight()),
            ("lost power".to_owned(), Weight::Unasked)
        );
        assert_eq!(
            (at(0x20).name(), at(0x20).weight()),
            ("unexpected-reset", Weight::Unasked)
        );
        assert_eq!((at(4).name(), at(4).weight()), ("update", Weight::Plain));
        assert_eq!(
            (at(1).words(), at(1).weight()),
            ("a cold boot".to_owned(), Weight::Plain)
        );
        assert_eq!(at(0x26), End::Unknown);
        assert_eq!(kept(None, None).verdict(true, false), End::Unknown);
    }

    #[test]
    fn an_error_kept_that_cant_be_read_is_unasked_not_a_reset() {
        let unreadable = kept(None, Some(Crash::Unreadable("cut".to_owned())));
        assert_eq!(unreadable.verdict(true, false), End::Unreadable);
        assert_eq!(unreadable.verdict(true, false).weight(), Weight::Unasked);
        let other = kept(None, Some(Crash::Other));
        assert_eq!(other.verdict(true, false).weight(), Weight::Unasked);
        let reason = kept(Some(reset(0b10, 0x21, 0)), Some(Crash::Other));
        assert_eq!(reason.verdict(true, false).name(), "fault");
    }

    #[test]
    fn an_error_kept_says_more_than_a_reboot_asked_for() {
        let cold = || Some(reset(0b10, 1, 0));
        let other = kept(cold(), Some(Crash::Other));
        assert_eq!(other.verdict(true, false), End::OtherError);
        let cut = kept(cold(), Some(Crash::Unreadable("cut".to_owned())));
        assert_eq!(cut.verdict(true, false), End::Unreadable);
        let mut unread = kept(cold(), Some(Crash::None));
        unread.unread.push((BERT_REGION, "denied".to_owned()));
        assert_eq!(unread.verdict(true, false), End::Unread);
        assert_eq!(unread.verdict(true, false).weight(), Weight::Unasked);
        let quiet = kept(cold(), Some(Crash::None));
        assert_eq!(quiet.verdict(true, false).name(), "cold-boot");
    }

    /// A state folder with a boot's folder in it, its tables written at
    /// the given second, holding the given files.
    fn boot_in(state: &Path, id: &str, at: u64, files: &[(&str, &[u8])]) {
        let folder = state.join(id);
        fs::create_dir_all(&folder).expect("a boot's folder");
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(at);
        for (name, bytes) in files {
            fs::write(folder.join(name), bytes).expect("a kept table");
            fs::File::options()
                .write(true)
                .open(folder.join(name))
                .and_then(|file| file.set_modified(time))
                .expect("the table's time");
        }
    }

    fn state(name: &str) -> std::path::PathBuf {
        let state =
            std::env::temp_dir().join(format!("firmware-errors-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&state);
        fs::create_dir_all(&state).expect("a state folder");
        state
    }

    #[test]
    fn boots_are_listed_by_when_they_began_and_other_names_left_out() {
        let state = state("listed");
        boot_in(
            &state,
            "53cb792037684896bc1e3870381c2398",
            100,
            &[("phat-table", b"")],
        );
        // Written later than 949a's, but it says it began before 949a's
        // tables were written, which is what places it.
        boot_in(
            &state,
            "6bccd85d80714583afd805c687f0cdb6",
            900,
            &[("began", b"150\n"), ("phat-table", b"")],
        );
        // Its tables written at two times, the first of which places it.
        boot_in(
            &state,
            "949a397ceafb4b789dd6b3d0e50c92e9",
            250,
            &[("bert-table", b"")],
        );
        boot_in(
            &state,
            "949a397ceafb4b789dd6b3d0e50c92e9",
            200,
            &[("phat-table", b"PHAT")],
        );
        boot_in(&state, "not-a-boot", 400, &[("phat-table", b"")]);
        // A folder holding no table is no kept boot, whatever else it holds.
        boot_in(
            &state,
            "83f85022e7b24638b538d6dcafafea46",
            500,
            &[("began", b"500\n")],
        );
        let kept = kept_in(&state).expect("the boots");
        let order: Vec<&str> = kept.iter().map(|boot| &boot.boot[..4]).collect();
        assert_eq!(order, ["949a", "6bcc", "53cb"]);
        // A folder that doesn't say when its boot began is ordered by when
        // its tables were first written, which isn't said as when it began.
        assert_eq!(
            kept.iter()
                .map(|boot| (boot.began, boot.written))
                .collect::<Vec<_>>(),
            [(None, Some(200)), (Some(150), Some(900)), (None, Some(100))]
        );
        assert_eq!((kept[0].reset.clone(), kept[0].crash.clone()), (None, None));
        let _ = fs::remove_dir_all(&state);
    }

    #[test]
    fn a_folder_that_cant_be_read_is_said_and_the_others_still_listed() {
        let state = state("unread");
        boot_in(&state, "53cb792037684896bc1e3870381c2398", 100, &[]);
        // A folder where a table should be can't be read as one.
        fs::create_dir_all(state.join("53cb792037684896bc1e3870381c2398/bert-region"))
            .expect("a folder in a table's place");
        boot_in(
            &state,
            "6bccd85d80714583afd805c687f0cdb6",
            300,
            &[("began", b"300"), ("phat-table", b"")],
        );
        let kept = kept_in(&state).expect("the boots");
        assert_eq!(kept.len(), 2, "a bad folder took the listing with it");
        let bad = kept
            .iter()
            .find(|boot| boot.boot.starts_with("53cb"))
            .expect("the bad boot");
        assert!(
            bad.unread.iter().map(|(name, _)| *name).eq(["bert-region"]),
            "the bad table wasn't said: {:?}",
            bad.unread
        );
        assert_eq!(bad.verdict(true, false), End::Unread);
        let _ = fs::remove_dir_all(&state);
    }

    #[test]
    fn a_folder_that_cant_be_searched_is_listed_and_said_unread() {
        use std::os::unix::fs::PermissionsExt;
        let state = state("closed");
        let id = "53cb792037684896bc1e3870381c2398";
        boot_in(&state, id, 100, &[("phat-table", b""), ("began", b"100")]);
        let closed = |mode| fs::set_permissions(state.join(id), fs::Permissions::from_mode(mode));
        // The build runs the tests as an account that isn't root, which a
        // folder's mode keeps out.
        closed(0o000).expect("the folder closed");
        let kept = kept_in(&state);
        closed(0o700).expect("the folder opened again");
        let kept = kept.expect("the boots");
        assert_eq!(
            kept.len(),
            1,
            "a folder that can't be searched was left out"
        );
        assert_eq!(kept[0].verdict(true, false), End::Unread);
        assert!(
            kept[0]
                .unread
                .iter()
                .map(|(name, _)| *name)
                .eq([PHAT_TABLE, BERT_REGION]),
            "the tables it couldn't read weren't said: {:?}",
            kept[0].unread
        );
        let _ = fs::remove_dir_all(&state);
    }

    #[test]
    fn a_began_that_cant_be_read_leaves_the_end_as_the_tables_say() {
        let state = state("began");
        let id = "53cb792037684896bc1e3870381c2398";
        boot_in(&state, id, 100, &[("phat-table", b"")]);
        fs::create_dir_all(state.join(id).join(BEGAN)).expect("a folder in began's place");
        let kept = kept_in(&state).expect("the boots");
        assert_eq!((kept[0].began, kept[0].unread.len()), (None, 0));
        let _ = fs::remove_dir_all(&state);
    }

    #[test]
    fn the_tables_are_read_by_the_names_the_keeping_gives_them() {
        let keep = include_str!("../keep.sh");
        for name in TABLES.iter().chain([&BEGAN]) {
            assert!(
                keep.contains(&format!(" {name} ")) || keep.contains(&format!("/{name}\"")),
                "keep.sh doesn't write {name}"
            );
        }
    }

    #[test]
    fn a_missing_state_folder_is_no_boots() {
        let kept = kept_in(Path::new("/nonexistent/firmware-errors")).expect("no boots");
        assert_eq!(kept, []);
    }

    #[test]
    fn a_boot_is_found_by_any_way_its_id_is_written() {
        let boots = [kept(None, None)];
        let found = |id: &str| plain_id(id).map(|plain| matching(&boots, &plain).len());
        assert_eq!(found("6BCCD85D-8071"), Some(1));
        assert_eq!(found("6bccd85d80714583afd805c687f0cdb6"), Some(1));
        assert_eq!(found("949a"), Some(0));
        // journalctl's index of the boot before, and of this one, and a
        // short flag, each of which strips to the start of a hex id.
        for not_an_id in ["", "-", "--", "6bcc/..", "zz", "-1", "0", "-6bcc", "6bc"] {
            assert_eq!(found(not_an_id), None, "{not_an_id:?} was taken as an id");
        }
    }
}
