//! How much work a build's pieces have done, so a build that says nothing
//! can be told from one that is stuck. A compile or a link may print
//! nothing for many minutes while it works: on 9 November a switch was
//! stopped 84 minutes in while ONNX Runtime compiled quietly. What the
//! pieces do is found from the nix in a build's process group. nix-daemon
//! gives each client a worker named `nix-daemon <client's pid>`, and the
//! worker's children are the pieces: a sandboxed builder in a cgroup of
//! its own when it builds here, or `nix __build-remote` with an ssh to
//! the builder when it builds elsewhere. A piece here counts its cgroup's
//! CPU time and bytes written. A piece elsewhere counts its builder's,
//! which nix-offload reads there into the builder's `health.json`.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Where the figures are read from: the machine's own, or a stand-in's.
#[derive(Clone, Debug)]
pub struct Roots {
    pub proc: PathBuf,
    pub cgroups: PathBuf,
    pub run: PathBuf,
}

impl Roots {
    pub fn machine() -> Roots {
        Roots {
            proc: PathBuf::from("/proc"),
            cgroups: PathBuf::from("/sys/fs/cgroup"),
            run: PathBuf::from("/run"),
        }
    }
}

/// A process as `/proc` gives it.
struct Process {
    parent: i32,
    group: i32,
    args: Vec<String>,
}

/// The work done so far by the pieces nix builds for the process group
/// `group`, as one figure that grows while any of them moves: CPU
/// microseconds and bytes written, here and on each builder in use.
/// It means nothing alone; two readings apart say whether they moved.
pub fn done(group: i32, roots: &Roots) -> u64 {
    let processes = processes(&roots.proc);
    let clients: BTreeSet<i32> = processes
        .iter()
        .filter(|(_, process)| process.group == group)
        .map(|(pid, _)| *pid)
        .collect();
    let workers: BTreeSet<i32> = processes
        .iter()
        .filter(|(_, process)| {
            process.args.first().map(String::as_str) == Some("nix-daemon")
                && process
                    .args
                    .get(1)
                    .and_then(|client| client.parse().ok())
                    .is_some_and(|client| clients.contains(&client))
        })
        .map(|(pid, _)| *pid)
        .collect();
    let mut cgroups = BTreeSet::new();
    let mut builders = BTreeSet::new();
    for (pid, process) in &processes {
        if workers.contains(&process.parent)
            && let Some(cgroup) = cgroup(&roots.proc, *pid)
        {
            cgroups.insert(cgroup);
        }
        if process.args.first().map(String::as_str) == Some("ssh")
            && processes
                .get(&process.parent)
                .is_some_and(|remote| workers.contains(&remote.parent))
            && let Some(host) = process.args.get(1)
        {
            builders.insert(host.clone());
        }
    }
    let here: u64 = cgroups
        .iter()
        .filter(|cgroup| cgroup.contains("/nix-build-"))
        .map(|cgroup| worked(&roots.cgroups.join(cgroup.trim_start_matches('/'))))
        .sum();
    let elsewhere: u64 = builders
        .iter()
        .filter(|host| !host.contains('/') && !host.starts_with('.'))
        .map(|host| builder_worked(&roots.run.join(host).join("health.json")))
        .sum();
    here + elsewhere
}

fn processes(proc: &Path) -> HashMap<i32, Process> {
    let Ok(entries) = fs::read_dir(proc) else {
        return HashMap::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid: i32 = entry.file_name().to_str()?.parse().ok()?;
            let stat = fs::read_to_string(entry.path().join("stat")).ok()?;
            // The name, in parentheses, may hold spaces and parentheses
            // of its own, so the fields are counted from its last `)`.
            let (_, rest) = stat.rsplit_once(')')?;
            let mut fields = rest.split_whitespace().skip(1);
            let parent = fields.next()?.parse().ok()?;
            let group = fields.next()?.parse().ok()?;
            let args = fs::read(entry.path().join("cmdline"))
                .ok()?
                .split(|byte| *byte == 0)
                .filter(|arg| !arg.is_empty())
                .map(|arg| String::from_utf8_lossy(arg).into_owned())
                .collect();
            Some((
                pid,
                Process {
                    parent,
                    group,
                    args,
                },
            ))
        })
        .collect()
}

/// A process's cgroup, from the unified hierarchy's `0::` line.
fn cgroup(proc: &Path, pid: i32) -> Option<String> {
    fs::read_to_string(proc.join(pid.to_string()).join("cgroup"))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("0::").map(str::to_owned))
}

/// A cgroup's CPU microseconds and bytes written.
fn worked(cgroup: &Path) -> u64 {
    let cpu = fs::read_to_string(cgroup.join("cpu.stat"))
        .ok()
        .and_then(|stat| {
            stat.lines()
                .find_map(|line| line.strip_prefix("usage_usec "))
                .and_then(|usec| usec.trim().parse().ok())
        })
        .unwrap_or(0);
    let written: u64 = fs::read_to_string(cgroup.join("io.stat"))
        .map(|stat| {
            stat.split_whitespace()
                .filter_map(|field| field.strip_prefix("wbytes="))
                .filter_map(|bytes| bytes.parse::<u64>().ok())
                .sum()
        })
        .unwrap_or(0);
    cpu + written
}

