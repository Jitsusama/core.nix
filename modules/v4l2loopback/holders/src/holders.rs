//! Which processes hold a camera's node, and the programs they are, read
//! from /proc.

use std::cell::OnceCell;
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

/// A process's program by its own name, or `?` for one that gives none,
/// since a blank line reads as nobody and its camera would read as off.
pub fn program(proc: &Path) -> io::Result<String> {
    let name = own_name(proc)?;
    Ok(if name.is_empty() {
        "?".to_owned()
    } else {
        name
    })
}

/// A process's program by its own name. The kernel cuts that name at 15
/// characters, so a Nix wrapper's, or one that long, is taken from the
/// executable or the name it was run by. An interpreter is named by the
/// script it runs.
fn own_name(proc: &Path) -> io::Result<String> {
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

/// Whether a process is in one of the system's units, by its cgroup.
/// A program can take any name, and a person's own manager can start a
/// unit of any name too, but only under user@UID.service; so the unit
/// counts only where the system's manager puts it, as the first part of
/// the path that isn't a slice. Only the unified hierarchy's line is read,
/// since a legacy hierarchy can be shaped by whoever it's delegated to,
/// and a process with no such line, or more than the one the kernel
/// writes, is never left out: naming the relay shows the camera on, while
/// leaving out a reader would show it off. The kernel refuses a newline in
/// a cgroup's name, so a second line is no more than a guard.
pub fn ignored(cgroup: &str, units: &[String]) -> bool {
    let mut unified = cgroup.lines().filter_map(|line| line.strip_prefix("0::/"));
    let (Some(path), None) = (unified.next(), unified.next()) else {
        return false;
    };
    path.split('/')
        .find(|part| {
            part.rsplit_once('.')
                .is_none_or(|(_, kind)| kind != "slice")
        })
        .is_some_and(|part| units.iter().any(|unit| unit == part))
}

/// Whether a process is left out, by what reading its cgroup file gave.
/// It's read as bytes, since a person can name a cgroup of their own with
/// any byte but a slash or a newline; and a file that can't be read names
/// the process rather than leaving it out, as one that has ended fails to
/// be named anyway.
pub fn left_out(cgroup: &io::Result<Vec<u8>>, units: &[String]) -> bool {
    cgroup
        .as_ref()
        .is_ok_and(|text| ignored(&String::from_utf8_lossy(text), units))
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

/// The mount an open file is on, by the `mnt_id` line of its fdinfo.
fn mount_id(fdinfo: &[u8]) -> Option<u64> {
    fdinfo
        .split(|byte| *byte == b'\n')
        .find_map(|line| line.strip_prefix(b"mnt_id:"))
        .and_then(|id| std::str::from_utf8(id).ok()?.trim().parse().ok())
}

/// The filesystem type of a mount, by its id, as a process's mountinfo
/// says: the id is a line's first field, and the type is the first field
/// after the lone dash that ends the optional ones. A path there has its
/// spaces escaped, so splitting at spaces never cuts one.
fn mount_type(mountinfo: &[u8], id: u64) -> Option<&[u8]> {
    mountinfo.split(|byte| *byte == b'\n').find_map(|line| {
        let mut fields = line.split(|byte| *byte == b' ');
        let own: u64 = std::str::from_utf8(fields.next()?).ok()?.parse().ok()?;
        if own != id {
            return None;
        }
        fields.skip_while(|field| *field != b"-").nth(1)
    })
}

/// Whether an open file is on a devtmpfs mount, by what the kernel says
/// of the descriptor and of the process's mounts, never by the file. The
/// mounts are read the first time a process needs them, and kept for its
/// other descriptors; a descriptor or mounts that can't be read say no.
fn on_devtmpfs(proc: &Path, fd: &OsStr, mountinfo: &OnceCell<Option<Vec<u8>>>) -> bool {
    let Some(id) = fs::read(proc.join("fdinfo").join(fd))
        .ok()
        .as_deref()
        .and_then(mount_id)
    else {
        return false;
    };
    mountinfo
        .get_or_init(|| fs::read(proc.join("mountinfo")).ok())
        .as_deref()
        .and_then(|mounts| mount_type(mounts, id))
        .is_some_and(|kind| kind == b"devtmpfs")
}

/// Whether a process has the node open, under any name: a by-id link,
/// /dev/videoN, or the node bound anywhere else. A process in a mount
/// namespace of its own, as a sandboxed app is, still reads as /dev, its
/// own namespace's path. A link outside /dev is followed only when the
/// kernel puts its descriptor on a devtmpfs mount, as a bind of the node
/// is: a user namespace can't make a device node, and a tmpfs it mounts
/// can't hold one that opens, so devtmpfs is the one place a camera's
/// node comes from. Which mount it is comes from the descriptor's fdinfo
/// and the process's mountinfo, read only for such a link, rather than
/// from the file itself, since one on a network or FUSE filesystem that
/// hangs would hang the watch for every camera. A descriptor whose fdinfo
/// or mountinfo can't be read is skipped, as one that's closed is.
fn holds(proc: &Path, device: u64) -> io::Result<bool> {
    let mountinfo = OnceCell::new();
    for fd in fs::read_dir(proc.join("fd"))? {
        let fd = fd?;
        let path = fd.path();
        let Ok(link) = fs::read_link(&path) else {
            continue;
        };
        if !link.as_os_str().as_bytes().starts_with(b"/dev/")
            && !on_devtmpfs(proc, &fd.file_name(), &mountinfo)
        {
            continue;
        }
        if fs::metadata(&path).is_ok_and(|status| status.rdev() == device) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The programs holding the node, and their processes. It fails when the
/// node has gone, or when it or /proc can't be read; a process can end at
/// any point while it's read, and whatever it does, it costs only itself.
pub fn holders(
    node: &Path,
    units: &[String],
    before: &HashSet<u32>,
) -> io::Result<(BTreeSet<String>, HashSet<u32>)> {
    holders_in(Path::new("/proc"), node, units, before)
}

/// The programs holding the node, and their processes, as the process
/// folder given has them, so a test can walk a folder it made.
fn holders_in(
    proc_root: &Path,
    node: &Path,
    units: &[String],
    before: &HashSet<u32>,
) -> io::Result<(BTreeSet<String>, HashSet<u32>)> {
    let status = fs::metadata(node)?;
    let own = std::process::id();
    let mut names = BTreeSet::new();
    let mut pids = HashSet::new();
    for entry in fs::read_dir(proc_root)?.flatten() {
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
            if !held || left_out(&fs::read(proc.join("cgroup")), units) {
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

    /// Holds each cgroup file to whether its process is left out, with
    /// the relay's unit ignored.
    fn each_left_out(cases: &[(&str, bool)]) {
        each_left_out_of(&["stand-in.service"], cases);
    }

    /// Holds each cgroup file, read as the watch reads it, to whether its
    /// process is left out with these units ignored.
    fn each_left_out_of(units: &[&str], cases: &[(&str, bool)]) {
        let units: Vec<String> = units.iter().map(|&unit| unit.to_owned()).collect();
        for (cgroup, out) in cases {
            assert_eq!(
                left_out(&Ok(cgroup.as_bytes().to_vec()), &units),
                *out,
                "whether {cgroup:?} is left out"
            );
        }
    }

    #[test]
    fn only_a_system_unit_in_the_cgroup_path_is_ignored() {
        each_left_out(&[
            ("0::/system.slice/stand-in.service\n", true),
            ("0::/system.slice/stand-in.service/relay\n", true),
            ("0::/a.slice/a-b.slice/stand-in.service\n", true),
            ("0::/user.slice/user-1000.slice/app.slice/x.scope\n", false),
            ("0::/system.slice/stand-in.service.d\n", false),
        ]);
    }

    #[test]
    fn a_unit_a_person_starts_by_the_same_name_is_not_ignored() {
        // Anyone can run systemd-run --user --unit=stand-in.service, or
        // make a cgroup by that name in the subtree their manager is given.
        let user = "0::/user.slice/user-1000.slice/user@1000.service";
        each_left_out(&[
            (&format!("{user}/app.slice/stand-in.service\n"), false),
            (&format!("{user}/stand-in.service\n"), false),
            (
                &format!("{user}/app.slice/x.service/stand-in.service\n"),
                false,
            ),
            ("0::/system.slice/x.service/stand-in.service\n", false),
        ]);
    }

    #[test]
    fn only_the_unified_hierarchy_says_a_unit() {
        each_left_out(&[
            ("1:name=systemd:/system.slice/stand-in.service\n", false),
            (
                "1:name=systemd:/user.slice\n0::/system.slice/stand-in.service\n",
                true,
            ),
            // The kernel writes one unified line, so a file with two
            // says nothing trustworthy, whichever way round they are.
            (
                "0::/user.slice/x.scope\n0::/system.slice/stand-in.service\n",
                false,
            ),
            (
                "0::/system.slice/stand-in.service\n0::/user.slice/x.scope\n",
                false,
            ),
        ]);
    }

    #[test]
    fn a_process_in_no_unit_is_never_left_out() {
        // Only root can put a process straight in a slice, or at the root,
        // but either has no unit to leave out, even with its slice named.
        let slices = ["stand-in.service", "system.slice", "user-1000.slice"];
        each_left_out_of(
            &slices,
            &[
                ("0::/system.slice/x.service\n", false),
                ("0::/\n", false),
                ("0::/system.slice\n", false),
                ("0::/user.slice/user-1000.slice\n", false),
                ("", false),
            ],
        );
    }

    #[test]
    fn a_cgroup_file_that_isnt_text_or_cant_be_read_leaves_out_only_the_relay() {
        let units = ["stand-in.service".to_owned()];
        let user = b"0::/user.slice/user-1000.slice/user@1000.service";
        let odd = Ok([&user[..], b"/\xff/stand-in.service\n"].concat());
        assert!(
            !left_out(&odd, &units),
            "a cgroup named with a byte that isn't text is named"
        );
        let gone = Err(io::Error::from(io::ErrorKind::NotFound));
        assert!(
            !left_out(&gone, &units),
            "a cgroup file that can't be read is named"
        );
        let relay = Ok(b"0::/system.slice/stand-in.service/\xff\n".to_vec());
        assert!(
            left_out(&relay, &units),
            "a relay's own odd cgroup is still left out"
        );
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
        assert_ne!(named, "");
        assert!(!named.contains('\n'));
    }

    #[test]
    fn a_process_that_gives_no_name_is_still_named() {
        let proc = std::env::temp_dir().join(format!("holders-nameless-{}", std::process::id()));
        fs::create_dir_all(&proc).unwrap();
        for comm in ["\n", " \n"] {
            fs::write(proc.join("comm"), comm).unwrap();
            fs::write(proc.join("cmdline"), "").unwrap();
            assert_eq!(program(&proc).unwrap(), "?", "a comm of {comm:?}");
        }
        fs::remove_dir_all(&proc).unwrap();
    }

    /// An empty process folder of the test's own, named by its case, so
    /// tests that run at once never share one. One a failed run left is
    /// cleared first.
    fn fake_proc(case: &str) -> std::path::PathBuf {
        let name = format!("holders-{case}-{}", std::process::id());
        let root = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    /// A process named reader in the fake folder, whose descriptor 3 links
    /// to what it holds, and whose cgroup file holds the bytes given, or is
    /// missing when there are none.
    fn fake_process(root: &Path, pid: u32, held: &Path, cgroup: Option<&[u8]>) {
        let proc = root.join(pid.to_string());
        fs::create_dir_all(proc.join("fd")).unwrap();
        fs::write(proc.join("comm"), "reader\n").unwrap();
        fs::write(proc.join("cmdline"), "").unwrap();
        std::os::unix::fs::symlink(held, proc.join("fd/3")).unwrap();
        if let Some(cgroup) = cgroup {
            fs::write(proc.join("cgroup"), cgroup).unwrap();
        }
    }

    /// The mount a fake process's descriptor 3 is on, in its fdinfo, and
    /// the process's mountinfo saying that mount is of the kind given, or
    /// no mountinfo when there's none. A devtmpfs mount whose id starts
    /// with the same digits comes first, so only the whole id finds it.
    fn fake_mount(root: &Path, pid: u32, kind: Option<&str>) {
        let proc = root.join(pid.to_string());
        fs::create_dir_all(proc.join("fdinfo")).unwrap();
        let fdinfo = "pos:\t0\nflags:\t0100000\nmnt_id:\t42\n";
        fs::write(proc.join("fdinfo/3"), fdinfo).unwrap();
        if let Some(kind) = kind {
            let mounts = format!(
                "421 1 0:5 / /dev rw,nosuid shared:2 - devtmpfs devtmpfs rw,mode=755\n\
                 42 421 0:5 /video0 /home/someone/v rw,nosuid shared:2 - {kind} {kind} rw\n"
            );
            fs::write(proc.join("mountinfo"), mounts).unwrap();
        }
    }

    /// A link outside /dev, in the fake folder, that leads to /dev/null, so
    /// following it finds the camera's device as a bind of its node would.
    fn bound(root: &Path) -> std::path::PathBuf {
        let outside = root.join("video0");
        std::os::unix::fs::symlink("/dev/null", &outside).unwrap();
        outside
    }

    /// What a walk of the fake folder names as holding /dev/null, with the
    /// stand-in unit the one left out. The folder is removed after, even
    /// when the walk fails. The walk follows descriptor links and compares
    /// devices, so /dev/null stands in for a camera and none is ever opened.
    fn walked(root: &Path) -> (BTreeSet<String>, HashSet<u32>) {
        walked_after(root, &HashSet::new())
    }

    /// What a walk of the fake folder names, as walked does, when the
    /// processes given held the camera at the walk before, so their maps
    /// are read too.
    fn walked_after(root: &Path, before: &HashSet<u32>) -> (BTreeSet<String>, HashSet<u32>) {
        let units = ["stand-in.service".to_owned()];
        let found = holders_in(root, Path::new("/dev/null"), &units, before);
        fs::remove_dir_all(root).unwrap();
        found.unwrap()
    }

    #[test]
    fn a_process_whose_cgroup_file_isnt_text_is_named() {
        // A person can name a cgroup of their own with a byte that isn't
        // text, and skipping its reader would show a camera in use as off.
        let root = fake_proc("not-text");
        let user = b"0::/user.slice/user-1000.slice/user@1000.service";
        let cgroup = [&user[..], b"/\xff/stand-in.service\n"].concat();
        fake_process(&root, 4_000_001, Path::new("/dev/null"), Some(&cgroup));
        let (names, pids) = walked(&root);
        assert_eq!(
            names,
            BTreeSet::from(["reader".to_owned()]),
            "a reader whose cgroup file isn't text is named"
        );
        assert_eq!(pids, HashSet::from([4_000_001]), "its process is seen");
    }

    #[test]
    fn a_process_with_no_cgroup_file_is_named() {
        let root = fake_proc("no-cgroup");
        fake_process(&root, 4_000_001, Path::new("/dev/null"), None);
        let (names, pids) = walked(&root);
        assert_eq!(
            names,
            BTreeSet::from(["reader".to_owned()]),
            "a reader whose cgroup file can't be read is named"
        );
        assert_eq!(pids, HashSet::from([4_000_001]), "its process is seen");
    }

    #[test]
    fn a_process_in_a_left_out_unit_is_not_named() {
        // A reader beside it shows the walk saw the folder, so the relay's
        // absence is its unit's doing.
        let root = fake_proc("left-out");
        let null = Path::new("/dev/null");
        let relay = b"0::/system.slice/stand-in.service\n";
        let reader = b"0::/user.slice/user-1000.slice\n";
        fake_process(&root, 4_000_001, null, Some(relay));
        fake_process(&root, 4_000_002, null, Some(reader));
        let (_, pids) = walked(&root);
        assert_eq!(
            pids,
            HashSet::from([4_000_002]),
            "only the process outside the stand-in unit is seen"
        );
    }

    #[test]
    fn a_node_bound_outside_dev_from_devtmpfs_is_named() {
        // A person can bind the camera's node anywhere in a mount namespace
        // of their own, and missing its reader would show a camera in use
        // as off.
        let root = fake_proc("bound");
        let outside = bound(&root);
        let cgroup = b"0::/user.slice/user-1000.slice\n";
        fake_process(&root, 4_000_001, &outside, Some(cgroup));
        fake_mount(&root, 4_000_001, Some("devtmpfs"));
        let (names, pids) = walked(&root);
        assert_eq!(
            names,
            BTreeSet::from(["reader".to_owned()]),
            "a reader of a node bound outside /dev is named"
        );
        assert_eq!(pids, HashSet::from([4_000_001]), "its process is seen");
    }

    #[test]
    fn a_link_outside_dev_on_another_kind_of_mount_is_not_named() {
        // The link leads to /dev/null, so the device alone would match; only
        // its mount's kind leaves it out. A reader beside it shows the walk
        // saw the folder.
        let root = fake_proc("other-mount");
        let outside = bound(&root);
        let cgroup = b"0::/user.slice/user-1000.slice\n";
        fake_process(&root, 4_000_001, &outside, Some(cgroup));
        fake_mount(&root, 4_000_001, Some("tmpfs"));
        fake_process(&root, 4_000_002, Path::new("/dev/null"), Some(cgroup));
        let (_, pids) = walked(&root);
        assert_eq!(
            pids,
            HashSet::from([4_000_002]),
            "only the process whose link is under /dev is seen"
        );
    }

    #[test]
    fn a_descriptor_whose_mount_cant_be_read_is_skipped_without_failing_the_walk() {
        // Each link leads to /dev/null, so following one would name its
        // process. The odd processes have no fdinfo, the even ones no
        // mountinfo; the ones that held the camera before still map it,
        // and are named only if skipping the link left their walk going.
        let root = fake_proc("unreadable-mount");
        let outside = bound(&root);
        let cgroup = b"0::/user.slice/user-1000.slice\n";
        let inode = fs::metadata("/dev/null").unwrap().ino();
        let mapped = format!("7f00-7f10 rw-s 00000000 00:06 {inode} /dev/null\n");
        for pid in 4_000_001..=4_000_004 {
            fake_process(&root, pid, &outside, Some(cgroup));
        }
        fake_mount(&root, 4_000_002, None);
        fake_mount(&root, 4_000_004, None);
        for pid in [4_000_003, 4_000_004] {
            fs::write(root.join(pid.to_string()).join("maps"), &mapped).unwrap();
        }
        let (_, pids) = walked_after(&root, &HashSet::from([4_000_003, 4_000_004]));
        assert_eq!(
            pids,
            HashSet::from([4_000_003, 4_000_004]),
            "only the processes that still map the camera are seen"
        );
    }
}
