//! A Mac's desktop: whether the person has asked to save power, through
//! pmset, and the notice the user daemon keeps them told with, through
//! osascript. macOS keeps what it showed, so a notice once shown can't be
//! withdrawn, only followed by the next.

use crate::commands::{Job, Ran};
use crate::notices::{Change, Urgency};
use std::ffi::OsString;
use std::time::{Duration, Instant};

/// How long a command may take, so a wedged one never stops the daemon's
/// loop, the urgent passes with it.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long Low Power Mode as read is trusted, since it changes only when
/// the person changes it and each step would otherwise ask.
const PROFILE_FRESH: Duration = Duration::from_secs(30);

/// The most of a command's output read.
const OUTPUT_LIMIT: usize = 64 << 10;

/// What the desktop is to its daemon, telling only when `notify`.
pub fn desk(notify: bool) -> Desktop {
    Desktop {
        notify,
        profile: None,
    }
}

pub struct Desktop {
    notify: bool,
    profile: Option<(Instant, bool)>,
}

/// What a command printed, or why it failed.
fn ran(argv: &[&str]) -> Result<String, String> {
    let job = Job {
        argv: argv.iter().map(OsString::from).collect(),
        cwd: None,
        env: Vec::new(),
        timeout: PATIENCE,
        limit: OUTPUT_LIMIT,
        idle: false,
    };
    match crate::commands::run(&job).map_err(|error| format!("running {}: {error}", argv[0]))? {
        Ran::Finished(output) if output.status == Some(0) => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ran::Finished(output) => Err(format!(
            "{} failed: {}",
            argv[0],
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Ran::TimedOut => Err(format!("{} ran past {PATIENCE:?}", argv[0])),
        Ran::Stopped => Err(format!("{} was stopped with the guard", argv[0])),
    }
}

impl crate::serve::Desk for Desktop {
    fn saving(&mut self) -> bool {
        if let Some((at, saving)) = self.profile
            && at.elapsed() < PROFILE_FRESH
        {
            return saving;
        }
        let saving =
            ran(&["/usr/bin/pmset", "-g"]).is_ok_and(|text| crate::pmset::low_power(&text));
        self.profile = Some((Instant::now(), saving));
        saving
    }

    fn tell(&mut self, change: &Change) -> Result<(), String> {
        let Change::Show(notice) = change else {
            return Ok(());
        };
        if !self.notify {
            return Ok(());
        }
        // The words go as arguments, so nothing in them is read as script.
        let sound = match notice.urgency {
            Urgency::Critical => " sound name \"Basso\"",
            Urgency::Normal => "",
        };
        let show =
            format!("display notification (item 2 of argv) with title (item 1 of argv){sound}");
        ran(&[
            "/usr/bin/osascript",
            "-e",
            "on run argv",
            "-e",
            &show,
            "-e",
            "end run",
            &notice.summary,
            &notice.body,
        ])
        .map(drop)
    }
}
