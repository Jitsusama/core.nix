//! The apps Joel waits on at all times, not only in a call: which of
//! Chrome's and Slack's threads draw what he sees, found once for each new
//! process of an app he opened, and kept, so each window reads only those
//! threads' own files.
//!
//! What they waited goes into each window's `waits`, and from the history
//! into the hourly apps-under-load measure that cadence judges. No trouble
//! line reads them yet: a threshold waits on what that measure shows.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::kernel::{Thread, started, tasks, text};

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

/// The kind of process a command line names, from its flags alone: those
/// before the first word that isn't one, as a browser started for a link
/// has the link there, and a link may carry `--type=` of its own. Each
/// word is split at a NUL or a space, since Chromium rewrites a child's
/// command line into one string.
fn drawing(args: &str) -> Drawing {
    let mut words = args
        .split(|c: char| c == '\0' || c.is_ascii_whitespace())
        .filter(|w| !w.is_empty());
    // No command line is a process ending, or the kernel's.
    if words.next().is_none() {
        return Drawing::NotYet;
    }
    let kind = words
        .take_while(|w| w.starts_with('-') && *w != "--")
        .find_map(|w| w.strip_prefix("--type="));
    match kind {
        None => Drawing::With {
            main: true,
            drawer: None,
        },
        Some("zygote") => Drawing::NotYet,
        Some("renderer") => Drawing::With {
            main: true,
            drawer: Some("Compositor"),
        },
        Some("gpu-process") => Drawing::With {
            main: false,
            drawer: Some("VizCompositorTh"),
        },
        Some(_) => Drawing::Nothing,
    }
}

/// A process of Joel's apps looked into: when it started, which tells it
/// from a later process given its pid, and its threads worth reading.
#[derive(Debug)]
struct Known {
    started: Option<u64>,
    threads: Vec<Thread>,
}

/// The apps' processes found so far, by pid. One of Joel's is looked into,
/// its command line and its threads' names, only when it's new: its
/// threads are kept and only their schedstat and stat read after, with its
/// own stat once a minute to see it's still the same process. One is
/// looked into again while it lacks what its kind draws with, as a
/// renderer does in the moment before it starts its compositor, or while
/// it says it's a zygote, a few files a minute for each. A process outside
/// his apps has only its cgroup read, each minute, since one starts in its
/// launcher's cgroup in the moment before it's moved into its scope.
#[derive(Debug)]
pub struct Apps {
    slice: String,
    known: BTreeMap<String, Known>,
}

impl Apps {
    /// The apps of the account whose manager is at `account` in the
    /// cgroup tree, such as `/user.slice/user-1000.slice/user@1000.service`.
    pub fn of_account(account: &str) -> Self {
        Apps {
            slice: format!("{account}/app.slice"),
            known: BTreeMap::new(),
        }
    }

    /// The threads of `process`, an app's labelled `label`, from what was
    /// found when it was new, or found now; none when it isn't one Joel
    /// opened.
    pub fn of(&mut self, pid: &str, process: &Path, label: &'static str) -> Option<Vec<Thread>> {
        let started = started(&text(&process.join("stat")));
        if let Some(known) = self.known.get(pid).filter(|k| k.started == started) {
            return Some(known.threads.clone());
        }
        self.known.remove(pid);
        if !self.launched_by_joel(&text(&process.join("cgroup"))) {
            return None;
        }
        let mut settle = |threads: Vec<Thread>| {
            self.known.insert(
                pid.to_string(),
                Known {
                    started,
                    threads: threads.clone(),
                },
            );
            Some(threads)
        };
        let (main, drawer) = match drawing(&text(&process.join("cmdline"))) {
            Drawing::With { main, drawer } => (main, drawer),
            Drawing::Nothing => return settle(Vec::new()),
            Drawing::NotYet => return Some(Vec::new()),
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
                return Some(found);
            }
        }
        if found.is_empty() {
            return Some(found);
        }
        settle(found)
    }

    /// Lets go of every process no longer running.
    pub fn keep_only(&mut self, running: &BTreeSet<String>) {
        self.known.retain(|pid, _| running.contains(pid));
    }

    /// Whether a process is one Joel opened, from its cgroup: a unit
    /// directly in his own `app.slice`, as systemd names those he starts.
    /// One niri, gio or the portal launches is in an `app-*.scope`, an
    /// autostarted one in an `app-*.service`, and one from his terminal in
    /// its `kitty-*.scope`; their children stay there. Another account's
    /// browser, an agent's, or a test's Chrome runs elsewhere, or in a
    /// service of its own.
    fn launched_by_joel(&self, cgroup: &str) -> bool {
        cgroup.lines().any(|line| {
            let Some(unit) = line
                .strip_prefix("0::")
                .and_then(|path| path.strip_prefix(self.slice.as_str()))
                .and_then(|rest| rest.strip_prefix('/'))
                .filter(|unit| !unit.contains('/'))
            else {
                return false;
            };
            match unit.rsplit_once('.') {
                Some((name, "scope")) => name.starts_with("app-") || name.starts_with("kitty-"),
                Some((name, "service")) => name.starts_with("app-"),
                _ => false,
            }
        })
    }
}

/// Whether a thread is an app's, read with its nice: a tab the app lowered
/// is one nobody is looking at, and is left out by it.
pub fn is_app(thread: &Thread) -> bool {
    APPS.iter().any(|(_, label)| *label == thread.label)
}

