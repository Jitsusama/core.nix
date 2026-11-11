//! What a quest still at work left in its `tmp/`, as the survey offers it.

use crate::discover::{dir, file};
use crate::survey::{DAY, config, find, offers, real, written_ago};
use disk_guard::activity::Process;
use disk_guard::plan::Class;
use disk_guard::survey::Action;

// A quest at work can leave its tmp/ for weeks: on one laptop 124 GiB of
// quest scratch lay in tmp/ folders, a quarter of it written to over a
// week before. What nothing has written to for six hours is offered, dated
// by its newest write, so the plan's week takes it; what's younger can't
// go in any band, and what a process works in stays whatever its age.
#[test]
fn what_a_quest_at_work_left_in_its_tmp_is_offered_by_its_last_write() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let tmp = dir(&workspaces.join("QEST-20200101-ABCDEF/tmp"));
    let aged = |name: &str, seconds: f64| {
        let lab = dir(&tmp.join(name));
        file(&lab.join("run.log"), "x");
        written_ago(&lab.join("run.log"), seconds);
        written_ago(&lab, seconds);
        lab
    };
    let left = aged("left-lab", 8.0 * DAY);
    let held = aged("held-lab", 8.0 * DAY);
    aged("young-lab", 3_600.0);
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        workspaces.display(),
        quests.display()
    ));
    let working = Process {
        pid: 1,
        uid: unsafe { libc::geteuid() },
        cwd: Some(held.clone()),
        readable: true,
        ..Process::default()
    };
    let found = offers(&config, &[working], &|_| false);
    assert_eq!(
        found.len(),
        1,
        "only what nothing works in, unwritten for 6 h: {found:?}"
    );
    let offered = find(&found, &left);
    assert_eq!(offered.candidate.class, Class::QuestTemp);
    assert!(
        offered.candidate.idle > 7.9 * DAY,
        "dated by its last write: {}",
        offered.candidate.idle
    );
    assert_eq!(offered.action, Action::Entry { root: tmp.clone() });
}
