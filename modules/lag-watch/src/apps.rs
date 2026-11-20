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
/// process its threads' names say it is, as Chromium names them: the
/// browser, the one process with a `Chrome_IOThread`, whose main thread
/// takes his input; a renderer, with a `Compositor`, whose main thread and
/// compositor lay out and draw a page; and the GPU process, with a
/// `VizCompositorThread` cut to 15 bytes, which puts every frame on the
/// screen. A main thread keeps the process's name. Any other draws
/// nothing: a utility process, with only a `Chrome_ChildIOThread`, or a
/// zygote, with none. A renderer just forked has none of them either for
/// a moment, so one is settled as drawing nothing only when it still has
/// none a look later.
///
/// The names are read, not the command line, which takes the process's
/// memory lock to read, and so could hold the watch behind a page that's
/// faulting under the very pressure the watch records.
#[derive(Debug, PartialEq, Eq)]
enum Drawing {
    With {
        main: bool,
        drawer: Option<&'static str>,
    },
    Unsure,
}

/// The kind of process its threads, by name, say it is.
fn drawing(threads: &[(String, PathBuf)]) -> Drawing {
    let has = |kind: &str| threads.iter().any(|(name, _)| name == kind);
    if has("VizCompositorTh") {
        Drawing::With {
            main: false,
            drawer: Some("VizCompositorTh"),
        }
    } else if has("Compositor") {
        Drawing::With {
            main: true,
            drawer: Some("Compositor"),
        }
    } else if has("Chrome_IOThread") {
        Drawing::With {
            main: true,
            drawer: None,
        }
    } else {
        Drawing::Unsure
    }
}

/// A process of Joel's apps looked into: when it started, which tells it
/// from a later process given its pid; its threads worth reading; and
/// whether that's settled, or it named none of its kind once and is looked
/// into a second time.
#[derive(Debug)]
struct Known {
    started: Option<u64>,
    threads: Vec<Thread>,
    settled: bool,
}

