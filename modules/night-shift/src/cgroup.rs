//! A job's limits, kept off the watcher: night-shift moves itself into a
//! `watch` cgroup under its unit's, and runs the job in a `job` cgroup
//! beside it that carries the limits. A job held at its memory limit is
//! made to reclaim, and every process in that cgroup with it, so a
//! watcher sharing the job's cgroup stalls with the job and can't stop
//! it: on 11 November one stopped sampling the moment its job passed its
//! high, while the machine's full pressure climbed past 30%.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// What a job may hold and read, as its cgroup's files take them.
#[derive(Debug, Default, PartialEq)]
pub struct Limits {
    /// `memory.high`, `memory.max` and `memory.swap.max`, in the kernel's
    /// own sizes, such as `12G`.
    pub memory_high: Option<String>,
    pub memory_max: Option<String>,
    pub swap_max: Option<String>,
    /// Each disk the job reads, and the most bytes a second it may.
    pub reads: Vec<(PathBuf, u64)>,
}

impl Limits {
    /// Whether there is any limit to keep off the watcher.
    pub fn any(&self) -> bool {
        self.memory_high.is_some()
            || self.memory_max.is_some()
            || self.swap_max.is_some()
            || !self.reads.is_empty()
    }
}

/// A rate as systemd's `IOReadBandwidthMax` reads it: bytes a second,
/// suffixed with K, M, G or T to the base of 1000.
pub fn rate(text: &str) -> Option<u64> {
    let (digits, scale) = match text.chars().last()? {
        'K' => (&text[..text.len() - 1], 1_000),
        'M' => (&text[..text.len() - 1], 1_000_000),
        'G' => (&text[..text.len() - 1], 1_000_000_000),
        'T' => (&text[..text.len() - 1], 1_000_000_000_000),
        _ => (text, 1),
    };
    digits.parse::<u64>().ok()?.checked_mul(scale)
}

/// A disk's number as `io.max` names it, `MAJOR:MINOR`, from its device
/// file, following a link such as `/dev/mapper/system`.
fn device(disk: &Path) -> io::Result<String> {
    let number = fs::metadata(disk)?.rdev();
    let major = ((number >> 32) & 0xffff_f000) | ((number >> 8) & 0x0000_0fff);
    let minor = ((number >> 12) & 0xffff_ff00) | (number & 0x0000_00ff);
    Ok(format!("{major}:{minor}"))
}

fn put(file: &Path, text: &str) -> io::Result<()> {
    fs::write(file, text)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", file.display())))
}

/// Splits `unit`, the cgroup this process runs in, into `watch`, which
/// this process moves into, and `job`, which takes `limits`. Gives the
/// job's cgroup and its `cgroup.procs` open for writing, so the job can
/// move itself in before it runs. The unit has to be delegated the
/// memory and io controllers, as systemd's `Delegate=memory io` does.
pub fn split(unit: &Path, limits: &Limits) -> io::Result<(PathBuf, File)> {
    let watch = unit.join("watch");
    let job = unit.join("job");
    fs::create_dir_all(&watch)?;
    put(&watch.join("cgroup.procs"), &std::process::id().to_string())?;
    put(&unit.join("cgroup.subtree_control"), "+memory +io")?;
    fs::create_dir_all(&job)?;
    let sizes = [
        ("memory.high", &limits.memory_high),
        ("memory.max", &limits.memory_max),
        ("memory.swap.max", &limits.swap_max),
    ];
    for (file, size) in sizes {
        if let Some(size) = size {
            put(&job.join(file), size)?;
        }
    }
    for (disk, bytes) in &limits.reads {
        put(
            &job.join("io.max"),
            &format!("{} rbps={bytes}", device(disk)?),
        )?;
    }
    let procs = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(job.join("cgroup.procs"))?;
    Ok((job, procs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rate_reads_as_systemd_reads_it() {
        assert_eq!(rate("50M"), Some(50_000_000));
        assert_eq!(rate("2G"), Some(2_000_000_000));
        assert_eq!(rate("4096"), Some(4096));
        assert_eq!(rate("fast"), None);
        assert_eq!(rate("M"), None);
    }

    #[test]
    fn a_device_file_gives_its_number() {
        assert_eq!(device(Path::new("/dev/null")).unwrap(), "1:3");
    }

    #[test]
    fn no_limit_needs_no_split() {
        assert!(!Limits::default().any(), "no limits asked for a split");
        let reads = Limits {
            reads: vec![(PathBuf::from("/dev/null"), 1)],
            ..Limits::default()
        };
        assert!(reads.any(), "a read limit asked for no split");
    }
}
