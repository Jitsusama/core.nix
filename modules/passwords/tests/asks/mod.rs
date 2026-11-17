//! Secret asks answered by the daemon, as the shell runs it: an ask put
//! up in the runtime folder's `secret-asks` is served as a row among
//! Passwords' own, and only a yes from the launcher, on the daemon's own
//! standard input, reads its listed reference from its manager and runs
//! its command with the value. With nothing listed it watches no folder
//! and draws no row; an item not listed, or a program outside the
//! root-owned folders, is refused unseen; a yes on the row's own key, one
//! before the line could be read, or one for what changed since it was
//! drawn releases nothing; and the value never reaches an answer, the
//! journal or a command line.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::common::{Client, RELEASED_VALUE, Running, stored_folder, until};
use serde_json::{Value, json};

mod docker;
mod grants;
mod history;
mod watch;

/// The one reference listed, and one that isn't.
const LISTED: &str = "op://Test/Throwaway/password";
const UNLISTED: &str = "op://Private/Something Real/password";

/// The shell's own bash, in the store, so it's a root-owned program.
fn stored_bash() -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .filter_map(|folder| std::fs::canonicalize(folder.join("bash")).ok())
        .find(|bash| bash.starts_with("/nix/store"))
        .expect("no bash in the store on PATH")
}

/// Puts an ask up as the harness does, written aside and renamed into
/// place, asking for `item` to run `argv` with it as `GIVEN`.
fn put_up(running: &Running, id: &str, item: &str, argv: &[&str]) {
    put_up_in(running, &running.folder, id, item, argv);
}

/// Puts an ask up as `put_up` does, from the tree at `tree`.
fn put_up_in(running: &Running, tree: &Path, id: &str, item: &str, argv: &[&str]) {
    let asks = running.asks();
    std::fs::create_dir_all(&asks).unwrap();
    let asked_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let ask = json!({
        "version": 1, "id": id, "asked_ms": u64::try_from(asked_ms).unwrap(),
        "session": "TESTER", "quest": null, "item": item,
        "why": "To try the line.", "argv": argv,
        "tree": tree.to_str().unwrap(), "given": "GIVEN", "minutes": 1,
    });
    let aside = asks.join(format!(".{id}.writing"));
    std::fs::write(&aside, ask.to_string()).unwrap();
    std::fs::rename(&aside, asks.join(format!("{id}.ask.json"))).unwrap();
}

/// A command that says the value it was given, which its answer masks.
fn saying(bash: &Path) -> Vec<String> {
    vec![
        bash.to_str().unwrap().to_owned(),
        "-c".to_owned(),
        "printf 'got %s\\n' \"$GIVEN\"".to_owned(),
    ]
}

/// The row of an ask, once a watch of rows is told of it.
fn row_of(client: &mut Client, id: &str) -> Value {
    let key = format!("row:asks:{id}");
    client.say(&json!({"id": "w", "ask": "watch", "kind": "row"}));
    loop {
        let heard = client.hear();
        let records = heard["records"].as_array().or(heard["changed"].as_array());
        if let Some(record) = records
            .into_iter()
            .flatten()
            .find(|record| record["key"] == key)
        {
            return record["row"].clone();
        }
    }
}

