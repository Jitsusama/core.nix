//! How far a lent builder's builds have got, for whatever judges a build
//! by its progress rather than its output: `nix-offload progress HOST`
//! prints, as a line of JSON, the processor time the builder's Nix
//! daemon and its builds have used and the bytes its disks have written,
//! both counted from its boot, and when they were read:
//!
//! ```json
//! {"host": "nix-offload-more", "cpuUsec": 235790660567,
//!  "writtenBytes": 1234567890, "at": 1794213000}
//! ```
//!
//! Two readings apart say whether anything moves: on 9 November a
//! switch's build was stopped for printing nothing for 12 minutes, and
//! one that prints nothing may still be compiling. The bytes are the whole
//! machine's, since a builder's daemon has no I/O accounting of its own,
//! so a session evaluating there counts too. It exits 75 when no lender
//! lends a builder by that name and 255 when ssh can't reach it.

use serde_json::json;

/// What the builder is asked: the daemon's processor time, then the
/// sectors its whole disks have written, each on a line.
pub const ASKED: &str = "awk '$1 == \"usage_usec\" { print $2 }' \
                         /sys/fs/cgroup/system.slice/nix-daemon.service/cpu.stat; \
                         awk '$3 ~ /^(sd[a-z]+|vd[a-z]+|nvme[0-9]+n[0-9]+)$/ \
                         { written += $10 } END { print written + 0 }' /proc/diskstats";

/// The two figures the builder answered `ASKED` with: processor time in
/// microseconds, and bytes written, from 512-byte sectors.
pub fn read(answer: &str) -> Option<(u64, u64)> {
    let mut lines = answer.lines().map(str::trim);
    let cpu = lines.next()?.parse().ok()?;
    let sectors: u64 = lines.next()?.parse().ok()?;
    Some((cpu, sectors.checked_mul(512)?))
}

/// The line `progress` prints for `host`, read `at`.
pub fn line(host: &str, (cpu, written): (u64, u64), at: u64) -> String {
    json!({
        "host": host,
        "cpuUsec": cpu,
        "writtenBytes": written,
        "at": at,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_builders_answer_is_its_processor_time_and_bytes_written() {
        assert_eq!(
            read("235790660567\n2048\n"),
            Some((235_790_660_567, 1_048_576))
        );
        assert_eq!(read("no daemon\n"), None);
        assert_eq!(read("17\n"), None, "written is missing");
    }

    #[test]
    fn its_line_names_the_builder_and_when() {
        let said: serde_json::Value = serde_json::from_str(&line("far", (5, 1024), 9)).unwrap();
        assert_eq!(
            said,
            json!({"host": "far", "cpuUsec": 5, "writtenBytes": 1024, "at": 9})
        );
    }
}
