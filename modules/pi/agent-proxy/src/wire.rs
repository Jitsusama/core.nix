//! The SSH agent protocol's framing and fields (draft-miller-ssh-agent):
//! each message is its length in four bytes, then its kind in one, then
//! fields of fixed-size integers and length-prefixed strings.

use std::io::{self, Read, Write};

/// The longest message read, as OpenSSH's own agent holds them to.
const MOST_BYTES: usize = 256 * 1024;

/// The next message from `stream`, or nothing once it ends between two.
pub fn read_message(stream: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut length = [0; 4];
    match stream.read_exact(&mut length) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let length = usize::try_from(u32::from_be_bytes(length)).unwrap_or(usize::MAX);
    if length == 0 || length > MOST_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("a message of {length} bytes, past the {MOST_BYTES} an agent takes"),
        ));
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    Ok(Some(body))
}

/// Send `body` as one message.
pub fn write_message(stream: &mut impl Write, body: &[u8]) -> io::Result<()> {
    let length = u32::try_from(body.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a message too long to frame"))?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// Add `bytes` to `out` as a string field.
pub fn put_string(out: &mut Vec<u8>, bytes: &[u8]) {
    let length = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(bytes);
}

/// A message's fields, read in order.
pub struct Fields<'a> {
    rest: &'a [u8],
}

impl<'a> Fields<'a> {
    /// The fields of `bytes`, from the first.
    pub fn new(bytes: &'a [u8]) -> Self {
        Fields { rest: bytes }
    }

    /// The next `count` bytes as they are.
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        if self.rest.len() < count {
            return Err("it ends before its fields do".to_owned());
        }
        let (taken, rest) = self.rest.split_at(count);
        self.rest = rest;
        Ok(taken)
    }

    /// The next byte.
    pub fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    /// The next byte, read as a boolean.
    pub fn boolean(&mut self) -> Result<bool, String> {
        Ok(self.byte()? != 0)
    }

    /// The next four bytes, read as a number.
    pub fn number(&mut self) -> Result<u32, String> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// The next string.
    pub fn string(&mut self) -> Result<&'a [u8], String> {
        let length = usize::try_from(self.number()?).unwrap_or(usize::MAX);
        self.take(length)
    }

    /// That nothing follows the fields read.
    pub fn done(&self) -> Result<(), String> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(format!("{} bytes follow its fields", self.rest.len()))
        }
    }
}
