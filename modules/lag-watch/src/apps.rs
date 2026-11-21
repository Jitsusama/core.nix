//! The apps Joel waits on at all times, not only in a call: which of
//! Chrome's and Slack's threads draw what he sees, found once for each new
//! process of an app he opened, and kept, so each window reads only those
//! threads' own files.
//!
//! Long because more than half of it is tests, which replay each kind of
//! opener against one made-up /proc and read best beside the rules they pin.
//!
//! What they waited goes into each window's `waits`, and from the history
//! into the hourly apps-under-load measure that cadence judges. No trouble
//! line reads them yet: a threshold waits on what that measure shows.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::kernel::{Thread, parent, started, tasks, text};

/// The apps whose drawing is watched, by the name the kernel keeps for
/// their processes, each with its label. Slack is Electron, so its
/// processes are Chromium's, named alike.
pub const APPS: [(&str, &str); 2] = [("chrome", "chrome-ui"), ("slack", "slack-ui")];

/// What units in Joel's app.slice that aren't his are named from: the
/// hourly test call's, `call-under-load.service` and any run of it by
/// hand, whose Chrome stands in for his with a fake camera and microphone,
/// and the windows and sessions jobs-sorter finds agents in there, a
/// terminal's (`kitty-*`), which a pi may run in, and mycelium's
/// (`mycelium-pi-*`), beside its hosts' units (`pi-host-*`).
const NOT_HIS: [&str; 4] = ["call-under-load", "kitty-", "mycelium-pi-", "pi-host-"];

/// The threads of an app's process that Joel waits on, by the kind of
/// process its threads' names say it is, as Chromium names them: the
/// browser, the one process with a `Chrome_IOThread`, whose main thread
/// takes his input; a renderer, with a `Compositor`, whose main thread and
/// compositor lay out and draw a page; and the GPU process, with a
/// `VizCompositorThread` cut to 15 bytes, which puts every frame on the
/// screen, and whose main thread runs the GPU work the others ask for. A
/// main thread keeps the process's name. Any other draws
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
            main: true,
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
/// before it's moved into its scope. One started from his terminal, in its
/// kitty scope or a scope of its own that his terminal started, is counted
/// by its label, so a gap in what's watched shows rather than reads as
/// quiet; a test's, an agent's or a service's isn't.
/// The count is as of the minute's look, so one that ends or starts is
/// seen at the next.
#[derive(Debug)]
pub struct Apps {
    slice: String,
    known: BTreeMap<String, Known>,
    outside: BTreeMap<String, &'static str>,
    /// Who opened each scope named for a process, judged once by who
    /// started its process, and kept while any of its processes is.
    own_scopes: BTreeMap<String, Opener>,
    scopes_looked: BTreeSet<String>,
    /// Where systemd keeps the units made as they're run, such as a
    /// `systemd-run`'s, which none of his own services is.
    transient: PathBuf,
}

/// Who opened an app's process, as its cgroup and its parent's say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opener {
    /// Joel, from his desktop, his session or a service of his.
    Joel,
    /// Joel's terminal, whose work isn't watched but is counted, and whose
    /// Chrome may carry his call.
    HisTerminal,
    /// A test, an agent, a service or another account.
    Other,
}

impl Apps {
    /// The apps of the account whose manager is at `account` in the
    /// cgroup tree, such as `/user.slice/user-1000.slice/user@1000.service`.
    pub fn of_account(account: &str) -> Self {
        Apps {
            slice: format!("{account}/app.slice"),
            known: BTreeMap::new(),
            outside: BTreeMap::new(),
            own_scopes: BTreeMap::new(),
            scopes_looked: BTreeSet::new(),
            transient: PathBuf::from("/nonexistent"),
        }
    }

