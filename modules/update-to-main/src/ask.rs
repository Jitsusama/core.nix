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
