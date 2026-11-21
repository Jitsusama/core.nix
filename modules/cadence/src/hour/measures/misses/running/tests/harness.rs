//! How a harness change to the pi hosts' program is judged: which files
//! the program loads, when the version carrying it was installed, and the
//! hosts and conversations that must have started since.

use super::*;

#[test]
fn a_harness_change_to_the_hosts_program_waits_on_them() {
    // The judge itself, not the one beside these tests that never reads the program.
    use super::super::judge;
    let all = |_: &str, _: &str| Some(true);
    let loads = |_: &str| Some(BTreeSet::from(["host/service/program.ts".to_owned()]));
    let program_only = Reaches {
        program: true,
        ..Reaches::default()
    };
    let lack = |_: &str, reaches: Reaches| {
        Some(if reaches == program_only {
            "program"
        } else {
            "misread"
        })
    };
    let at = Some("a1b2c3d");
    let to_program = |_: &str| Some("host/service/program.ts\ndocs/host.md".to_owned());
    assert_eq!(
        judge("harness", at, Some("f00d"), all, to_program, loads, lack),
        Some("program"),
        "installed, but a change to the hosts' program didn't wait on them alone"
    );
    let to_extension = |_: &str| Some("extensions/quest/index.ts".to_owned());
    assert_eq!(
        judge("harness", at, Some("f00d"), all, to_extension, loads, lack),
        None,
        "a change no host's program loads waited on the hosts"
    );
    assert_eq!(
        judge("harness", at, Some("f00d"), all, to_program, |_| None, lack),
        Some("cadence can't tell which harness files a pi host loads"),
        "a program cadence couldn't read counted as loading nothing"
    );
    assert_eq!(
        judge("harness", at, Some("f00d"), all, |_| None, loads, lack),
        Some("cadence can't read what it changed"),
        "an unread diff skipped the hosts"
    );
}

#[test]
fn every_host_must_have_started_since_its_program_was_installed() {
    let program = Reaches {
        program: true,
        ..Reaches::default()
    };
    let lack = |hosts: &[Host], at| hosts_lack(Some(hosts), &program, at, |_| Some(true));
    assert_eq!(lack(&[host(None, Some(200))], Some(100)), None);
    assert_eq!(
        lack(&[host(None, Some(200)), host(None, Some(50))], Some(100)),
        Some("a pi host or conversation runs the harness from before it"),
        "a host started before the install counted as running the new program"
    );
    let stopped = Host {
        standing: Standing::Stopped,
        ..host(None, Some(50))
    };
    assert_eq!(
        lack(&[host(None, Some(200)), stopped], Some(100)),
        None,
        "a stopped host, which loads what current names when it starts, held it"
    );
    assert_eq!(
        lack(&[host(None, None)], None),
        Some("cadence can't tell when the harness carrying it was installed"),
        "an unread install was reported as an unread host"
    );
    assert_eq!(
        lack(&[host(None, Some(200))], None),
        Some("cadence can't tell when the harness carrying it was installed"),
        "no install found carrying it, yet the hosts counted as started since"
    );
    assert_eq!(
        lack(&[host(None, None)], Some(100)),
        Some("cadence can't tell when a pi host started")
    );
    assert_eq!(
        lack(&[], Some(100)),
        None,
        "with no pi host running, a program change waited on one, though the next to start \
         loads the harness installed"
    );
}

#[test]
fn a_program_change_counts_from_the_first_install_seen_carrying_it() {
    let key = "harness #1900";
    let mut first = BTreeMap::new();
    let mut kept = BTreeMap::new();
    assert_eq!(
        first_installed(&first, &mut kept, key, || Some(100)),
        Some(100)
    );
    first = std::mem::take(&mut kept);
    assert_eq!(
        first_installed(&first, &mut kept, key, || Some(300)),
        Some(100),
        "a later install moved the time a conversation on an earlier one carrying it is \
         judged by"
    );
    // An hour not seeing it carried keeps nothing, so it counts afresh.
    first = BTreeMap::new();
    assert_eq!(
        first_installed(&first, &mut kept, key, || Some(500)),
        Some(500)
    );
    let mut none = BTreeMap::new();
    assert_eq!(
        first_installed(&BTreeMap::new(), &mut none, key, || None),
        None
    );
    assert!(none.is_empty(), "an unread install was kept as one");
}