    /// The same, taking a unit in his app.slice whose file is in
    /// `transient`, such as `$XDG_RUNTIME_DIR/systemd/transient`, as made by
    /// a `systemd-run` there, an agent's or a test's, and so not his.
    #[must_use]
    pub fn reading_transient_units_from(mut self, transient: PathBuf) -> Self {
        self.transient = transient;
        self
    }

    /// How many processes of each app's label Joel started from his
    /// terminal, which go unwatched, as of the minute's last look: one in
    /// its kitty scope, or a Chrome in the scope of its own it moves into
    /// within about half a second, which his terminal started. One
    /// jobs-sorter has since moved to the jobs slice can't be told from an
    /// agent's there, and isn't counted.
    pub fn unwatched(&self) -> BTreeMap<String, usize> {
        let mut counted = BTreeMap::new();
        for label in self.outside.values() {
            *counted.entry((*label).to_string()).or_default() += 1;
        }
        counted
    }

    /// The threads of `process`, an app's labelled `label`, from what was
    /// found when it was new, or found now; none when it isn't one Joel
    /// opened.
    pub fn of(&mut self, pid: &str, process: &Path, label: &'static str) -> Option<Vec<Thread>> {
        // Its cgroup first, so a process that isn't Joel's has nothing else
        // read.
        let cgroup = text(&process.join("cgroup"));
        let proc = process.parent().unwrap_or(process);
        let opener = self.opener(&cgroup, proc);
        if opener != Opener::Joel {
            self.known.remove(pid);
            if opener == Opener::HisTerminal {
                self.outside.insert(pid.to_string(), label);
            } else {
                self.outside.remove(pid);
            }
            return None;
        }
        self.outside.remove(pid);
        let started = started(&text(&process.join("stat")));
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

    /// Lets go of every process not looked into in the pass just made,
    /// which has ended, or whose pid now names another program.
    pub fn keep_only(&mut self, looked: &BTreeSet<String>) {
        self.known.retain(|pid, _| looked.contains(pid));
        self.outside.retain(|pid, _| looked.contains(pid));
        let scopes = std::mem::take(&mut self.scopes_looked);
        self.own_scopes.retain(|scope, _| scopes.contains(scope));
    }

    /// Whether `cgroup` is a scope of Joel's terminal, directly in his own
    /// `app.slice`, as kitty puts each window's shell. An agent's kitty
    /// scope is in its own slice, and is no window of his.
    fn in_his_terminal(&self, cgroup: &str) -> bool {
        cgroup.lines().any(|line| {
            line.strip_prefix("0::")
                .and_then(|path| path.strip_prefix(self.slice.as_str()))
                .and_then(|rest| rest.strip_prefix('/'))
                .is_some_and(|unit| {
                    !unit.contains('/')
                        && unit.starts_with("kitty-")
                        && unit.rsplit_once('.').is_some_and(|(_, k)| k == "scope")
                })
        })
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
    /// or one he started from a terminal, in its kitty scope or the scope of
    /// its own it moves into. A call is his
    /// however he opened the browser, so this is looser than the drawing
    /// test, which a terminal's work fails since jobs-sorter moves it once
    /// it's busy. A test's or an agent's Chrome, whose fake microphone would
    /// read as a call, is in neither. One jobs-sorter has moved into the
    /// jobs slice can't be told from an agent's there, and is left out.
    /// Processes are read from `proc`.
    pub fn may_call(&mut self, cgroup: &str, proc: &Path) -> bool {
        self.opener(cgroup, proc) != Opener::Other
    }

    /// Who opened a process in `cgroup`: his terminal, when it's in a kitty
    /// scope directly in his app.slice; Joel, when `launched_by_joel` says
    /// so and, in a scope named for a process, `app-<name>-<pid>.scope`, he
    /// started its process, read from `proc`. Chrome moves itself into such a scope of its own in his
    /// `app.slice`, `app-com.google.Chrome-<pid>` for the browser's pid,
    /// whoever started it: jobs-sorter moves an agent's back about half a
    /// second later, which a minute's look can land in (5FWIUS 6f812ab4),
    /// and one an agent's session started stays. So the scope is his only
    /// when its process's parent is his: his user manager, his session, or
    /// any unit in his app.slice, where his own services (1Password's,
    /// Tuple's, the launcher's opener) open links too (5FWIUS 72d972c9),
    /// but those `NOT_HIS` names and those made as they're run, as a
    /// `systemd-run` makes them, which none of his services is. One his
    /// terminal started is his terminal's: counted, and free to carry his
    /// call, but not watched, since an agent's there reads the same. The
    /// verdict is judged once and kept while the watch runs, since a parent
    /// moved later, as jobs-sorter moves a terminal's busy work, didn't
    /// change who opened it; a restart judges again. A scope whose named
    /// process is gone, a launcher's wrapper that ended, or whose pid now
    /// names a process elsewhere, is left to its unit's name, as any app's,
    /// and judged again at the next look. So is one whose parent ended
    /// before its cgroup was read, and by the next look the user manager
    /// has taken it on. A Chrome that's a service's own process, or an
    /// agent's whose launcher ended before it was judged, is the user
    /// manager's child, as his is, and reads as his: nothing about its
    /// parent tells them apart, and browser tools keep their launcher.
    fn opener(&mut self, cgroup: &str, proc: &Path) -> Opener {
        if self.in_his_terminal(cgroup) {
            return Opener::HisTerminal;
        }
        if !self.launched_by_joel(cgroup) {
            return Opener::Other;
        }
        let Some((scope, pid)) = named_scope(cgroup) else {
            return Opener::Joel;
        };
        self.scopes_looked.insert(scope.to_string());
        if let Some(his) = self.own_scopes.get(scope) {
            return *his;
        }
        let its_own = text(&proc.join(pid).join("cgroup"))
            .lines()
            .any(|line| line.strip_prefix("0::") == Some(scope));
        let stat = text(&proc.join(pid).join("stat"));
        let (true, Some(parent)) = (its_own, parent(&stat)) else {
            return Opener::Joel;
        };
        let parents = text(&proc.join(parent).join("cgroup"));
        if parents.lines().all(|line| !line.starts_with("0::")) {
            return Opener::Joel;
        }
        let opener = if self.in_his_terminal(&parents) {
            Opener::HisTerminal
        } else if self.his_parent(&parents) {
            Opener::Joel
        } else {
            Opener::Other
        };
        self.own_scopes.insert(scope.to_string(), opener);
        opener
    }

    /// Whether a parent in the cgroup `parents` is his: his user manager,
    /// his session, or a unit in his app.slice but those `NOT_HIS` names
    /// and those made as they're run. A launcher's `app-*.scope` is made as
    /// it's run too, and is his: his Slack or 1Password opening a link with
    /// no browser open starts one (5FWIUS 9a142177).
    fn his_parent(&self, parents: &str) -> bool {
        let account = self.slice.trim_end_matches("/app.slice");
        parents
            .lines()
            .filter_map(|line| line.strip_prefix("0::"))
            .filter_map(|path| path.strip_prefix(account))
            .any(|rest| {
                rest == "/init.scope"
                    || rest.starts_with("/session.slice/")
                    || rest.strip_prefix("/app.slice/").is_some_and(|path| {
                        // A run's own cgroups under it, a delegated one's,
                        // are the run's.
                        let unit = path.split('/').next().unwrap_or(path);
                        let launched = unit
                            .strip_prefix("app-")
                            .is_some_and(|rest| rest.strip_suffix(".scope").is_some());
                        !NOT_HIS.iter().any(|name| unit.starts_with(name))
                            && (launched || !self.transient.join(unit).exists())
                    })
            })
    }
}

/// A scope named for a process, `app-<name>-<pid>.scope`, from a cgroup
/// file: the scope's path and the pid.
fn named_scope(cgroup: &str) -> Option<(&str, &str)> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::"))?;
    let unit = path.rsplit('/').next()?;
    let (_, pid) = unit
        .strip_prefix("app-")?
        .strip_suffix(".scope")?
        .rsplit_once('-')?;
    (!pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit())).then_some((path, pid))
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
        // The GPU process: its compositor and its main thread.
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
        // Ones Joel started from his terminal go unwatched and are
        // counted; an agent's, in its own slice, a test's in its service
        // and the other session's browser above are not.
        for (pid, comm) in [("94", "chrome"), ("96", "slack")] {
            write(&proc.join(pid).join("comm"), &format!("{comm}\n"));
            write(
                &proc.join(pid).join("cgroup"),
                &format!("0::{ACCOUNT}/app.slice/kitty-70-0.scope\n"),
            );
        }
        write(&proc.join("97/comm"), "chrome\n");
        write(
            &proc.join("97/cgroup"),
            &format!("0::{ACCOUNT}/app.slice/call-under-load.service\n"),
        );
        write(&proc.join("98/comm"), "chrome\n");
        write(
            &proc.join("98/cgroup"),
            &format!("0::{ACCOUNT}/agents.slice/agent.scope\n"),
        );
        // A scope systemd-run made directly in his app.slice is no terminal.
        write(&proc.join("99/comm"), "chrome\n");
        write(
            &proc.join("99/cgroup"),
            &format!("0::{ACCOUNT}/app.slice/run-u123.scope\n"),
        );
        // Neither an app nor a call: nothing of it is read.
        write(&proc.join("90/comm"), "bash\n");
        let mut apps = Apps::of_account(ACCOUNT);
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui Compositor 72/task/73/schedstat",
                "chrome-ui VizCompositorTh 75/task/76/schedstat",
                "chrome-ui chrome 70/task/70/schedstat",
                "chrome-ui chrome 72/task/72/schedstat",
                "chrome-ui chrome 75/task/75/schedstat",
                "slack-ui Compositor 80/task/81/schedstat",
                "slack-ui slack 80/task/80/schedstat",
            ]
        );
        assert_eq!(
            apps.unwatched(),
            BTreeMap::from([("chrome-ui".to_string(), 1), ("slack-ui".to_string(), 1)])
        );
        // Once one has ended, it's no longer counted.
        fs::remove_dir_all(proc.join("96")).unwrap();
        fs::remove_dir_all(proc.join("94")).unwrap();
        drawing_threads(&proc, &mut apps);
        assert!(apps.unwatched().is_empty(), "{:?}", apps.unwatched());
        fs::create_dir_all(proc.join("94")).unwrap();
        write(&proc.join("94/comm"), "chrome\n");
        write(
            &proc.join("94/cgroup"),
            &format!("0::{ACCOUNT}/app.slice/kitty-70-0.scope\n"),
        );
        drawing_threads(&proc, &mut apps);
        assert_eq!(
            apps.unwatched(),
            BTreeMap::from([("chrome-ui".to_string(), 1)])
        );
        // Its pid taken by another program, it's let go at once.
        write(&proc.join("94/comm"), "bash\n");
        drawing_threads(&proc, &mut apps);
        assert!(apps.unwatched().is_empty(), "{:?}", apps.unwatched());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_chrome_in_a_scope_of_its_own_is_joel_s_only_when_he_started_it() {
        let root = std::env::temp_dir().join(format!("lag-watch-own-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        let own = |pid: &str, browser: &str, parent: &str| {
            write(&proc.join(pid).join("comm"), "chrome\n");
            let scope = format!("{ACCOUNT}/app.slice/app-com.google.Chrome-{browser}.scope");
            write(&proc.join(pid).join("cgroup"), &format!("0::{scope}\n"));
            write(
                &proc.join(pid).join("stat"),
                &format!("{pid} (chrome) S {parent} 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0 9\n"),
            );
            write(
                &proc.join(pid).join("task").join(pid).join("comm"),
                "chrome\n",
            );
        };
        for (pid, parent, unit) in [
            // Joel's browsers: one his user manager started, one the portal
            // opened for a link, one Slack opened, and one 1Password's
            // service opened.
            ("20", "2", "init.scope"),
            ("22", "4", "session.slice/xdg-desktop-portal.service"),
            ("24", "8", "app.slice/app-niri-slack-8.scope"),
            ("28", "12", "app.slice/onepassword.service"),
            // One his terminal started, which may be a pi's: counted, not
            // watched.
            ("26", "10", "app.slice/kitty-70-0.scope"),
            // Browsers the hourly test call and a run of it by hand started,
            // a session of mycelium's in his app.slice, an agent's shared
            // run and a pi host, which nothing moves back.
            ("50", "5", "app.slice/call-under-load.service"),
            ("54", "13", "app.slice/call-under-load-5193fv.service"),
            ("80", "9", "app.slice/mycelium-pi-a.scope"),
            ("60", "6", "shared.slice/run-u1.scope"),
            ("70", "7", "agents.slice/pi-host-work-attended.service"),
        ] {
            own(pid, pid, parent);
            write(&proc.join(pid).join("task/99/comm"), "Chrome_IOThread\n");
            write(
                &proc.join(parent).join("cgroup"),
                &format!("0::{ACCOUNT}/{unit}\n"),
            );
        }
        // One whose parent ended before its cgroup was read, which is left
        // to its unit's name until the next look, when the user manager has
        // taken it on.
        own("36", "36", "14");
        write(&proc.join("36/task/37/comm"), "Chrome_IOThread\n");
        // An agent's browser in the moment before jobs-sorter moves it back,
        // and its renderer, whose own parent is the zygote in that scope.
        own("30", "30", "3");
        write(&proc.join("30/task/31/comm"), "Chrome_IOThread\n");
        write(
            &proc.join("3/cgroup"),
            &format!("0::{ACCOUNT}/jobs.slice/jobs-work.slice/work-1-2.scope\n"),
        );
        own("32", "30", "30");
        write(&proc.join("32/task/33/comm"), "Compositor\n");
        // A scope whose named browser has ended, its pid now an agent's
        // elsewhere, is left to its unit's name.
        own("42", "40", "1");
        write(&proc.join("42/task/43/comm"), "Chrome_IOThread\n");
        write(&proc.join("40/comm"), "bash\n");
        write(
            &proc.join("40/cgroup"),
            &format!("0::{ACCOUNT}/jobs.slice/jobs-work.slice/work-1-2.scope\n"),
        );
        write(
            &proc.join("40/stat"),
            "40 (bash) S 3 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0 9\n",
        );
        let mut apps = Apps::of_account(ACCOUNT);
        let his = ["20", "22", "24", "28", "36", "42"]
            .map(|pid| format!("chrome-ui chrome {pid}/task/{pid}/schedstat"));
        assert_eq!(drawing_threads(&proc, &mut apps), his);
        assert_eq!(
            apps.unwatched(),
            BTreeMap::from([("chrome-ui".to_string(), 1)]),
            "only the browser his terminal started goes unwatched"
        );
        let mut calls = |pid: &str| apps.may_call(&text(&proc.join(pid).join("cgroup")), &proc);
        for pid in ["20", "22", "24", "28", "26"] {
            assert!(calls(pid), "Joel's browser {pid} carries his call");
        }
        for pid in ["50", "54", "80", "70", "32"] {
            assert!(
                !calls(pid),
                "browser {pid}, not his, carries no call of his"
            );
        }
        // Who started it is judged once: a parent moved later, as
        // jobs-sorter moves a terminal's busy work, changes nothing. The
        // one whose parent ended is now the user manager's, and reads as
        // his, as an orphaned browser does.
        let jobs = format!("0::{ACCOUNT}/jobs.slice/jobs-work.slice/work-1-2.scope\n");
        write(&proc.join("2/cgroup"), &jobs);
        write(
            &proc.join("16/cgroup"),
            &format!("0::{ACCOUNT}/init.scope\n"),
        );
        write(
            &proc.join("36/stat"),
            "36 (chrome) S 16 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0 9\n",
        );
        // Pass after pass, so a verdict kept only by the look just made
        // wouldn't do.
        for _ in 0..2 {
            assert_eq!(drawing_threads(&proc, &mut apps), his);
        }
        // A scope that empties is forgotten: a browser started again in it
        // is judged by its own parent.
        fs::remove_dir_all(proc.join("20")).unwrap();
        drawing_threads(&proc, &mut apps);
        own("20", "20", "2");
        write(&proc.join("20/task/21/comm"), "Chrome_IOThread\n");
        let again = drawing_threads(&proc, &mut apps);
        assert!(
            !again.iter().any(|line| line.contains(" 20/")),
            "a browser started again in an emptied scope kept the last one's verdict: {again:?}"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_chrome_a_run_made_in_his_app_slice_started_isn_t_his() {
        let root = std::env::temp_dir().join(format!("lag-watch-run-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        let transient = root.join("transient");
        write(&transient.join("brio-night-2.service"), "[Unit]\n");
        write(&transient.join("app-slack-4.scope"), "[Unit]\n");
        // One a declared service of his started, one his Slack did from the
        // scope his launcher made for it, and one an agent's systemd-run
        // made in his app.slice did.
        for (pid, parent, unit) in [
            ("20", "2", "onepassword.service"),
            ("24", "4", "app-slack-4.scope"),
            ("30", "3", "brio-night-2.service"),
            ("32", "5", "brio-night-2.service/payload"),
        ] {
            let scope = format!("{ACCOUNT}/app.slice/app-com.google.Chrome-{pid}.scope");
            write(&proc.join(pid).join("comm"), "chrome\n");
            write(&proc.join(pid).join("cgroup"), &format!("0::{scope}\n"));
            write(
                &proc.join(pid).join("stat"),
                &format!("{pid} (chrome) S {parent} 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0 9\n"),
            );
            write(
                &proc.join(pid).join("task").join(pid).join("comm"),
                "chrome\n",
            );
            write(&proc.join(pid).join("task/99/comm"), "Chrome_IOThread\n");
            write(
                &proc.join(parent).join("cgroup"),
                &format!("0::{ACCOUNT}/app.slice/{unit}\n"),
            );
        }
        let mut apps = Apps::of_account(ACCOUNT).reading_transient_units_from(transient);
        assert_eq!(
            drawing_threads(&proc, &mut apps),
            [
                "chrome-ui chrome 20/task/20/schedstat",
                "chrome-ui chrome 24/task/24/schedstat",
            ]
        );
        assert!(
            !apps.may_call(&text(&proc.join("30/cgroup")), &proc),
            "a run's Chrome carries no call of his"
        );
        assert!(
            !apps.may_call(&text(&proc.join("32/cgroup")), &proc),
            "a Chrome from a run's own cgroup carries no call of his"
        );
        assert!(apps.unwatched().is_empty(), "{:?}", apps.unwatched());
        fs::remove_dir_all(&root).unwrap();
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
            format!("{mine}/call-under-load.service"),
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
        let mut apps = Apps::of_account(ACCOUNT);
        let mut calls = |path: &str| apps.may_call(&format!("0::{path}\n"), Path::new("/nowhere"));
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
            // A cgroup a program made below a terminal's scope.
            format!("{mine}/kitty-70-0.scope/payload/app-a.scope"),
            // A test's Chrome, and another account's terminal.
            format!("{mine}/call-under-load.service"),
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
        fs::remove_dir_all(&root).unwrap();
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
        fs::remove_dir_all(&root).unwrap();
    }
}
