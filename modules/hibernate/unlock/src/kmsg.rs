//! The kernel's log, read from `/dev/kmsg`: its place is kept before an
//! image is made, so what it logged since can be read alone once the
//! machine is back, to say why a hibernation rolled back (why.rs).

use std::fs;
use std::io::{ErrorKind, Read};
use std::os::unix::fs::OpenOptionsExt;

/// Where the number of the kernel's last log record before a hibernation is
/// kept, so what it logged after can be read alone.
pub(crate) const LOGGED: &str = "/run/hibernate-unlock-logged";

/// One record of the kernel's log, as `/dev/kmsg` gives it:
/// `<level>,<number>,<time>,<flags>;<message>`, then any of its fields on
/// lines of their own.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Record {
    pub(crate) number: u64,
    message: String,
}

impl Record {
    fn parse(raw: &str) -> Option<Self> {
        let (head, rest) = raw.split_once(';')?;
        Some(Self {
            number: head.split(',').nth(1)?.parse().ok()?,
            message: rest.lines().next().unwrap_or_default().to_owned(),
        })
    }
}

/// Every record the kernel's log still holds, oldest first, or none if it
/// can't be read. Read without blocking, it ends where the log does.
pub(crate) fn kernel_log() -> Vec<Record> {
    let Ok(mut log) = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/kmsg")
    else {
        return Vec::new();
    };
    let mut records = Vec::new();
    // The longest record the kernel gives a reader.
    let mut buffer = vec![0_u8; 8192];
    loop {
        match log.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => records.extend(Record::parse(&String::from_utf8_lossy(&buffer[..read]))),
            // A record written over before it was read; the next follows.
            Err(error) if error.kind() == ErrorKind::BrokenPipe => {}
            Err(_) => break,
        }
    }
    records
}

/// What the kernel logged since `before` kept its place.
pub(crate) fn logged_since_before() -> Vec<String> {
    let Some(since) = fs::read_to_string(LOGGED)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
    else {
        return Vec::new();
    };
    kernel_log()
        .into_iter()
        .filter(|record| record.number > since)
        .map(|record| record.message)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Record;

    #[test]
    fn a_record_of_the_kernels_log_is_read_with_its_number() {
        assert_eq!(
            Record::parse(
                "3,2813,4021375310,-;amdgpu 0000:03:00.0: PM: failed to freeze: error -16\n \
                 SUBSYSTEM=pci\n DEVICE=+pci:0000:03:00.0\n"
            ),
            Some(Record {
                number: 2813,
                message: "amdgpu 0000:03:00.0: PM: failed to freeze: error -16".to_owned(),
            })
        );
        assert_eq!(Record::parse("no record"), None);
        assert_eq!(Record::parse("6,x,1,-;nothing"), None);
    }
}