/// The apps' processes found so far, by pid. One of Joel's is looked into,
/// its threads' names, only when it's new: its
/// threads are kept and only their schedstat and stat read after. Each
/// minute its stat and cgroup are read again, to see it's still the same
/// process and still Joel's. One is looked into again while it lacks what
/// its kind draws with, as a renderer does in the moment before it starts
/// its compositor. A process outside his apps has only its cgroup read,
/// each minute, since one starts in its launcher's cgroup in the moment
/// before it's moved into its scope.
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
        if !self.launched_by_joel(&text(&process.join("cgroup"))) {
            self.known.remove(pid);
            return None;
        }
        let mut unsure_before = false;
        match self.known.get(pid) {
            Some(known) if known.started == started && known.settled => {
                return Some(known.threads.clone());
            }
            Some(known) => unsure_before = known.started == started,
            None => {}
        }
        let mut keep = |threads: Vec<Thread>, settled: bool| {
            self.known.insert(
                pid.to_string(),
                Known {
                    started,
                    threads: threads.clone(),
                    settled,
                },
            );
            Some(threads)
        };
        let threads = tasks(process);
        let (main, drawer) = match drawing(&threads) {
            Drawing::With { main, drawer } => (main, drawer),
            // One that names none of its kind a look later draws nothing.
            Drawing::Unsure => return keep(Vec::new(), unsure_before),
        };
        let main_thread = process.join("task").join(pid).join("schedstat");
        let found: Vec<Thread> = threads
            .into_iter()
            .filter(|(name, schedstat)| {
                (main && *schedstat == main_thread) || drawer == Some(name.as_str())
            })
            .map(|(name, schedstat)| Thread {
                label,
                name,
                schedstat,
                call: false,
            })
            .collect();
        // Its main thread unread, as when it's ending, it's looked into again.
        if main && !found.iter().any(|t| t.schedstat == main_thread) {
            return Some(found);
        }
        keep(found, true)
    }

    /// Lets go of every process no longer running.
    pub fn keep_only(&mut self, running: &BTreeSet<String>) {
        self.known.retain(|pid, _| running.contains(pid));
    }

    /// Whether a process is one Joel opened, from its cgroup: an app's unit
    /// in his own `app.slice`, as systemd names those a desktop starts, and
    /// which their children stay in. One niri, gio or the portal launches
    /// is in an `app-*.scope`, an autostarted one in an `app-*.service`;
    /// a launcher may group them in an `app-*.slice` first, as uwsm does.
    /// Another account's browser, an agent's, a test's Chrome, or one from
    /// his terminal, which jobs-sorter moves to the jobs slice once it's
    /// busy, runs elsewhere or in a unit of its own.
    pub fn launched_by_joel(&self, cgroup: &str) -> bool {
        cgroup.lines().any(|line| {
            let Some(rest) = line
                .strip_prefix("0::")
                .and_then(|path| path.strip_prefix(self.slice.as_str()))
                .and_then(|rest| rest.strip_prefix('/'))
            else {
                return false;
            };
            let unit = match rest.split_once('/') {
                None => rest,
                Some((slice, unit))
                    if slice.starts_with("app-")
                        && slice.rsplit_once('.').is_some_and(|(_, k)| k == "slice")
                        && !unit.contains('/') =>
                {
                    unit
                }
                Some(_) => return false,
            };
            match unit.rsplit_once('.') {
                Some((name, "scope" | "service")) => name.starts_with("app-"),
                _ => false,
            }
        })
    }

    /// Whether a Chrome in `cgroup` may carry Joel's call: one of his apps,
    /// or one he started from a terminal, in its kitty scope. A call is his
    /// however he opened the browser, so this is looser than the drawing
    /// test, which a terminal's work fails since jobs-sorter moves it once
    /// it's busy. A test's or an agent's Chrome, whose fake microphone would
    /// read as a call, is in neither. One jobs-sorter has moved into the
    /// jobs slice can't be told from an agent's there, and is left out.
    pub fn may_call(&self, cgroup: &str) -> bool {
        self.launched_by_joel(cgroup)
            || cgroup.lines().any(|line| {
                line.strip_prefix("0::")
                    .and_then(|path| path.strip_prefix(self.slice.as_str()))
                    .and_then(|rest| rest.strip_prefix('/'))
                    .is_some_and(|unit| {
                        unit.starts_with("kitty-")
                            && unit.rsplit_once('.').is_some_and(|(_, k)| k == "scope")
                    })
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
        // The browser, which takes Joel's input on its main thread, and
        // draws its tiles on threads whose names only start alike.
        opened(&proc, "70", "chrome");
        write(&proc.join("70/task/70/comm"), "chrome\n");
        write(&proc.join("70/task/71/comm"), "Chrome_IOThread\n");
        write(&proc.join("70/task/79/comm"), "CompositorTileW\n");
        // A renderer.
        opened(&proc, "72", "chrome");
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        write(&proc.join("72/task/74/comm"), "Chrome_ChildIOT\n");
        // The GPU process: its compositor, not its main thread.
        opened(&proc, "75", "chrome");
        write(&proc.join("75/task/75/comm"), "chrome\n");
        write(&proc.join("75/task/76/comm"), "VizCompositorTh\n");
        write(&proc.join("75/task/78/comm"), "Chrome_ChildIOT\n");
        // A utility process draws nothing.
        opened(&proc, "77", "chrome");
        write(&proc.join("77/task/77/comm"), "chrome\n");
        write(&proc.join("77/task/87/comm"), "Chrome_ChildIOT\n");
        // Slack's renderer, Electron's, named as Chromium names them.
        opened(&proc, "80", "slack");
        write(&proc.join("80/task/80/comm"), "slack\n");
        write(&proc.join("80/task/81/comm"), "Compositor\n");
        // Another session's browser, in a service of its own, isn't Joel's.
        write(&proc.join("92/comm"), "chrome\n");
        write(
            &proc.join("92/cgroup"),
            &format!("0::{ACCOUNT}/app.slice/browser.service\n"),
        );
        write(&proc.join("92/task/92/comm"), "chrome\n");
        write(&proc.join("92/task/93/comm"), "Chrome_IOThread\n");
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
            // An app a launcher grouped in a slice of its own, as uwsm does.
            "app-graphical.slice/app-niri-a-70.scope",
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
            // One from his terminal, which jobs-sorter moves once it's busy.
            format!("{mine}/kitty-70-0.scope"),
            // A unit in a slice that isn't an app's, or two slices down.
            format!("{mine}/tests.slice/app-niri-a-70.scope"),
            format!("{mine}/app-a.slice/app-b.slice/app-niri-a-70.scope"),
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
    fn a_chrome_carries_joel_s_call_from_his_apps_and_his_own_terminals_alone() {
        let apps = Apps::of_account(ACCOUNT);
        let calls = |path: &str| apps.may_call(&format!("0::{path}\n"));
        let mine = format!("{ACCOUNT}/app.slice");
        for unit in ["app-niri-a-70.scope", "kitty-70-0.scope"] {
            assert!(
                calls(&format!("{mine}/{unit}")),
                "{unit} carries Joel's call"
            );
        }
        for path in [
            // A terminal an agent's session opened, and one in a slice below.
            format!("{ACCOUNT}/agents.slice/kitty-70-0.scope"),
            format!("{mine}/tests.slice/kitty-70-0.scope"),
            // A test's Chrome, and another account's terminal.
            format!("{mine}/apps-under-load.service"),
            "/user.slice/user-1001.slice/user@1001.service/app.slice/kitty-70-0.scope".into(),
        ] {
            assert!(!calls(&path), "{path} carries no call of Joel's");
        }
    }

    #[test]
    fn an_app_s_process_is_looked_into_only_while_it_is_new() {
        let root = std::env::temp_dir().join(format!("lag-watch-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        opened(&proc, "72", "chrome");
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        // A renderer just forked, before it has started its compositor,
        // and a zygote, which never does.
        opened(&proc, "82", "chrome");
        write(&proc.join("82/task/82/comm"), "chrome\n");
        opened(&proc, "88", "chrome");
        write(&proc.join("88/task/88/comm"), "chrome\n");
        // A renderer whose main thread's name didn't read.
        opened(&proc, "94", "chrome");
        write(&proc.join("94/task/95/comm"), "Compositor\n");
        // A utility process, and a browser still in its launcher's cgroup.
        opened(&proc, "77", "chrome");
        write(&proc.join("77/task/77/comm"), "chrome\n");
        write(&proc.join("77/task/78/comm"), "Chrome_ChildIOT\n");
        write(&proc.join("86/comm"), "chrome\n");
        write(
            &proc.join("86/cgroup"),
            &format!("0::{ACCOUNT}/session.slice/niri.service\n"),
        );
        write(&proc.join("86/task/86/comm"), "chrome\n");
        write(&proc.join("86/task/87/comm"), "Chrome_IOThread\n");
        let mut apps = Apps::of_account(ACCOUNT);
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui Compositor 94/task/95/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
            ]
        );
        // A settled renderer isn't looked into again: a thread it starts
        // later under the same name isn't read, though a new one's is, and
        // the browser moved into its scope is found.
        write(&proc.join("72/task/74/comm"), "Compositor\n");
        write(&proc.join("82/task/83/comm"), "Compositor\n");
        write(&proc.join("94/task/94/comm"), "chrome\n");
        opened(&proc, "86", "chrome");
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui Compositor 82/task/83/schedstat",
                "chrome-ui Compositor 94/task/95/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
                "chrome-ui chrome 82/task/82/schedstat",
                "chrome-ui chrome 86/task/86/schedstat",
                "chrome-ui chrome 94/task/94/schedstat",
            ]
        );
        // The zygote and the utility process, naming none of their kind
        // at two looks, are settled as drawing nothing, and not looked
        // into again.
        write(&proc.join("88/task/89/comm"), "Compositor\n");
        write(&proc.join("77/task/79/comm"), "Compositor\n");
        let third = drawing_threads(&proc, &mut apps);
        assert!(
            !third
                .iter()
                .any(|t| t.contains(" 88/") || t.contains(" 77/")),
            "a settled process was looked into again: {third:?}"
        );
        // One of Joel's moved out of his apps, as jobs-sorter moves a busy
        // group, is let go at the next look.
        write(
            &proc.join("72/cgroup"),
            &format!("0::{ACCOUNT}/jobs.slice/jobs-work.slice/work-1-2.scope\n"),
        );
        let moved = drawing_threads(&proc, &mut apps);
        assert!(
            !moved.iter().any(|t| t.contains(" 72/")),
            "a process moved out of Joel's apps was still read: {moved:?}"
        );
    }

    #[test]
    fn a_pid_used_again_is_looked_into_as_a_new_process() {
        let root = std::env::temp_dir().join(format!("lag-watch-reused-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        opened(&proc, "72", "chrome");
        write(&proc.join("72/stat"), &stat("72", 100));
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Compositor\n");
        let mut apps = Apps::of_account(ACCOUNT);
        assert_eq!(drawing_threads(&proc, &mut apps).len(), 2);
        // Between two looks the renderer ends and its pid goes to a utility
        // process, which draws nothing.
        write(&proc.join("72/stat"), &stat("72", 200));
        write(&proc.join("72/task/73/comm"), "Chrome_ChildIOT\n");
        assert_eq!(drawing_threads(&proc, &mut apps), Vec::<String>::new());
        // Then, gone at a look and let go, to a browser, whose main thread
        // alone is read.
        fs::remove_dir_all(proc.join("72")).unwrap();
        assert_eq!(drawing_threads(&proc, &mut apps), Vec::<String>::new());
        assert_eq!(apps.known.len(), 0, "an ended process is let go");
        opened(&proc, "72", "chrome");
        write(&proc.join("72/stat"), &stat("72", 300));
        write(&proc.join("72/task/72/comm"), "chrome\n");
        write(&proc.join("72/task/73/comm"), "Chrome_IOThread\n");
        write(&proc.join("72/task/74/comm"), "CompositorTileW\n");
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            ["chrome-ui chrome 72/task/72/schedstat"]
        );
    }
}
