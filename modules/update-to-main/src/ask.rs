//! The asking end: sends an ask and gives back each line as it comes,
//! ending at the update's ending.

use crate::line::Line;
use crate::serve::{ASK, AT, FOLLOW, STOP, STOP_FOR_NEWER};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// Asks the machine's update-to-main to update, telling `hear` each line,
/// and gives the ending. A connection that ends before one is an error.
pub fn ask(socket: &Path, hear: &mut dyn FnMut(&Line)) -> io::Result<Line> {
    asking(socket, ASK, hear)
}

/// Asks as `ask` does, with the followed input `input` at `commit` rather
/// than its main's head.
pub fn ask_at(
    socket: &Path,
    input: &str,
    commit: &str,
    hear: &mut dyn FnMut(&Line),
) -> io::Result<Line> {
    asking(socket, &format!("{ASK} {AT} {input} {commit}"), hear)
}

/// Asks as `ask` does, for the follower, whose switch waits once built
/// while someone is at the machine.
pub fn follow(socket: &Path, hear: &mut dyn FnMut(&Line)) -> io::Result<Line> {
    asking(socket, FOLLOW, hear)
}

fn asking(socket: &Path, word: &str, hear: &mut dyn FnMut(&Line)) -> io::Result<Line> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(stream, "{word}")?;
    for line in BufReader::new(stream).lines() {
        let line: Line = serde_json::from_str(&line?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        hear(&line);
        if line.ends() {
            return Ok(line);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "update-to-main stopped before saying how the update ended",
    ))
}

/// Asks the machine's update-to-main, for a person, to stop the update
/// it's fetching or building, one a hold was given for too, and gives its
/// one answer, which says what it did.
pub fn stop(socket: &Path) -> io::Result<Line> {
    once(socket, STOP)
}

/// Asks as `stop` does once main has moved past the heads the update
/// builds, which leaves one a hold was given for to run.
pub fn stop_for_newer(socket: &Path) -> io::Result<Line> {
    once(socket, STOP_FOR_NEWER)
}

/// Sends an ask with one answer, such as a pause or a hold on a commit,
/// and gives that answer.
pub fn once(socket: &Path, ask: &str) -> io::Result<Line> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(stream, "{ask}")?;
    let mut said = String::new();
    BufReader::new(stream).read_line(&mut said)?;
    serde_json::from_str(&said).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// The ending an ask that never reached an answer stands for, saying why.
/// A refused connection most often means the asking process holds the
/// groups of a session started before its account was made an asker,
/// which a switch can't give a session already running, so it says so.
pub fn unanswered(socket: &Path, error: &io::Error) -> Line {
    let mut why = format!("{}: {error}", socket.display());
    if error.kind() == io::ErrorKind::PermissionDenied {
        why.push_str(
            "; an account may ask only from a session started since it became an asker, \
             so this one can ask once its session starts again",
        );
    }
    Line::Refused { commit: None, why }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn why(line: &Line) -> &str {
        match line {
            Line::Refused { why, .. } => why,
            other => panic!("an unanswered ask ended as {other:?}, not refused"),
        }
    }

    #[test]
    fn a_refused_connection_says_the_session_must_start_again() {
        let error = io::Error::from(io::ErrorKind::PermissionDenied);
        let line = unanswered(Path::new("/run/update-to-main/ask.sock"), &error);
        assert!(why(&line).starts_with("/run/update-to-main/ask.sock: "));
        assert!(why(&line).contains("once its session starts again"));
    }

    #[test]
    fn another_error_says_only_itself() {
        let error = io::Error::from(io::ErrorKind::NotFound);
        let line = unanswered(Path::new("/run/update-to-main/ask.sock"), &error);
        assert!(!why(&line).contains("session"));
    }
}
