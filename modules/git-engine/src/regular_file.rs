//! Opening a file of git's that must be a plain one: a ref, packed-refs or a
//! loose object. A FIFO there would hold the prompt till something wrote to
//! it, and a device such as /dev/zero would give bytes without end, so each
//! is opened without waiting on a writer and refused unless it's a regular
//! file, as git refuses one it can't read as a ref.

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// The regular file at `path`, or None where nothing is; anything else
/// there, a FIFO, a device or a folder, is refused as data that won't read.
pub(crate) fn open(path: &Path) -> io::Result<Option<File>> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if file.metadata()?.is_file() {
        Ok(Some(file))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} isn't a file", path.display()),
        ))
    }
}

/// The bytes of the regular file at `path`, or None where nothing is.
pub(crate) fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let Some(mut file) = open(path)? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::read;
    use std::path::Path;

    #[test]
    fn only_a_regular_file_is_read() {
        let folder = crate::pack_index::tests::unique("regular");
        std::fs::create_dir_all(&folder).expect("the temp folder takes a folder");
        let plain = folder.join("plain");
        std::fs::write(&plain, b"ref").expect("the temp folder takes a file");
        let fifo = folder.join("fifo");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo made no FIFO at {}", fifo.display());
        let read = |path: &Path| read(path).map_err(|error| error.kind());
        let found = [
            read(&plain),
            read(&folder.join("none")),
            read(&fifo),
            read(Path::new("/dev/zero")),
            read(&folder),
        ];
        std::fs::remove_dir_all(&folder).expect("the temp folder gives its folder back");
        let refused = Err(std::io::ErrorKind::InvalidData);
        assert_eq!(
            found,
            [
                Ok(Some(b"ref".to_vec())),
                Ok(None),
                refused.clone(),
                refused.clone(),
                refused
            ],
            "a plain file wasn't read, a missing one wasn't none, or a FIFO, a device or \
             a folder was read, or waited on, as a file"
        );
    }
}
