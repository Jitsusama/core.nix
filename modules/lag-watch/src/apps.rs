//! The apps Joel waits on at all times, not only in a call: which of
//! Chrome's and Slack's threads draw what he sees, found once for each new
//! process of an app he opened, and kept, so each window reads only those
//! threads' own files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::kernel::{Thread, tasks, text};

/// The apps whose drawing is watched, by the name the kernel keeps for
/// their processes, each with its label. Slack is Electron, so its
/// processes are Chromium's, named alike.
pub const APPS: [(&str, &str); 2] = [("chrome", "chrome-ui"), ("slack", "slack-ui")];

/// The threads of an app's process that Joel waits on, by the kind of
/// process its command line says it is: the browser's main thread, which
/// takes his input; a renderer's main thread and its `Compositor`, which
/// lay out and draw a page; and the GPU process's `VizCompositorThread`,
/// cut to 15 bytes, which puts every frame on the screen. A main thread
/// keeps the process's name. A utility process draws nothing. A zygote
/// draws nothing either, but a renderer it has just forked says it is the
/// zygote until it names itself, so one is never taken as settled.
#[derive(Debug, PartialEq, Eq)]
enum Drawing {
    With {
        main: bool,
        drawer: Option<&'static str>,
    },
    Nothing,
    NotYet,
}

fn drawing(args: &str) -> Drawing {
    if args.is_empty() || args.contains("--type=zygote") {
        // No command line is a process ending, or the kernel's.
        Drawing::NotYet
    } else if !args.contains("--type=") {
        Drawing::With {
            main: true,
            drawer: None,
        }
    } else if args.contains("--type=renderer") {
        Drawing::With {
            main: true,
            drawer: Some("Compositor"),
        }
    } else if args.contains("--type=gpu-process") {
        Drawing::With {
            main: false,
            drawer: Some("VizCompositorTh"),
        }
    } else {
        Drawing::Nothing
    }
}

/// The apps' processes found so far, by pid, with the threads of each worth
/// reading. A process is looked into, its cgroup, command line and its
/// threads' names, only when it's new: its threads are kept and only their
/// schedstat and stat read after, so watching the apps always costs little.
/// One is looked into again only while it lacks what its kind draws with,
/// as a renderer does in the moment before it starts its compositor.
#[derive(Debug, Default)]
pub struct Apps {
    known: BTreeMap<String, Vec<Thread>>,
}

impl Apps {
    /// The threads of `process`, an app's labelled `label`, from what was
    /// found when it was new, or found now.
    pub fn of(&mut self, pid: &str, process: &Path, label: &'static str) -> Vec<Thread> {
        if let Some(found) = self.known.get(pid) {
            return found.clone();
        }
        if !launched_by_joel(&text(&process.join("cgroup"))) {
            self.known.insert(pid.to_string(), Vec::new());
            return Vec::new();
        }
        let (main, drawer) = match drawing(&text(&process.join("cmdline"))) {
            Drawing::With { main, drawer } => (main, drawer),
            Drawing::Nothing => {
                self.known.insert(pid.to_string(), Vec::new());
                return Vec::new();
            }
            Drawing::NotYet => return Vec::new(),
        };
        let thread = |name: String, schedstat: PathBuf| Thread {
            label,
            name,
            schedstat,
            call: false,
        };
        let mut found = Vec::new();
        if main {
            let at = process.join("task").join(pid);
            let name = text(&at.join("comm"));
            if let Some(name) = name.strip_suffix('\n').filter(|n| !n.is_empty()) {
                found.push(thread(name.to_string(), at.join("schedstat")));
            }
        }
        if let Some(drawer) = drawer {
            let before = found.len();
            found.extend(
                tasks(process)
                    .into_iter()
                    .filter(|(name, _)| name == drawer)
                    .map(|(name, schedstat)| thread(name, schedstat)),
            );
            if found.len() == before {
                return found;
            }
        }
        if found.is_empty() {
            return found;
        }
        self.known.insert(pid.to_string(), found.clone());
        found
    }

    /// Lets go of every process no longer running, so a pid used again is
    /// looked into as new.
    pub fn keep_only(&mut self, running: &BTreeSet<String>) {
        self.known.retain(|pid, _| running.contains(pid));
    }
}

/// Whether a process is one Joel opened: the scope systemd starts each app
/// he launches in, `app-*.scope` in his `app.slice`, and which its children
/// stay in. Another session's browser, or a test's Chrome, runs in a
/// service of its own and isn't what he waits on.
fn launched_by_joel(cgroup: &str) -> bool {
    cgroup.lines().any(|line| {
        let path = line.strip_prefix("0::").unwrap_or_default();
        let (parent, unit) = path.rsplit_once('/').unwrap_or_default();
        parent.ends_with("/app.slice")
            && unit
                .strip_suffix(".scope")
                .is_some_and(|unit| unit.starts_with("app-"))
    })
}