/// The folder in /proc of the process an app's thread is in, from its
/// schedstat's path, `<process>/task/<tid>/schedstat`.
pub fn process(thread: &Thread) -> Option<&Path> {
    thread.schedstat.parent()?.parent()?.parent()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::tests::{ACCOUNT, opened, write};
    use crate::kernel::threads;
    use std::fs;

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

    /// A stat for a process that started at `start`.
    fn stat(pid: &str, start: u64) -> String {
        format!("{pid} (chrome) S 1 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0 {start}\n")
    }

    #[test]
    fn an_app_s_drawing_threads_are_found_by_the_kind_of_its_process() {
        let root = std::env::temp_dir().join(format!("lag-watch-apps-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        // The browser, which takes Joel's input on its main thread, started
        // for a link that names a kind of its own, after a space.
        opened(&proc, "70", "chrome");
        write(
            &proc.join("70/cmdline"),
            "/opt/google/chrome/chrome\0--enable-features=A\0https://a.test/a --type=zygote\0",
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
            &format!("0::{ACCOUNT}/app.slice/browser.service\n"),
        );
        write(&proc.join("92/cmdline"), "chrome\0--headless\0");
        write(&proc.join("92/task/92/comm"), "chrome\n");
        // Neither an app nor a call: nothing of it is read.
        write(&proc.join("90/comm"), "bash\n");
        assert_eq!(
            drawing_threads(&proc, &mut Apps::of_account(ACCOUNT)),
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
    fn a_process_is_joel_s_only_in_a_unit_he_started_in_his_own_app_slice() {
        let apps = Apps::of_account(ACCOUNT);
        let joels = |path: &str| apps.launched_by_joel(&format!("0::{path}\n"));
        let mine = format!("{ACCOUNT}/app.slice");
        for unit in [
            "app-niri-google\\x2dchrome\\x2dstable-70.scope",
            "app-gio-chrome-70.scope",
            "app-slack@autostart.service",
            "kitty-70-0.scope",
        ] {
            assert!(joels(&format!("{mine}/{unit}")), "{unit} is Joel's");
        }
        let theirs = "/user.slice/user-1001.slice/user@1001.service/app.slice";
        for path in [
            // A test's Chrome, and a program's own service.
            format!("{mine}/apps-under-load.service"),
            format!("{mine}/tuple.service"),
            // An agent's session, and an agent's Chrome in the work slices.
            format!("{mine}/mycelium-pi-a.scope"),
            format!("{ACCOUNT}/jobs.slice/jobs-work.slice/work-1-2.scope"),
            // An app in a slice of its own, nested below his.
            format!("{mine}/app-graphical.slice/app-niri-a-70.scope"),
            // Another account's app, and one in a container's account.
            format!("{theirs}/app-niri-a-70.scope"),
            format!("/machine.slice/a.scope/payload{mine}/app-niri-a-70.scope"),
            // A process ended, its cgroup unread.
            String::new(),
        ] {
            assert!(!joels(&path), "{path} isn't Joel's");
        }
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
        // A utility process, and a browser still in its launcher's cgroup.
        opened(&proc, "77", "chrome");
        write(&proc.join("77/cmdline"), "chrome --type=utility");
        write(&proc.join("77/task/77/comm"), "chrome\n");
        write(&proc.join("86/comm"), "chrome\n");
        write(
            &proc.join("86/cgroup"),
            &format!("0::{ACCOUNT}/session.slice/niri.service\n"),
        );
        write(&proc.join("86/cmdline"), "chrome\0--enable-features=A\0");
        write(&proc.join("86/task/86/comm"), "chrome\n");
        let mut apps = Apps::of_account(ACCOUNT);
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
                "chrome-ui chrome 82/task/82/schedstat",
            ]
        );
        // A settled renderer isn't looked into again: a thread it starts
        // later under the same name isn't read, though a new one's is. The
        // utility process stays settled as drawing nothing, and the browser
        // moved into its scope is found.
        write(&proc.join("72/task/74/comm"), "Compositor\n");
        write(&proc.join("82/task/83/comm"), "Compositor\n");
        write(&proc.join("84/cmdline"), "chrome --type=renderer");
        write(&proc.join("84/task/85/comm"), "Compositor\n");
        write(&proc.join("77/cmdline"), "chrome --type=renderer");
        write(&proc.join("77/task/78/comm"), "Compositor\n");
        opened(&proc, "86", "chrome");
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui Compositor 82/task/83/schedstat",
                "chrome-ui Compositor 84/task/85/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
                "chrome-ui chrome 82/task/82/schedstat",
                "chrome-ui chrome 84/task/84/schedstat",
                "chrome-ui chrome 86/task/86/schedstat",
            ]
        );
    }

    #[test]
    fn a_pid_used_again_is_looked_into_as_a_new_process() {
        let root = std::env::temp_dir().join(format!("lag-watch-reused-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        opened(&proc, "72", "chrome");
        write(&proc.join("72/stat"), &stat("72", 100));
        write(&proc.join("72/cmdline"), "chrome --type=renderer");
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        let mut apps = Apps::of_account(ACCOUNT);
        assert_eq!(drawing_threads(&proc, &mut apps).len(), 2);
        // Between two looks the renderer ends and its pid goes to a utility
        // process, which draws nothing.
        write(&proc.join("72/stat"), &stat("72", 200));
        write(&proc.join("72/cmdline"), "chrome --type=utility");
        assert_eq!(drawing_threads(&proc, &mut apps), Vec::<String>::new());
        // Then, gone at a look, to a browser, whose main thread alone is
        // read.
        fs::remove_dir_all(proc.join("72")).unwrap();
        assert_eq!(drawing_threads(&proc, &mut apps), Vec::<String>::new());
        opened(&proc, "72", "chrome");
        write(&proc.join("72/stat"), &stat("72", 300));
        write(&proc.join("72/cmdline"), "chrome\0--enable-features=A\0");
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            ["chrome-ui chrome 72/task/72/schedstat"]
        );
    }
}
