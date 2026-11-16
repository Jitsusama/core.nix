//! Which process holds a socket, found by its inode among /proc's open
//! files. Only this user's processes can be read, so a socket of root's or
//! another user's has no owner here and is told apart by its uid alone.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A process holding a socket: what it's called, how it was started and
/// where systemd placed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub cgroup: String,
}

/// Owners by socket inode, kept between windows, since a socket keeps its
/// owner for its life and walking /proc is the dear part of a sample.
#[derive(Debug, Default)]
pub struct Owners {
    known: HashMap<u32, Owner>,
}

impl Owners {
    /// The owner of each inode asked for, walking /proc once for any not
    /// known yet, and forgetting the inodes no longer asked for.
    pub fn of(&mut self, inodes: &[u32]) -> &HashMap<u32, Owner> {
        self.known.retain(|inode, _| inodes.contains(inode));
        if inodes.iter().any(|inode| !self.known.contains_key(inode)) {
            self.walk(Path::new("/proc"), inodes);
        }
        &self.known
    }

    fn walk(&mut self, proc: &Path, wanted: &[u32]) {
        let Ok(entries) = fs::read_dir(proc) else {
            return;
        };
        for entry in entries.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse().ok())
            else {
                continue;
            };
            // Another user's process, or one that ended mid-walk, can't be
            // read; its sockets keep no owner, which the reader says.
            let Ok(fds) = fs::read_dir(entry.path().join("fd")) else {
                continue;
            };
            let mut owner = None;
            for fd in fds.flatten() {
                let Some(inode) = fs::read_link(fd.path())
                    .ok()
                    .and_then(|target| socket_inode(&target.to_string_lossy()))
                else {
                    continue;
                };
                if wanted.contains(&inode) && !self.known.contains_key(&inode) {
                    let held = owner.get_or_insert_with(|| read(&entry.path(), pid));
                    self.known.insert(inode, held.clone());
                }
            }
        }
    }
}

/// The inode of an fd's link when it's a socket, `socket:[12345]`.
pub(super) fn socket_inode(target: &str) -> Option<u32> {
    target
        .strip_prefix("socket:[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

fn read(folder: &Path, pid: u32) -> Owner {
    let text = |name: &str| fs::read_to_string(folder.join(name)).unwrap_or_default();
    Owner {
        pid,
        name: text("comm").trim().to_owned(),
        command: text("cmdline").replace('\0', " ").trim().to_owned(),
        cgroup: cgroup_path(&text("cgroup")),
    }
}

/// The unified hierarchy's path from /proc/PID/cgroup, `0::/user.slice/...`.
pub(super) fn cgroup_path(said: &str) -> String {
    said.lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn a_socket_link_gives_its_inode_and_nothing_else_does() {
        assert_eq!(socket_inode("socket:[48213]"), Some(48213));
        assert_eq!(socket_inode("pipe:[48213]"), None);
        assert_eq!(socket_inode("/dev/null"), None);
    }

    #[test]
    fn the_cgroup_is_the_unified_line() {
        let said = "1:name=systemd:/old\n0::/user.slice/app.slice/app-x.scope\n";
        assert_eq!(cgroup_path(said), "/user.slice/app.slice/app-x.scope");
    }

    #[test]
    fn a_socket_this_process_holds_is_owned_by_it() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let fd = std::os::fd::AsRawFd::as_raw_fd(&listener);
        let inode = fs::metadata(format!("/proc/self/fd/{fd}")).unwrap().ino();
        let inode = u32::try_from(inode).unwrap();
        let mut owners = Owners::default();
        let found = owners.of(&[inode]);
        assert_eq!(found[&inode].pid, std::process::id());
    }
}