/// Whether a thread is an app's, read with its nice: a tab the app lowered
/// is one nobody is looking at, and is left out by it.
pub fn is_app(thread: &Thread) -> bool {
    APPS.iter().any(|(_, label)| *label == thread.label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::tests::write;
    use crate::kernel::threads;
    use std::fs;

    /// A process named `comm`, in the scope of an app Joel opened.
    fn opened(proc: &Path, pid: &str, comm: &str) {
        write(&proc.join(pid).join("comm"), &format!("{comm}\n"));
        let scope = "/user.slice/user-1000.slice/user@1000.service/app.slice/app-niri-a-70.scope";
        write(&proc.join(pid).join("cgroup"), &format!("0::{scope}\n"));
    }

    /// The apps' threads found, each as its label, name and schedstat path
    /// below `proc`.
    fn drawing_threads(proc: &Path, apps: &mut Apps) -> Vec<String> {
        threads(proc, None, &proc.join("none.service"), apps)
            .into_iter()
            .filter(is_app)
            .map(|t| {
                let at = t.schedstat.strip_prefix(proc).unwrap_or(&t.schedstat);
                format!("{} {} {}", t.label, t.name, at.display())
            })
            .collect()
    }

    #[test]
    fn an_app_s_drawing_threads_are_found_by_the_kind_of_its_process() {
        let root = std::env::temp_dir().join(format!("lag-watch-apps-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        // The browser, which takes Joel's input on its main thread.
        opened(&proc, "70", "chrome");
        write(
            &proc.join("70/cmdline"),
            "/opt/google/chrome/chrome\0--enable-features=A\0",
        );
        write(&proc.join("70/task/70/comm"), "chrome\n");
        write(&proc.join("70/task/71/comm"), "Chrome_IOThread\n");
        // A renderer, its command line rewritten into one string as
        // Chromium rewrites it.
        opened(&proc, "72", "chrome");
        write(
            &proc.join("72/cmdline"),
            "/opt/google/chrome/chrome --type=renderer --renderer-client-id=7",
        );
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        write(&proc.join("72/task/74/comm"), "ThreadPoolForeg\n");
        // The GPU process: its compositor, not its main thread.
        opened(&proc, "75", "chrome");
        write(&proc.join("75/cmdline"), "chrome\0--type=gpu-process\0");
        write(&proc.join("75/task/75/comm"), "chrome\n");
        write(&proc.join("75/task/76/comm"), "VizCompositorTh\n");
        // A utility process draws nothing.
        opened(&proc, "77", "chrome");
        write(&proc.join("77/cmdline"), "chrome\0--type=utility\0");
        write(&proc.join("77/task/77/comm"), "chrome\n");
        // Slack's renderer, Electron's, named as Chromium names them.
        opened(&proc, "80", "slack");
        write(
            &proc.join("80/cmdline"),
            "/opt/slack/slack\0--type=renderer\0",
        );
        write(&proc.join("80/task/80/comm"), "slack\n");
        write(&proc.join("80/task/81/comm"), "Compositor\n");
        // Another session's browser, in a service of its own, isn't Joel's.
        write(&proc.join("92/comm"), "chrome\n");
        write(
            &proc.join("92/cgroup"),
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/browser.service\n",
        );
        write(&proc.join("92/cmdline"), "chrome\0--headless\0");
        write(&proc.join("92/task/92/comm"), "chrome\n");
        // Neither an app nor a call: nothing of it is read.
        write(&proc.join("90/comm"), "bash\n");
        assert_eq!(
            drawing_threads(&proc, &mut Apps::default()),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui VizCompositorTh 75/task/76/schedstat",
                "chrome-ui chrome 70/task/70/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
                "slack-ui Compositor 80/task/81/schedstat",
                "slack-ui slack 80/task/80/schedstat",
            ]
        );
    }

    #[test]
    fn an_app_s_process_is_looked_into_only_while_it_is_new() {
        let root = std::env::temp_dir().join(format!("lag-watch-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        opened(&proc, "72", "chrome");
        write(&proc.join("72/cmdline"), "chrome --type=renderer");
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        // A renderer just forked, before it has started its compositor.
        opened(&proc, "82", "chrome");
        write(&proc.join("82/cmdline"), "chrome --type=renderer");
        write(&proc.join("82/task/82/comm"), "chrome\n");
        // One so new it still says it is the zygote that forked it.
        opened(&proc, "84", "chrome");
        write(&proc.join("84/cmdline"), "chrome --type=zygote");
        write(&proc.join("84/task/84/comm"), "chrome\n");
        let mut apps = Apps::default();
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
                "chrome-ui chrome 82/task/82/schedstat",
            ]
        );
        // A settled renderer isn't looked into again: a thread it starts
        // later under the same name isn't read, though a new one's is.
        write(&proc.join("72/task/74/comm"), "Compositor\n");
        write(&proc.join("82/task/83/comm"), "Compositor\n");
        write(&proc.join("84/cmdline"), "chrome --type=renderer");
        write(&proc.join("84/task/85/comm"), "Compositor\n");
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui Compositor 82/task/83/schedstat",
                "chrome-ui Compositor 84/task/85/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
                "chrome-ui chrome 82/task/82/schedstat",
                "chrome-ui chrome 84/task/84/schedstat",
            ]
        );
        // A pid let go when its process ends is looked into as new when
        // it's used again, here by the browser, whose main thread alone
        // is read.
        fs::remove_dir_all(proc.join("72")).unwrap();
        assert_eq!(drawing_threads(&proc, &mut apps).len(), 4);
        opened(&proc, "72", "chrome");
        write(&proc.join("72/cmdline"), "chrome\0--enable-features=A\0");
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        let again = drawing_threads(&proc, &mut apps);
        assert_eq!(
            again
                .iter()
                .filter(|t| t.contains(" 72/"))
                .collect::<Vec<_>>(),
            ["chrome-ui chrome 72/task/72/schedstat"]
        );
    }
}