/// The keys of the rows held now, asking no manager anything.
fn row_keys(client: &mut Client) -> Vec<String> {
    let held = client.ask(&json!({"id": "l", "ask": "list", "kind": "row", "held": true}));
    held["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|record| record["key"].as_str().map(str::to_owned))
        .collect()
}

/// The yes the launcher sends for a row, the line shown `shown` ms.
fn yes(row: &Value, shown: u64) -> Value {
    let mut asked = row["acts"][0]["ask"].clone();
    asked["id"] = json!(7);
    asked["shown"] = json!(shown);
    asked["opening"] = json!(1);
    asked
}

/// An ask's answer, once it's written.
fn answer(running: &Running, id: &str) -> Value {
    let path = running.asks().join(format!("{id}.answer.json"));
    until("the answer", || path.exists());
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Whether the manager was asked to release anything.
fn released(running: &Running) -> bool {
    running.asked_of("1password").contains("\"release\"")
}

/// Says the value is nowhere it may not be: the log, the asks folder or
/// the command line bubblewrap was given.
fn nowhere(running: &Running) {
    assert!(
        !running.logged().contains(RELEASED_VALUE),
        "the log holds the value"
    );
    assert!(
        !running.bwrapped().contains(RELEASED_VALUE),
        "a command line holds the value"
    );
    for entry in std::fs::read_dir(running.asks()).into_iter().flatten() {
        let path = entry.unwrap().path();
        let held = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !held.contains(RELEASED_VALUE),
            "{} holds the value",
            path.display()
        );
    }
}

#[test]
fn with_nothing_listed_an_ask_draws_no_row_and_a_yes_releases_nothing() {
    let running = Running::releasing("asks-off", &[]);
    let id = "0123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    // Nothing is meant to happen, so there is no event to wait on: this
    // gives a folder watch, were one running, time to draw the ask.
    std::thread::sleep(Duration::from_millis(1500));
    let keys = row_keys(&mut client);
    assert!(
        !keys.iter().any(|key| key.starts_with("row:asks:")),
        "an ask drew a row with nothing listed: {keys:?}"
    );
    assert!(
        !running.asks().join("answering").exists(),
        "a sign says asks are answered with nothing listed"
    );
    let refused = client.ask(
        &json!({"id": 7, "ask": "release", "key": format!("ask:{id}"),
                                     "digest": "0", "shown": 812, "opening": 1}),
    );
    assert_eq!(
        refused["error"]["message"], "not listed",
        "a yes with nothing listed: {refused}"
    );
    assert!(
        running.asks().join(format!("{id}.ask.json")).exists()
            && !running.asks().join(format!("{id}.answer.json")).exists(),
        "an ask was answered with nothing listed"
    );
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn a_listed_ask_is_drawn_and_its_yes_runs_the_command_with_the_value_masked() {
    let running = Running::releasing("asks-yes", &[LISTED]);
    let id = "1123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    assert_eq!(row["thing"], format!("ask:{id}"), "the row: {row}");
    assert_eq!(row["state"], "waiting", "the row: {row}");
    let said = client.ask(&yes(&row, 812));
    assert_eq!(said["releasing"], true, "the yes: {said}");
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "ran", "the answer: {answer}");
    assert_eq!(answer["exit"], 0, "the answer: {answer}");
    assert_eq!(
        answer["output"], "got [released value]\n",
        "the output wasn't masked: {answer}"
    );
    let asked = running.asked_of("1password");
    assert!(
        asked.contains(&format!("\"reference\":\"{LISTED}\"")) && asked.contains("\"asked\":true"),
        "the manager wasn't asked for the listed reference as Joel: {asked}"
    );
    let bwrapped = running.bwrapped();
    for kept in [
        "--die-with-parent",
        "--unshare-pid",
        "--new-session",
        "--chdir",
    ] {
        assert!(
            bwrapped.lines().any(|line| line == kept),
            "bubblewrap wasn't given {kept}: {bwrapped}"
        );
    }
    nowhere(&running);
    until("the ask taken away", || {
        !running.asks().join(format!("{id}.ask.json")).exists()
    });
}

#[test]
fn an_asks_reading_reaches_the_journal_the_managers_file_names() {
    // The daemon's journal is the file's, so a test's readings never
    // reach the account's own, where health would count them.
    let running = Running::releasing("asks-journal", &[LISTED]);
    let journal = running.journal();
    let id = "0323456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    let said = client.ask(&yes(&row, 812));
    assert_eq!(said["releasing"], true, "the yes: {said}");
    let mut line = [0_u8; 4096];
    let length = journal
        .recv(&mut line)
        .expect("a reading reached the file's journal");
    let reading = String::from_utf8_lossy(&line[..length]);
    assert!(
        reading.contains("ASK_OUTCOME=released\n"),
        "the reading: {reading}"
    );
}

#[test]
fn an_ask_for_an_item_not_listed_is_refused_unseen() {
    let running = Running::releasing("asks-unlisted", &[LISTED]);
    let id = "2123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        UNLISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(answer["why"], "not listed", "the answer: {answer}");
    let mut client = running.client();
    let keys = row_keys(&mut client);
    assert!(
        !keys.iter().any(|key| key.starts_with("row:asks:")),
        "an unlisted ask drew a row: {keys:?}"
    );
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn an_ask_to_run_a_program_outside_the_root_owned_folders_is_refused() {
    let running = Running::releasing("asks-unowned", &[LISTED]);
    let id = "3123456789abcdef0123456789abcdef";
    let program = running.folder.join("mine");
    std::fs::write(&program, "#!/bin/sh\necho \"$GIVEN\"\n").unwrap();
    std::fs::set_permissions(
        &program,
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    put_up(&running, id, LISTED, &[program.to_str().unwrap()]);
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(
        answer["why"], "not a root-owned program",
        "the answer: {answer}"
    );
    assert!(!released(&running), "the manager was asked to release");
}

/// A profile the account may repoint, as `~/.nix-profile` is a link it
/// owns, listed among the daemon's profiles and planted at a store
/// folder no other profile is: an agent could point it at a program it
/// built itself, which is in the store too.
fn planted_profile(running: &Running) -> PathBuf {
    let bin = running.planted_profile();
    std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(stored_elsewhere().parent().unwrap(), &bin).unwrap();
    bin
}

/// The name of the program only the planted profile holds.
fn planted_name() -> String {
    stored_elsewhere()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[test]
fn a_program_named_through_a_profile_the_account_may_repoint_is_refused() {
    let running = Running::releasing("asks-repointed", &[LISTED]);
    planted_profile(&running);
    let id = "7123456789abcdef0123456789abcdef";
    put_up(&running, id, LISTED, &[&planted_name()]);
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(answer["why"], NOT_IN_A_PROFILE, "the answer: {answer}");
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn a_program_whose_path_goes_through_a_link_the_account_owns_is_refused() {
    let running = Running::releasing("asks-linked", &[LISTED]);
    let planted = planted_profile(&running).join(planted_name());
    let id = "9123456789abcdef0123456789abcdef";
    put_up(&running, id, LISTED, &[planted.to_str().unwrap()]);
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(
        answer["why"], "not a root-owned program",
        "the answer: {answer}"
    );
    assert!(!released(&running), "the manager was asked to release");
}

/// Why an ask naming what no profile holds is refused, saying how to ask
/// for a project's own tools instead: through its dev shell, which
/// direnv, a profile's program, opens.
const NOT_IN_A_PROFILE: &str =
    "not in a profile; ask for direnv exec . <cmd> to use the project's own tools";

/// A program in the store but in none of the profiles: an agent can
/// build one, or add it to the store, itself.
fn stored_elsewhere() -> PathBuf {
    let profile = stored_folder("bash");
    ["ls", "cat", "true", "env"]
        .into_iter()
        .map(|name| stored_folder(name).join(name))
        .find(|program| !program.starts_with(&profile))
        .expect("no program in the store outside bash's folder")
}

#[test]
fn a_program_named_by_a_store_path_outside_the_profiles_is_refused() {
    let running = Running::releasing("asks-stored", &[LISTED]);
    let id = "6123456789abcdef0123456789abcdef";
    let elsewhere = stored_elsewhere();
    put_up(&running, id, LISTED, &[elsewhere.to_str().unwrap()]);
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(answer["why"], NOT_IN_A_PROFILE, "the answer: {answer}");
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn a_released_commands_path_leaves_out_a_profile_the_account_may_repoint() {
    let running = Running::releasing("asks-path", &[LISTED]);
    let planted = planted_profile(&running);
    let id = "8123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    put_up(
        &running,
        id,
        LISTED,
        &[
            bash.to_str().unwrap(),
            "-c",
            "printf 'path %s\\n' \"$PATH\"",
        ],
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    client.ask(&yes(&row, 812));
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "ran", "the answer: {answer}");
    let said = answer["output"].as_str().unwrap_or_default();
    assert!(
        said.starts_with("path ") && !said.contains(planted.parent().unwrap().to_str().unwrap()),
        "the command's PATH holds the account's own profile: {said}"
    );
}

#[test]
fn a_yes_on_the_rows_key_or_too_soon_releases_nothing_and_a_no_refuses() {
    let running = Running::releasing("asks-no", &[LISTED]);
    let id = "4123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    let mut by_row = yes(&row, 812);
    by_row["key"] = json!(format!("row:asks:{id}"));
    let said = client.ask(&by_row);
    assert!(said["error"].is_object(), "a yes by the row's key: {said}");
    let said = client.ask(&yes(&row, 120));
    assert_eq!(said["error"]["code"], "refused", "a yes too soon: {said}");
    let mut no = row["acts"][1]["ask"].clone();
    no["id"] = json!(8);
    let said = client.ask(&no);
    assert_eq!(said["refused"], true, "the no: {said}");
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(answer["why"], "refused by Joel", "the answer: {answer}");
    assert!(
        answer.get("output").is_none(),
        "a no ran something: {answer}"
    );
    assert!(!released(&running), "the manager was asked to release");
    nowhere(&running);
}

#[test]
fn a_yes_for_what_changed_since_it_was_drawn_releases_nothing() {
    let running = Running::releasing("asks-changed", &[LISTED]);
    let id = "5123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    let mut stale = yes(&row, 812);
    stale["digest"] = json!("0".repeat(64));
    let said = client.ask(&stale);
    assert_eq!(said["error"]["code"], "changed", "a stale yes: {said}");
    assert!(!released(&running), "the manager was asked to release");
    assert!(
        row_keys(&mut client).contains(&format!("row:asks:{id}")),
        "the ask's row went"
    );
}

/// What a listed ask running `argv` showed and was answered, once Joel
/// said yes and its command ran, with bubblewrap's arguments.
struct Release {
    row: Value,
    answer: Value,
    args: Vec<String>,
}

/// Releases a listed ask running `argv`, once Joel says yes.
fn release(running: &Running, id: &str, argv: &[String]) -> Release {
    put_up(
        running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    client.ask(&yes(&row, 812));
    let answer = answer(running, id);
    assert_eq!(answer["state"], "ran", "the answer: {answer}");
    let bwrapped = running.bwrapped().lines().map(str::to_owned).collect();
    Release {
        row,
        answer,
        args: bwrapped,
    }
}

/// Releases a listed ask whose command says its value.
fn releasing_saying(running: &Running, id: &str) -> Release {
    release(running, id, &saying(&stored_bash()))
}

/// Bubblewrap's arguments for a listed ask Joel said yes to, once its
/// command ran.
fn released_with(running: &Running, id: &str) -> Vec<String> {
    releasing_saying(running, id).args
}

/// Where in `args` these come together, in this order.
fn found(args: &[String], run: &[&Path]) -> Option<usize> {
    args.windows(run.len()).position(|window| {
        window
            .iter()
            .zip(run)
            .all(|(arg, wanted)| Path::new(arg) == *wanted)
    })
}

/// Says `args` hold these together, in this order, and where.
fn holds(args: &[String], run: &[&Path]) -> usize {
    found(args, run).unwrap_or_else(|| panic!("bubblewrap wasn't given {run:?}: {args:?}"))
}

#[test]
fn a_released_command_is_kept_from_what_the_fence_hides() {
    let running = Running::releasing("asks-hidden", &[LISTED]);
    let args = released_with(&running, "a123456789abcdef0123456789abcdef");
    let home = running.folder.join("home");
    let null = Path::new("/dev/null");
    let ro = Path::new("--ro-bind");
    let first = holds(&args, &[ro, null, &home.join(".secret-file")]);
    let folder = home.join(".secret-folder");
    holds(
        &args,
        &[
            Path::new("--tmpfs"),
            &folder,
            Path::new("--remount-ro"),
            &folder,
        ],
    );
    holds(&args, &[ro, null, &running.folder.join("outside-secret")]);
    assert!(
        !args.iter().any(|arg| arg.contains(".absent-file")),
        "bubblewrap was given what isn't there: {args:?}"
    );
    let ssh = home.join(".ssh");
    let emptied = holds(&args, &[Path::new("--tmpfs"), &ssh]);
    for kept in ["config", "id_ed25519.pub"] {
        let kept = ssh.join(kept);
        assert!(
            holds(&args, &[ro, &kept, &kept]) > emptied,
            "{} was bound back before ~/.ssh was emptied: {args:?}",
            kept.display()
        );
    }
    assert!(
        !args
            .iter()
            .any(|arg| Path::new(arg) == ssh.join("id_ed25519")),
        "the private key was bound back: {args:?}"
    );
    assert!(
        holds(&args, &[Path::new("--remount-ro"), &ssh]) > emptied,
        "~/.ssh wasn't made read-only: {args:?}"
    );
    let npmrc = home.join(".npmrc");
    let copy = args
        .windows(3)
        .find(|window| window[0] == "--ro-bind" && Path::new(&window[2]) == npmrc)
        .map_or_else(
            || panic!("no copy of ~/.npmrc was bound over it: {args:?}"),
            |window| window[1].clone(),
        );
    let copied = std::fs::read_to_string(&copy).unwrap();
    assert!(
        copied.contains("registry=https://registry.example/")
            && !copied.contains("npm-made-up-token"),
        "the copy of ~/.npmrc: {copied}"
    );
    let docker = holds(&args, &[ro, null, &home.join(".docker/config.json")]);
    assert!(
        !args
            .iter()
            .any(|arg| Path::new(arg) == home.join(".kept-settings.json")),
        "settings holding no credential were emptied: {args:?}"
    );
    let tree = &running.folder;
    assert!(
        holds(&args, &[Path::new("--bind"), tree, tree]) < first.min(docker),
        "what the fence hides was hidden before the tree was bound writable: {args:?}"
    );
}

/// The folders `args` let the command write.
fn writable(args: &[String]) -> Vec<&String> {
    args.windows(2)
        .filter(|window| window[0] == "--bind")
        .map(|window| &window[1])
        .collect()
}

#[test]
fn a_released_command_writes_its_tree_and_caches_alone() {
    let running = Running::releasing("asks-writes", &[LISTED]);
    let git = running.folder.join(".git");
    for folder in ["objects", "refs", "hooks"] {
        std::fs::create_dir_all(git.join(folder)).unwrap();
    }
    std::fs::write(git.join("config"), "").unwrap();
    let home = running.folder.join("home");
    caches(&home);
    let args = released_with(&running, "b123456789abcdef0123456789abcdef");
    let bind = Path::new("--bind");
    let ro = Path::new("--ro-bind");
    let tree = &running.folder;
    holds(&args, &[bind, tree, tree]);
    let kept = holds(&args, &[ro, &git, &git]);
    for written in ["objects", "refs"] {
        let written = git.join(written);
        assert!(
            holds(&args, &[bind, &written, &written]) > kept,
            "{} wasn't bound back writable: {args:?}",
            written.display()
        );
    }
    let cache = home.join(".cache");
    let nix = cache.join("nix");
    assert!(
        holds(&args, &[ro, &nix, &nix]) > holds(&args, &[bind, &cache, &cache]),
        "Nix's fetcher cache, which later unfenced runs trust, was left writable: {args:?}"
    );
    let npm = home.join(".npm/_cacache");
    holds(&args, &[bind, &npm, &npm]);
    for layered in OVERLAID {
        let layered = home.join(layered);
        holds(
            &args,
            &[
                Path::new("--overlay-src"),
                &layered,
                Path::new("--tmp-overlay"),
                &layered,
            ],
        );
    }
    let writable = writable(&args);
    assert_eq!(
        writable.len(),
        5,
        "the command may write more than its tree, git's objects and refs, and the caches \
         that check what they hold: {writable:?}"
    );
}

/// The tools' caches that take a throwaway layer, since what they hold
/// is unpacked and run unchecked by later runs outside the fence.
const OVERLAID: [&str; 2] = [".cargo/registry", ".gem"];

/// Lays out the tools' caches in `home`, beside some they don't write.
fn caches(home: &Path) {
    for folder in [
        ".cache/nix",
        ".npm/_cacache",
        ".npm/_npx",
        ".cargo/registry",
        ".cargo/bin",
        ".gem",
    ] {
        std::fs::create_dir_all(home.join(folder)).unwrap();
    }
}

#[test]
fn a_released_command_without_throwaway_layers_reads_the_caches_that_take_them() {
    let running = Running::releasing_as("asks-unlayered", &[LISTED], false);
    let home = running.folder.join("home");
    caches(&home);
    let args = released_with(&running, "1223456789abcdef0123456789abcdef");
    for layered in OVERLAID {
        let layered = home.join(layered);
        assert!(
            !args.iter().any(|arg| Path::new(arg) == layered),
            "{} was bound, where it stays read-only: {args:?}",
            layered.display()
        );
    }
    assert!(
        !args.iter().any(|arg| arg.contains("overlay")),
        "a bubblewrap without layers was asked for one: {args:?}"
    );
}

#[test]
fn an_ask_is_refused_unseen_without_the_fences_list_or_with_one_it_cant_read() {
    let running = Running::releasing("asks-unfenced", &[LISTED]);
    let bash = stored_bash();
    let argv = saying(&bash);
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    std::fs::remove_file(running.fence_list()).unwrap();
    let missing = "c123456789abcdef0123456789abcdef";
    put_up(&running, missing, LISTED, &argv);
    let answered = answer(&running, missing);
    assert_eq!(answered["state"], "refused", "the answer: {answered}");
    assert_eq!(
        answered["why"], "no fence list to hide by",
        "the answer: {answered}"
    );
    std::fs::write(
        running.fence_list(),
        crate::common::FENCE_LIST.replace("\"version\": 1", "\"version\": 2"),
    )
    .unwrap();
    let unknown = "d123456789abcdef0123456789abcdef";
    put_up(&running, unknown, LISTED, &argv);
    let answered = answer(&running, unknown);
    assert_eq!(
        answered["why"], "no fence list to hide by",
        "the answer: {answered}"
    );
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn a_sign_says_asks_are_answered_while_the_daemon_answers_them() {
    let mut running = Running::releasing("asks-sign", &[LISTED]);
    let sign = running.asks().join("answering");
    until("the sign", || sign.exists());
    assert_eq!(
        std::fs::read_to_string(&sign).unwrap().trim(),
        running.pid().to_string(),
        "the sign doesn't name the daemon"
    );
    running.close();
    let started = std::time::Instant::now();
    while !running.ended() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the daemon outlived its channel"
        );
        // A child's end raises no event to wait on: this looks again
        // soon, as the daemon's own test of its channel does.
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!sign.exists(), "the sign outlived the daemon");
}
