//! What a quest still at work left in its `tmp/` and beside it, as the
//! survey offers it.

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

// Beside tmp/, a quest at work keeps its labs, clones and runs, 53 GiB of
// them on one laptop on 11 November, 11 GiB unwritten for a week. Those
// are offered too, by their last write, but never tmp/ itself (its
// entries go by their own rule), the handover a leaving machine packs,
// or a folder of GC roots, whose links keep what running programs read.
#[test]
fn what_a_quest_at_work_left_beside_its_tmp_is_offered_but_what_it_keeps() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let workspace = dir(&workspaces.join("QEST-20200101-ABCDEF"));
    let aged = |name: &str, seconds: f64| {
        let folder = dir(&workspace.join(name));
        file(&folder.join("run.log"), "x");
        written_ago(&folder.join("run.log"), seconds);
        written_ago(&folder, seconds);
        folder
    };
    let left = aged("lab", 8.0 * DAY);
    let held = aged("runs", 8.0 * DAY);
    aged("clone", 3_600.0);
    aged("tmp", 8.0 * DAY);
    aged("handover", 8.0 * DAY);
    aged("gcroots-pi", 8.0 * DAY);
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
    let paths: Vec<_> = found
        .iter()
        .filter(|offer| offer.candidate.class != Class::QuestTemp)
        .map(|offer| &offer.candidate.path)
        .collect();
    assert_eq!(
        paths,
        [&left],
        "only the lab, not tmp/, the handover, the roots, the young or the worked in"
    );
    let offered = find(&found, &left);
    assert_eq!(offered.candidate.class, Class::QuestScratch);
    assert_eq!(
        offered.action,
        Action::Entry {
            root: workspace.clone()
        }
    );
}
