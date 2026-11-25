//! What a step read from disk, so the journal can name the walk that costs
//! it. On 25 November the account guard read 15.3 GB in 24 minutes of a
//! reach pass while its read calls came to 0.6 GB: metadata of walks, which
//! no step's seconds told apart. Linux counts it per thread, and each step
//! named here runs on one, so another thread's walk at the same time is
//! never counted to it. What a step hands to a thread of its own, a
//! command, a Nix delete or an ask of the system daemon, isn't counted.

use std::time::Instant;

/// The bytes the calling thread has had read from storage so far, from
/// `/proc/thread-self/io`, or none where that can't be read, as on a Mac.
#[must_use]
pub fn so_far() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/thread-self/io")
            .ok()
            .and_then(|io| read_bytes(&io))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// The `read_bytes` line of a task's `io` file: what it had storage read,
/// past the page cache, which is what a walk costs the disk.
#[must_use]
pub fn read_bytes(io: &str) -> Option<u64> {
    io.lines()
        .find_map(|line| line.strip_prefix("read_bytes:"))
        .and_then(|bytes| bytes.trim().parse().ok())
}

/// A walk begun on one thread, ended on the same one.
#[derive(Debug)]
pub struct Walk {
    started: Instant,
    read: Option<u64>,
}

/// How long a walk took, in seconds, and what it had read from disk, or
/// none where that can't be told.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walked {
    pub seconds: f64,
    pub read: Option<u64>,
}

impl Walk {
    #[must_use]
    pub fn start() -> Self {
        Self {
            started: Instant::now(),
            read: so_far(),
        }
    }

    #[must_use]
    pub fn end(&self) -> Walked {
        Walked {
            seconds: self.started.elapsed().as_secs_f64(),
            read: so_far()
                .zip(self.read)
                .map(|(now, then)| now.saturating_sub(then)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bytes_is_found_among_the_other_counts() {
        let io = "rchar: 8971\nwchar: 0\nsyscr: 12\nsyscw: 0\nread_bytes: 40960\n\
                  write_bytes: 0\ncancelled_write_bytes: 0\n";
        assert_eq!(
            read_bytes(io),
            Some(40960),
            "read_bytes wasn't read from {io:?}"
        );
    }

    #[test]
    fn a_file_without_read_bytes_says_nothing() {
        assert_eq!(read_bytes("rchar: 1\n"), None, "a count was made up");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_walk_on_linux_says_what_this_thread_read() {
        assert!(
            Walk::start().end().read.is_some(),
            "this thread's reads couldn't be told"
        );
    }
}
