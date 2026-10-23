//! Which processes hold a camera's node, and the programs they are, read
//! from /proc.

use std::collections::{BTreeSet, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Programs that run a script, so are named by the script instead.
const INTERPRETERS: [&str; 7] = ["bash", "sh", "dash", "perl", "ruby", "node", "python"];
/// How long a name may be, so a line stays a line a person reads.
const LONGEST: usize = 64;
/// The longest name the kernel keeps for a process, and one more for the
/// end it cuts at.
const COMM: usize = 15;

/// A name as a line can carry it: printable, trimmed, and not too long.
pub fn text(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .trim()
        .chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .take(LONGEST)
        .collect()
}

/// A name without the dot and -wrapped a Nix wrapper puts around it.
pub fn unwrapped(name: &str) -> String {
    name.strip_prefix('.')
        .and_then(|rest| rest.strip_suffix("-wrapped"))
        .unwrap_or(name)
        .to_owned()
}

fn base(path: &[u8]) -> &[u8] {
    Path::new(OsStr::from_bytes(path))
        .file_name()
        .map_or(path, OsStrExt::as_bytes)
}

fn arguments(proc: &Path) -> io::Result<Vec<Vec<u8>>> {
    let line = fs::read(proc.join("cmdline"))?;
    Ok(line
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(<[u8]>::to_vec)
        .collect())
}

/// A process's program by its own name. The kernel cuts that name at 15
/// characters, so a Nix wrapper's, or one that long, is taken from the
/// executable or the name it was run by. An interpreter is named by the
/// script it runs.
pub fn program(proc: &Path) -> io::Result<String> {
    let name = text(&fs::read(proc.join("comm"))?);
    if INTERPRETERS.contains(&name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')) {
        // Its script, the first argument that's a file: not an option, and
        // not the command a -c hands it.
        for argument in arguments(proc)?.iter().skip(1) {
            if argument.starts_with(b"-") {
                if argument == b"-c" {
                    break;
                }
                continue;
            }
            if Path::new(OsStr::from_bytes(argument)).is_file() {
                return Ok(unwrapped(&text(base(argument))));
            }
            break;
        }
        return Ok(name);
    }
    if !name.starts_with('.') && name.chars().count() < COMM {
        return Ok(name);
    }
    let exe = fs::read_link(proc.join("exe"))
        .map(|exe| text(base(exe.as_os_str().as_bytes())))
        .unwrap_or_default();
    // The executable only when it's the same program, not an interpreter.
    let start: String = name.trim_start_matches('.').chars().take(8).collect();
    if !exe.is_empty() && exe.trim_start_matches('.').starts_with(&start) {
        return Ok(unwrapped(&exe));
    }
    if name.starts_with('.') {
        return Ok(unwrapped(&name));
    }
    if let Some(first) = arguments(proc)?.first() {
        let called = unwrapped(&text(base(first)));
        if !called.is_empty() {
            return Ok(called);
        }
    }
    Ok(name)
}

/// Whether a process is in one of the units, by its cgroup, which no
/// program can put itself in as it can take any name.
pub fn ignored(cgroup: &str, units: &[String]) -> bool {
    cgroup.lines().any(|line| {
        let path = line.rsplit_once(':').map_or(line, |(_, path)| path);
        path.split('/')
            .any(|part| units.iter().any(|unit| unit == part))
    })
}

/// Whether a process's maps still hold the node's buffers, as a reader
/// that closed its descriptor and kept streaming would.
pub fn maps(mapped: &[u8], node: &Path, inode: u64) -> bool {
    mapped.split(|byte| *byte == b'\n').any(|line| {
        let mut fields = line
            .splitn(6, u8::is_ascii_whitespace)
            .filter(|f| !f.is_empty());
        let inode_field = fields.nth(4);
        let path = fields.next().map(<[u8]>::trim_ascii);
        inode_field.and_then(|f| std::str::from_utf8(f).ok()?.parse::<u64>().ok()) == Some(inode)
            && path == Some(node.as_os_str().as_bytes())
    })
}

/// Whether a process has the node open, under any name: a by-id link, or
/// /dev/videoN.
fn holds(proc: &Path, device: u64) -> io::Result<bool> {
    for fd in fs::read_dir(proc.join("fd"))? {
        let fd = fd?.path();
        let Ok(link) = fs::read_link(&fd) else {
            continue;
        };
        if !link.as_os_str().as_bytes().starts_with(b"/dev/") {
            continue;
        }
        if fs::metadata(&fd).is_ok_and(|status| status.rdev() == device) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The programs holding the node, and their processes. It fails only
/// when the node itself has gone; a process can end at any point while
/// it's read, and whatever it does, it costs only itself.
pub fn holders(
    node: &Path,
    units: &[String],
    before: &HashSet<u32>,
) -> io::Result<(BTreeSet<String>, HashSet<u32>)> {
    let status = fs::metadata(node)?;
    let own = std::process::id();
    let mut names = BTreeSet::new();
    let mut pids = HashSet::new();
    for entry in fs::read_dir("/proc")?.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == own {
            continue;
        }
        let proc = entry.path();
        let found = (|| -> io::Result<Option<String>> {
            let mut held = holds(&proc, status.rdev())?;
            if !held && before.contains(&pid) {
                held = maps(&fs::read(proc.join("maps"))?, node, status.ino());
            }
            if !held || ignored(&fs::read_to_string(proc.join("cgroup"))?, units) {
                return Ok(None);
            }
            program(&proc).map(Some)
        })();
        if let Ok(Some(name)) = found {
            names.insert(name);
            pids.insert(pid);
        }
    }
    Ok((names, pids))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_one_printable_line_of_a_readable_length() {
        assert_eq!(text(b" zoom\n"), "zoom");
        assert_eq!(text(b"a\nb\xff"), "a?b\u{fffd}");
        assert_eq!(text(&[b'x'; 100]).len(), LONGEST);
    }

    #[test]
    fn a_nix_wrapper_is_named_for_what_it_wraps() {
        assert_eq!(unwrapped(".firefox-wrapped"), "firefox");
        assert_eq!(unwrapped("firefox-wrapped"), "firefox-wrapped");
        assert_eq!(unwrapped(".hidden"), ".hidden");
    }

    #[test]
    fn only_a_unit_in_the_cgroup_path_is_ignored() {
        let units = ["stand-in.service".to_owned()];
        assert!(ignored("0::/system.slice/stand-in.service\n", &units));
        assert!(!ignored(
            "0::/user.slice/user-1000.slice/app.slice/x.scope\n",
            &units
        ));
        assert!(!ignored("0::/system.slice/stand-in.service.d\n", &units));
    }

    #[test]
    fn a_mapping_counts_only_by_the_nodes_inode_and_path() {
        let maps_text = b"7f00-7f10 rw-s 00000000 00:06 812 /dev/video0\n\
            7f10-7f20 r-xp 00000000 103:02 55 /usr/lib/libc.so\n";
        assert!(maps(maps_text, Path::new("/dev/video0"), 812));
        assert!(!maps(maps_text, Path::new("/dev/video0"), 55));
        assert!(!maps(maps_text, Path::new("/dev/video1"), 812));
    }

    #[test]
    fn this_process_is_named_by_its_own_program() {
        let named = program(Path::new("/proc/self")).unwrap();
        assert!(!named.is_empty());
        assert!(!named.contains('\n'));
    }
}
