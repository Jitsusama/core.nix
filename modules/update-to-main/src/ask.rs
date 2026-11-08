//! The asking end: sends the one ask and gives back each line as it comes,
//! ending at the update's ending.

use crate::line::Line;
use crate::serve::ASK;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// Asks the machine's update-to-main to update, telling `hear` each line,
/// and gives the ending. A connection that ends before one is an error.
pub fn ask(socket: &Path, hear: &mut dyn FnMut(&Line)) -> io::Result<Line> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(stream, "{ASK}")?;
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