/// A builder's CPU microseconds and bytes written by its builds, as
/// nix-offload reads them there into `builds`.
fn builder_worked(health: &Path) -> u64 {
    let Some(health) = fs::read_to_string(health)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
    else {
        return 0;
    };
    let builds = &health["builds"];
    ["cpuUsec", "writtenBytes"]
        .iter()
        .filter_map(|figure| builds[figure].as_u64())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Machine {
        dir: tempfile::TempDir,
    }

    impl Machine {
        fn new() -> Machine {
            Machine {
                dir: tempfile::tempdir().unwrap(),
            }
        }

        fn roots(&self) -> Roots {
            Roots {
                proc: self.dir.path().join("proc"),
                cgroups: self.dir.path().join("cgroup"),
                run: self.dir.path().join("run"),
            }
        }

        fn process(&self, pid: i32, parent: i32, group: i32, args: &[&str], cgroup: &str) {
            let at = self.dir.path().join("proc").join(pid.to_string());
            fs::create_dir_all(&at).unwrap();
            fs::write(
                at.join("stat"),
                format!("{pid} (a (b) c) S {parent} {group} {group} 0 -1"),
            )
            .unwrap();
            fs::write(at.join("cmdline"), format!("{}\0", args.join("\0"))).unwrap();
            fs::write(at.join("cgroup"), format!("0::{cgroup}\n")).unwrap();
        }

        fn cgroup(&self, cgroup: &str, usec: u64, written: u64) {
            let at = self.dir.path().join("cgroup").join(cgroup);
            fs::create_dir_all(&at).unwrap();
            fs::write(
                at.join("cpu.stat"),
                format!("usage_usec {usec}\nuser_usec 1\n"),
            )
            .unwrap();
            fs::write(
                at.join("io.stat"),
                format!("259:0 rbytes=7 wbytes={written} rios=1 wios=1\n"),
            )
            .unwrap();
        }

        fn builder(&self, host: &str, health: &str) {
            let at = self.dir.path().join("run").join(host);
            fs::create_dir_all(&at).unwrap();
            fs::write(at.join("health.json"), health).unwrap();
        }
    }

    const DAEMON: &str = "/build.slice/nix-daemon.service/nix-daemon";

    #[test]
    fn a_build_s_pieces_here_count_what_their_cgroups_did() {
        let machine = Machine::new();
        machine.process(100, 1, 100, &["nix", "build", "-L"], "/user");
        machine.process(200, 954, 954, &["nix-daemon", "100"], DAEMON);
        let piece = "build.slice/nix-daemon.service/nix-build-uid-7";
        machine.process(
            300,
            200,
            300,
            &["bash", "-e", "builder.sh"],
            &format!("/{piece}"),
        );
        machine.cgroup(piece, 1000, 20);
        // Another client's piece isn't this build's.
        machine.process(101, 1, 101, &["nix", "build"], "/user");
        machine.process(201, 954, 954, &["nix-daemon", "101"], DAEMON);
        let other = "build.slice/nix-daemon.service/nix-build-uid-8";
        machine.process(301, 201, 301, &["bash"], &format!("/{other}"));
        machine.cgroup(other, 50_000, 0);
        assert_eq!(done(100, &machine.roots()), 1020);
        machine.cgroup(piece, 4000, 20);
        assert_eq!(done(100, &machine.roots()), 4020);
    }

    #[test]
    fn a_build_s_pieces_elsewhere_count_their_builder_s_builds() {
        let machine = Machine::new();
        machine.process(100, 1, 100, &["nix", "build", "-L"], "/user");
        machine.process(200, 954, 954, &["nix-daemon", "100"], DAEMON);
        machine.process(210, 200, 954, &["nix", "__build-remote", "3"], DAEMON);
        machine.process(220, 210, 954, &["ssh", "nix-offload-more", "-x"], DAEMON);
        machine.builder(
            "nix-offload-more",
            r#"{"state":"passing","builds":{"cpuUsec":900,"writtenBytes":100,"at":1}}"#,
        );
        // A builder whose health says nothing of its builds counts none.
        machine.process(211, 200, 954, &["nix", "__build-remote", "4"], DAEMON);
        machine.process(221, 211, 954, &["ssh", "nix-offload-vm", "-x"], DAEMON);
        machine.builder("nix-offload-vm", r#"{"state":"passing"}"#);
        assert_eq!(done(100, &machine.roots()), 1000);
    }

    #[test]
    fn a_build_waiting_on_nothing_it_can_see_has_done_nothing() {
        let machine = Machine::new();
        machine.process(100, 1, 100, &["nix", "build", "-L"], "/user");
        assert_eq!(done(100, &machine.roots()), 0);
        assert_eq!(
            done(
                100,
                &Roots {
                    proc: machine.dir.path().join("none"),
                    ..machine.roots()
                }
            ),
            0
        );
    }
}
