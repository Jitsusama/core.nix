//! What the far line says of a job before it runs it, on a line of its own
//! on stdout, so the near side tells a job it started from one it refused
//! or couldn't take, and either from a far side mycelium never reached.
//! Its code can't say so alone: mycelium ends with sysexits of its own,
//! which a far line's 75 or 77 would read the same as.

use std::io::{self, BufRead, BufReader, Read, Write};
use terminal_kit::Exit;

/// What the far line ends with when it refuses the job's tree before the
/// job starts, as it would again on another try.
pub const REFUSES: u8 = 77;

/// What the far line ends with when it couldn't take the job's files
/// before the job starts, which another try may not meet.
pub const UNTAKEN: u8 = 75;

/// What the far line said of a job before it ran it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heard {
    /// It started the job, whose own output follows.
    Started,
    /// It refused the job's tree, as it would on every try.
    Refused,
    /// The job's files never reached it, or it couldn't make the job's
    /// tree of them, which another try may not meet.
    Untaken,
    /// Nothing: mycelium never reached it, or it ended before saying.
    Nothing,
}

impl Heard {
    /// The line the far line writes for the job `id` when it says this.
    pub fn line(self, id: &str) -> String {
        let said = match self {
            Heard::Started => "running",
            Heard::Refused => "refusing",
            Heard::Untaken => "can't take",
            Heard::Nothing => "silent on",
        };
        format!("elsewhere: {said} {id}")
    }

    /// How a far run that said this and never started its job ends here,
    /// and what the far machine did.
    pub fn unstarted(self) -> (Exit, &'static str) {
        match self {
            Heard::Refused => (Exit::Refused, "refused its tree"),
            Heard::Untaken => (Exit::TryAgain, "couldn't take its files"),
            Heard::Started | Heard::Nothing => {
                (Exit::Unavailable, "wasn't reached through mycelium")
            }
        }
    }
}

/// Hands on what the far run says on `out`: what comes before the line
/// saying the job started is the far shell's own, and goes to stderr; what
/// comes after is the job's, and goes to stdout as it comes. Says what the
/// far line said of the job, `Started` once it did, which a stream lost
/// after it never unsays, so a job that ran is never run again here.
pub fn relay(out: impl Read, id: &str) -> Heard {
    let says = [Heard::Started, Heard::Refused, Heard::Untaken]
        .map(|heard| (heard.line(id).into_bytes(), heard));
    let mut out = BufReader::new(out);
    let mut line = Vec::new();
    let heard = loop {
        line.clear();
        match out.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return Heard::Nothing,
            Ok(_) => {
                // As a terminal would end it, too, should mycelium give
                // the far side one.
                let bare = line.strip_suffix(b"\n").unwrap_or(&line);
                let bare = bare.strip_suffix(b"\r").unwrap_or(bare);
                match says.iter().find(|(said, _)| *said == bare) {
                    Some((_, heard)) => break *heard,
                    // The blank line each word follows is the far line's.
                    None if bare.is_empty() => {}
                    // The far shell's own words are worth no failure of
                    // the job's.
                    None => drop(io::stderr().write_all(&line)),
                }
            }
        }
    };
    // Whatever follows is the job's once it started, and otherwise the
    // far side's, read to its end so mycelium never writes to nobody.
    let drained = if heard == Heard::Started {
        let mut stdout = io::stdout();
        io::copy(&mut out, &mut stdout).and_then(|_| stdout.flush())
    } else {
        io::copy(&mut out, &mut io::stderr()).map(drop)
    };
    // A reader of ours that has gone is the job's caller's business, and
    // the job's code still says how it ended.
    drop(drained);
    heard
}