#[test]
fn a_conversation_holds_a_change_to_the_program_it_runs_but_not_to_the_unit() {
    let program = Reaches {
        program: true,
        ..Reaches::default()
    };
    let unit = Reaches {
        unit: true,
        ..Reaches::default()
    };
    let hosts = [host(None, Some(200))];
    let old_talk = [host(None, Some(50))];
    let lack = |hosts: Option<&[Host]>, talks: Option<&[Host]>, reaches: &Reaches| {
        lacking(hosts, talks, reaches, Some(100), |_| Some(true))
    };
    assert_eq!(
        lack(Some(&hosts), Some(&old_talk), &program),
        Some("a pi host or conversation runs the harness from before it"),
        "a conversation started before the install didn't hold its program change"
    );
    assert_eq!(
        lack(Some(&hosts), Some(&old_talk), &unit),
        None,
        "a conversation held a change to the hosts' unit, which it doesn't run"
    );
    assert_eq!(
        lack(Some(&hosts), None, &program),
        Some("cadence can't list the pi conversations"),
        "conversations that couldn't be listed counted as none, or as the hosts unlisted"
    );
    assert_eq!(
        lack(Some(&old_talk), None, &program),
        Some("a pi host or conversation runs the harness from before it"),
        "an old host was outranked by conversations that couldn't be listed"
    );
    assert_eq!(
        lack(None, Some(&[]), &program),
        Some("cadence can't list the pi hosts"),
        "hosts that couldn't be listed counted as the conversations alone"
    );
}

#[test]
fn the_program_is_followed_through_the_harness_as_git_holds_it() {
    let repo = scratch("program");
    let write = |file: &str, text: &str| {
        let at = repo.join(file);
        std::fs::create_dir_all(at.parent().expect("a folder")).expect("a folder made");
        std::fs::write(at, text).expect("written");
    };
    write(
        "scripts/pi-host.ts",
        "import { a } from \"../host/a.ts\";\n",
    );
    write(
        "host/a.ts",
        "export {\n  b,\n} from './b.mjs';\nawait import(\"./c.ts\");\n",
    );
    write("host/b.mjs", "export const b = 1;\n");
    write(
        "host/c.ts",
        "new Worker(fileURLToPath(new URL(\"./d.mjs\", import.meta.url)));\n",
    );
    write("host/d.mjs", "export {};\n");
    write("extensions/x/index.ts", "import \"../../host/e.ts\";\n");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "a program"]);
    let head =
        command("git", &["-C", &repo.to_string_lossy(), "rev-parse", "HEAD"]).expect("the head");
    let path = repo.to_string_lossy();
    assert_eq!(
        program::files(&path, head.trim()),
        Some(BTreeSet::from(
            [
                "scripts/pi-host.ts",
                "host/a.ts",
                "host/b.mjs",
                "host/c.ts",
                "host/d.mjs",
                "package.json",
                "pnpm-lock.yaml",
            ]
            .map(str::to_owned)
        )),
        "git's grep didn't follow a split import, either quote, a dynamic import, a script \
         run on a thread or a script, or followed an extension's"
    );
    write("host/c.ts", "await import(\"./gone.ts\");\n");
    git(
        &repo,
        &["commit", "-q", "-am", "a program naming what isn't there"],
    );
    let head = command("git", &["-C", &path, "rev-parse", "HEAD"]).expect("the head");
    assert_eq!(
        program::files(&path, head.trim()),
        None,
        "a program naming a file its commit doesn't hold read as loading nothing past it"
    );
    write("scripts/pi-host.ts", "export {};\n");
    git(
        &repo,
        &["commit", "-q", "-am", "a program importing nothing"],
    );
    let head = command("git", &["-C", &path, "rev-parse", "HEAD"]).expect("the head");
    assert_eq!(
        program::files(&path, head.trim()),
        None,
        "a program cadence can't follow read as loading nothing"
    );
    std::fs::remove_dir_all(&repo).ok();
}
