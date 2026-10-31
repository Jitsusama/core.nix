//! update-to-main against a real git repository, with real signatures, and
//! stand-ins for Nix and for the systems it switches between.
//!
//! The repository is the "remote": its main is what an update reads. The
//! stand-in nix builds a commit by giving the system folder made for it
//! (`World::system`), and fails for one marked to; a system's stand-in
//! switch points `running` at itself, and fails for one marked to. The
//! health check fails while the running system is marked unhealthy. So
//! every test reads, after an update, which system runs and what the
//! profile points at.
//!
//! A hosted world's main is the remote's, as `ls-remote` names it, with a
//! checkout of the test's own account holding its objects; the stand-in
//! nix records the inputs it's asked to fetch, and what the unpacked
//! flake's folder held.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use update_to_main::config::{Config, Hosted, Kind, Signers, Tools};
use update_to_main::line::{Applied, Line};
use update_to_main::{ask, serve, update};

/// The shell the stand-ins run under: the build's own in Nix's sandbox,
/// which has no /usr/bin/env.
fn shell() -> String {
    std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned())
}

/// A stand-in's first lines: its shell, and the tests' own path, since
/// update-to-main runs what it runs with a path of the system's, which a
/// build's sandbox doesn't have.
fn stand_in_head() -> String {
    format!(
        "#!{}\nPATH='{}'",
        shell(),
        std::env::var("PATH").unwrap_or_default()
    )
}

fn program(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("a PATH");
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{name} isn't on the path"))
}

struct World {
    _dir: tempfile::TempDir,
    root: PathBuf,
    origin: PathBuf,
    key: PathBuf,
    config: Config,
}

impl World {
    fn new() -> World {
        let dir = tempfile::Builder::new()
            .prefix("utm")
            .tempdir_in(if Path::new("/tmp").is_dir() {
                "/tmp"
            } else {
                "."
            })
            .unwrap();
        // Canonical, since what the switch goes back to is read through
        // the profile's links, and a Mac's /tmp is a link to /private/tmp.
        let root = fs::canonicalize(dir.path()).unwrap();
        let origin = root.join("origin");
        let key = root.join("key");
        let stand_ins = root.join("stand-ins");
        for d in [
            &origin,
            &stand_ins,
            &root.join("systems"),
            &root.join("state"),
        ] {
            fs::create_dir_all(d).unwrap();
        }
        keygen(&key);
        let signers = root.join("allowed_signers");
        let public = fs::read_to_string(key.with_extension("pub")).unwrap();
        fs::write(&signers, format!("joel namespaces=\"git\" {public}")).unwrap();
        git(&origin, &["init", "--quiet", "--initial-branch=main"]);

        let sh = shell();
        let head = stand_in_head();
        // nix build --print-out-paths --out-link LINK git+file://...&rev=REV#OUT,
        // or path:STATE/source/REV/FOLDER#OUT for a hosted main; and nix
        // eval or nix build --no-link for an input fetched by its lock.
        write_program(
            &stand_ins.join("nix"),
            &format!(
                "{head}\nfor a; do last=$a; done\n\
                 case \" $* \" in\n\
                 *' eval '*) echo \"$UPDATE_TO_MAIN_LOCKED\" >> {fetched}; echo /nix/store/x-source; exit 0;;\n\
                 *' --no-link '*) echo \"$last\" >> {fetched}; exit 0;;\n\
                 esac\n\
                 flake=$last\n\
                 case $flake in\n\
                 *rev=*) rev=${{flake##*rev=}}; rev=${{rev%%#*}};;\n\
                 *) rev=${{flake##*/source/}}; rev=${{rev%%/*}}; rev=${{rev%%#*}}\n\
                 folder=${{flake#path:}}; cat \"${{folder%%#*}}/file\" >> {seen};;\n\
                 esac\n\
                 echo \"$rev\" >> {log}\n\
                 if [ -e {systems}/$rev.fails ]; then echo 'error: builder failed' >&2; exit 1; fi\n\
                 echo {systems}/$rev\n",
                log = root.join("builds").display(),
                fetched = root.join("fetched").display(),
                seen = root.join("seen").display(),
                systems = root.join("systems").display()
            ),
        );
        // nix-env --profile PROFILE --set SYSTEM
        write_program(
            &stand_ins.join("nix-env"),
            &format!("{head}\nln -sfn \"$4\" \"$2\"\n"),
        );
        let config = Config {
            repository: format!("file://{}", origin.display()),
            branch: "main".to_owned(),
            output: "system".to_owned(),
            kind: Kind::Nixos,
            state: root.join("state"),
            socket: root.join("ask.sock"),
            group: own_group(),
            profile: root.join("profile"),
            signers: Signers {
                ssh: Some(signers),
                openpgp: Vec::new(),
            },
            hosted: None,
            health: vec![vec![
                sh.clone(),
                "-c".to_owned(),
                format!("test ! -e {}/unhealthy", root.join("running").display()),
            ]],
            health_within: 1,
            battery_floor: 20,
            power_supplies: root.join("power"),
            tools: Tools {
                git: program("git"),
                gpg: program("gpg"),
                ssh_keygen: program("ssh-keygen"),
                nix: stand_ins.join("nix"),
                nix_env: stand_ins.join("nix-env"),
                tar: program("tar"),
                pmset: None,
            },
            program: None,
            booted: root.join("booted"),
        };
        let world = World {
            _dir: dir,
            root,
            origin,
            key,
            config,
        };
        // The machine runs a system already, from before the first update.
        let first = world.system("first");
        fs::create_dir_all(&world.config.state).unwrap();
        std::os::unix::fs::symlink(&first, &world.config.profile).unwrap();
        std::os::unix::fs::symlink(&first, world.root.join("running")).unwrap();
        world
    }

    /// Commits to the remote's main, signed by the machine's trusted key.
    fn signed(&self, message: &str) -> String {
        fs::write(self.origin.join("file"), message).unwrap();
        git(&self.origin, &["add", "file"]);
        git(
            &self.origin,
            &[
                "-c",
                "gpg.format=ssh",
                "-c",
                &format!("user.signingkey={}", self.key.display()),
                "commit",
                "--quiet",
                "-S",
                "-m",
                message,
            ],
        );
        self.head()
    }

    /// A world whose main is the remote's word, its objects read from a
    /// checkout of the test's own account, which asks for a token with the
    /// shell command `token` gives for the world's folder.
    fn hosted_from(mut world: World, token: impl Fn(&Path) -> String) -> World {
        let token = token(&world.root);
        let checkout = world.root.join("checkout");
        git(
            &world.root,
            &["clone", "--quiet", &world.config.repository, "checkout"],
        );
        let path = std::env::var_os("PATH").expect("a PATH");
        world.config.signers.ssh = None;
        world.config.hosted = Some(Hosted {
            user: own_user(),
            checkout,
            directory: "machine".to_owned(),
            token: vec![shell(), "-c".to_owned(), token],
            path: std::env::split_paths(&path).collect(),
            inputs: Vec::new(),
        });
        world
    }

    /// Commits to the remote's main, unsigned, a flake in its machine folder
    /// beside a file saying `message`, and `lock` as its lock when given.
    fn hosted_commit(&self, message: &str, lock: Option<&str>) -> String {
        let machine = self.origin.join("machine");
        fs::create_dir_all(&machine).unwrap();
        fs::write(machine.join("flake.nix"), "{ outputs = _: { }; }\n").unwrap();
        fs::write(machine.join("file"), message).unwrap();
        if let Some(lock) = lock {
            fs::write(machine.join("flake.lock"), lock).unwrap();
        }
        git(&self.origin, &["add", "machine"]);
        git(&self.origin, &["commit", "--quiet", "-m", message]);
        self.head()
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.root.join(name)).unwrap_or_default()
    }

    /// Commits to the remote's main with no signature.
    fn unsigned(&self, message: &str) -> String {
        fs::write(self.origin.join("file"), message).unwrap();
        git(&self.origin, &["add", "file"]);
        git(&self.origin, &["commit", "--quiet", "-m", message]);
        self.head()
    }

    fn head(&self) -> String {
        git(&self.origin, &["rev-parse", "HEAD"])
    }

    /// The system the stand-in nix gives for `commit`, made if it isn't.
    fn system(&self, commit: &str) -> PathBuf {
        let system = self.root.join("systems").join(commit);
        if !system.exists() {
            fs::create_dir_all(system.join("bin")).unwrap();
            fs::write(system.join("nixos-version"), "test\n").unwrap();
            write_program(
                &system.join("bin/switch-to-configuration"),
                &format!(
                    "{head}\nhere=$(cd \"$(dirname \"$0\")/..\" && pwd)\n\
                     if [ -e \"$here/breaks\" ]; then echo 'activation failed' >&2; exit 1; fi\n\
                     ln -sfn \"$here\" {running}\necho \"restarting the following units: sshd.service\"\n",
                    head = stand_in_head(),
                    running = self.root.join("running").display()
                ),
            );
        }
        system
    }

    fn running(&self) -> PathBuf {
        fs::read_link(self.root.join("running")).unwrap()
    }

    fn booting(&self) -> PathBuf {
        fs::read_link(&self.config.profile).unwrap()
    }

    fn builds(&self) -> usize {
        fs::read_to_string(self.root.join("builds")).map_or(0, |s| s.lines().count())
    }

    fn update(&self) -> (Vec<Line>, Line) {
        let mut told = Vec::new();
        let ending = update::update(&self.config, &mut |l| told.push(l.clone()));
        (told, ending)
    }
}

fn keygen(key: &Path) {
    let done = Command::new(program("ssh-keygen"))
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "joel", "-f"])
        .arg(key)
        .status()
        .unwrap();
    assert!(done.success());
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(program("git"))
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Joel", "-c", "user.email=joel@example.com"])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn write_program(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn own_user() -> String {
    // SAFETY: getpwuid's record is read at once, before anything else
    // could call it.
    unsafe {
        let user = libc::getpwuid(libc::getuid());
        assert!(!user.is_null(), "the test's account has no name");
        std::ffi::CStr::from_ptr((*user).pw_name)
            .to_string_lossy()
            .into_owned()
    }
}

fn own_group() -> String {
    // SAFETY: getgrgid's record is read at once, before anything else
    // could call it.
    unsafe {
        let group = libc::getgrgid(libc::getgid());
        assert!(!group.is_null(), "the test's group has no name");
        std::ffi::CStr::from_ptr((*group).gr_name)
            .to_string_lossy()
            .into_owned()
    }
}

fn done(commit: &str) -> Line {
    Line::Done {
        commit: commit.to_owned(),
        applied: Applied::Now,
        restart: None,
        restarted: vec!["sshd.service".to_owned()],
    }
}

#[test]
fn a_signed_main_is_built_and_switched_to() {
    let world = World::new();
    let head = world.signed("one");
    let system = world.system(&head);
    let (told, ending) = world.update();
    assert_eq!(
        told,
        [
            Line::Fetching,
            Line::Building {
                commit: head.clone()
            },
            Line::Switching {
                commit: head.clone()
            },
        ]
    );
    assert_eq!(ending, done(&head));
    assert_eq!(world.running(), system);
    assert_eq!(world.booting(), system);
}

#[test]
fn an_ask_with_main_where_it_was_builds_nothing() {
    let world = World::new();
    let head = world.signed("one");
    world.system(&head);
    world.update();
    let (_, ending) = world.update();
    assert_eq!(ending, Line::Unchanged { commit: head });
    assert_eq!(world.builds(), 1);
}

#[test]
fn main_moved_on_is_switched_to_again() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    world.update();
    let two = world.signed("two");
    let system = world.system(&two);
    assert_eq!(world.update().1, done(&two));
    assert_eq!(world.running(), system);
}

#[test]
fn an_unsigned_main_is_refused_and_nothing_is_built() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    world.update();
    let first = world.running();
    let two = world.unsigned("two");
    world.system(&two);
    let (_, ending) = world.update();
    let Line::Refused { commit, why } = ending else {
        panic!("not refused: {ending:?}")
    };
    assert_eq!(commit.as_deref(), Some(one.as_str()));
    assert!(
        why.contains("isn't signed by a key this machine trusts"),
        "{why}"
    );
    assert_eq!(world.builds(), 1);
    assert_eq!(world.running(), first);
}

#[test]
fn a_main_signed_by_a_key_it_doesnt_trust_is_refused() {
    let world = World::new();
    keygen(&world.root.join("stranger"));
    fs::rename(world.root.join("stranger"), &world.key).unwrap();
    fs::rename(
        world.root.join("stranger.pub"),
        world.key.with_extension("pub"),
    )
    .unwrap();
    let head = world.signed("one");
    world.system(&head);
    let (_, ending) = world.update();
    assert!(
        matches!(ending, Line::Refused { commit: None, .. }),
        "{ending:?}"
    );
    assert_eq!(world.builds(), 0);
}

#[test]
fn a_main_signed_with_a_trusted_openpgp_key_is_switched_to() {
    let world = World::new();
    let (world, head) = openpgp_signed(world, "Merger");
    assert_eq!(world.update().1, done(&head));
}

#[test]
fn a_main_signed_with_an_openpgp_key_it_wasnt_given_is_refused() {
    let world = World::new();
    let (mut world, _) = openpgp_signed(world, "Merger");
    let (other, _) = openpgp_signed(World::new(), "Stranger");
    world.config.signers.openpgp = other.config.signers.openpgp.clone();
    let (_, ending) = world.update();
    assert!(
        matches!(ending, Line::Refused { commit: None, .. }),
        "{ending:?}"
    );
    assert_eq!(world.builds(), 0);
}

/// Makes a PGP key for `who`, gives it to the config as one that
/// counts, and commits to main signed with it, as GitHub signs a merge.
fn openpgp_signed(mut world: World, who: &str) -> (World, String) {
    let email = format!("{}@example.com", who.to_lowercase());
    let home = world.root.join("g");
    fs::create_dir(&home).unwrap();
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    let gpg = |args: &[&str]| {
        let out = Command::new(program("gpg"))
            .env("GNUPGHOME", &home)
            .args([
                "--batch",
                "--quiet",
                "--pinentry-mode",
                "loopback",
                "--passphrase",
                "",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "gpg {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    gpg(&[
        "--quick-gen-key",
        &format!("{who} <{email}>"),
        "ed25519",
        "sign",
        "never",
    ]);
    let armoured = world.root.join("signer.asc");
    fs::write(&armoured, gpg(&["--armor", "--export", &email])).unwrap();
    world.config.signers.openpgp = vec![armoured];
    fs::write(world.origin.join("file"), "merged").unwrap();
    git(&world.origin, &["add", "file"]);
    let out = Command::new(program("git"))
        .arg("-C")
        .arg(&world.origin)
        .args([
            "-c",
            &format!("user.name={who}"),
            "-c",
            &format!("user.email={email}"),
        ])
        .args(["-c", &format!("user.signingkey={email}")])
        .args(["commit", "--quiet", "-S", "-m", "merged"])
        .env("GNUPGHOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let head = world.head();
    world.system(&head);
    (world, head)
}

#[test]
fn a_rewritten_main_is_refused() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    world.update();
    git(&world.origin, &["checkout", "--quiet", "--orphan", "other"]);
    let other = world.signed("another history");
    git(&world.origin, &["branch", "--quiet", "-f", "main", &other]);
    world.system(&other);
    let (_, ending) = world.update();
    let Line::Refused { commit, why } = ending else {
        panic!("not refused: {ending:?}")
    };
    assert_eq!(commit.as_deref(), Some(one.as_str()));
    assert!(why.contains("doesn't descend from"), "{why}");
    assert_eq!(world.running(), world.system(&one));
}

#[test]
fn a_failed_build_leaves_the_machine_as_it_was() {
    let world = World::new();
    let first = world.running();
    let head = world.signed("one");
    fs::write(world.root.join("systems").join(format!("{head}.fails")), "").unwrap();
    let (_, ending) = world.update();
    let Line::Failed { commit, why, lines } = ending else {
        panic!("not failed: {ending:?}")
    };
    assert_eq!(commit, None);
    assert!(why.contains("building the system failed"), "{why}");
    assert_eq!(lines, ["error: builder failed"]);
    assert_eq!(world.running(), first);
    assert_eq!(world.booting(), first);
}

#[test]
fn a_switch_that_fails_goes_back_to_what_it_ran() {
    let world = World::new();
    let first = world.running();
    let head = world.signed("one");
    fs::write(world.system(&head).join("breaks"), "").unwrap();
    let (_, ending) = world.update();
    let Line::Failed { why, lines, .. } = ending else {
        panic!("not failed: {ending:?}")
    };
    assert!(why.contains("switching failed"), "{why}");
    assert!(why.contains("went back to what it ran"), "{why}");
    assert_eq!(lines, ["activation failed"]);
    assert_eq!(world.running(), first);
    assert_eq!(world.booting(), first);
    // Not remembered, so the next ask tries main again.
    assert!(update::last(&world.config).is_none());
}

#[test]
fn a_machine_not_whole_after_its_switch_goes_back() {
    let world = World::new();
    let first = world.running();
    let head = world.signed("one");
    fs::write(world.system(&head).join("unhealthy"), "").unwrap();
    let (_, ending) = world.update();
    let Line::Failed { why, .. } = ending else {
        panic!("not failed: {ending:?}")
    };
    assert!(why.contains("the health check"), "{why}");
    assert!(why.contains("went back to what it ran"), "{why}");
    assert_eq!(world.running(), first);
    assert_eq!(world.booting(), first);
}

#[test]
fn a_new_kernel_is_left_for_the_next_boot() {
    let world = World::new();
    let first = world.running();
    for (system, kernel) in [(&first, "one"), (&world.root.join("booted"), "one")] {
        fs::create_dir_all(system).unwrap();
        fs::write(system.join("kernel"), kernel).unwrap();
    }
    let head = world.signed("one");
    let system = world.system(&head);
    fs::write(system.join("kernel"), "two").unwrap();
    // The stand-in's boot mode points the profile only, as NixOS's does.
    write_program(
        &system.join("bin/switch-to-configuration"),
        &format!("{}\n[ \"$1\" = boot ] || exit 9\n", stand_in_head()),
    );
    let (_, ending) = world.update();
    assert_eq!(
        ending,
        Line::Done {
            commit: head,
            applied: Applied::NextBoot,
            restart: Some("its kernel changed".to_owned()),
            restarted: Vec::new(),
        }
    );
    assert_eq!(world.running(), first);
    assert_eq!(world.booting(), system);
}

#[test]
fn a_low_battery_starts_nothing() {
    let world = World::new();
    for (name, files) in [
        ("BAT0", [("type", "Battery"), ("capacity", "12")]),
        ("AC", [("type", "Mains"), ("online", "0")]),
    ] {
        let dir = world.config.power_supplies.join(name);
        fs::create_dir_all(&dir).unwrap();
        for (file, text) in files {
            fs::write(dir.join(file), text).unwrap();
        }
    }
    let head = world.signed("one");
    world.system(&head);
    let (told, ending) = world.update();
    assert!(told.is_empty());
    let Line::Refused { why, .. } = ending else {
        panic!("not refused: {ending:?}")
    };
    assert!(why.contains("on battery at 12%"), "{why}");
    assert_eq!(world.builds(), 0);
}

#[test]
fn asks_while_an_update_runs_join_it() {
    let world = World::new();
    let main = world.signed("one");
    world.system(&main);
    // The build waits for a file, so both asks come while it runs.
    let gate = world.root.join("gate");
    let nix = world.config.tools.nix.clone();
    let text = fs::read_to_string(&nix).unwrap();
    let waiting = text.replacen(
        "\nfor a;",
        &format!(
            "\nwhile [ ! -e {} ]; do sleep 0.05; done\nfor a;",
            gate.display()
        ),
        1,
    );
    write_program(&nix, &waiting);

    let config = world.config.clone();
    thread::spawn(move || serve::serve(config));
    let socket = world.config.socket.clone();
    while !socket.exists() {
        thread::sleep(std::time::Duration::from_millis(20));
    }
    let askers: Vec<_> = (0..2)
        .map(|_| {
            let socket = socket.clone();
            thread::spawn(move || {
                let mut told = Vec::new();
                let ending = ask::ask(&socket, &mut |l| told.push(l.clone())).unwrap();
                (told, ending)
            })
        })
        .collect();
    thread::sleep(std::time::Duration::from_millis(300));
    fs::write(&gate, "").unwrap();
    for asker in askers {
        let (told, ending) = asker.join().unwrap();
        assert_eq!(told.first(), Some(&Line::Fetching));
        assert_eq!(ending, done(&main));
    }
    assert_eq!(world.builds(), 1);
}

#[test]
fn anything_but_the_one_ask_is_refused() {
    use std::io::{BufRead, BufReader, Write};
    let world = World::new();
    let config = world.config.clone();
    thread::spawn(move || serve::serve(config));
    while !world.config.socket.exists() {
        thread::sleep(std::time::Duration::from_millis(20));
    }
    let mut stream = std::os::unix::net::UnixStream::connect(&world.config.socket).unwrap();
    writeln!(stream, "update /nix/store/something").unwrap();
    let mut said = String::new();
    BufReader::new(stream).read_line(&mut said).unwrap();
    let line: Line = serde_json::from_str(&said).unwrap();
    assert!(matches!(line, Line::Refused { .. }), "{line:?}");
    assert_eq!(world.builds(), 0);
}

#[test]
fn the_socket_is_only_its_groups_to_write() {
    use std::os::unix::fs::MetadataExt;
    let world = World::new();
    let config = world.config.clone();
    thread::spawn(move || serve::serve(config));
    while !world.config.socket.exists() {
        thread::sleep(std::time::Duration::from_millis(20));
    }
    let meta = fs::metadata(&world.config.socket).unwrap();
    assert_eq!(meta.mode() & 0o777, 0o660);
    // SAFETY: getgid can't fail.
    assert_eq!(meta.gid(), unsafe { libc::getgid() });
}

/// A token command that counts its asks and gives a token.
fn counted_token(world_root: &Path) -> String {
    format!(
        "echo asked >> {}; echo a-token",
        world_root.join("tokens").display()
    )
}

#[test]
fn a_hosted_main_is_built_from_its_folder_unsigned() {
    let world = World::hosted_from(World::new(), counted_token);
    let head = world.hosted_commit("one", None);
    let system = world.system(&head);
    let (told, ending) = world.update();
    assert_eq!(told.len(), 3, "{told:?}");
    assert_eq!(ending, done(&head));
    assert_eq!(world.running(), system);
    assert_eq!(world.read("seen"), "one");
    assert_eq!(world.read("tokens"), "asked\n");
}

#[test]
fn a_hosted_main_the_checkout_lacks_is_fetched_there_first() {
    let world = World::hosted_from(World::new(), counted_token);
    let head = world.hosted_commit("one", None);
    world.system(&head);
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    assert_eq!(world.update().1, done(&head));
    git(&checkout, &["cat-file", "-e", &head]);
    // Root's copy holds none of the objects itself: it reads the checkout's.
    let copy = world.config.state.join("repository.git/objects");
    let own = fs::read_dir(&copy)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().len() == 2)
        .count();
    assert_eq!(own, 0);
}

#[test]
fn a_hosted_main_with_no_token_builds_nothing() {
    let world = World::hosted_from(World::new(), |_| "echo no login >&2; exit 3".to_owned());
    let head = world.hosted_commit("one", None);
    world.system(&head);
    let (_, ending) = world.update();
    let Line::Failed { why, lines, .. } = ending else {
        panic!("not failed: {ending:?}")
    };
    assert!(why.contains("asking for a token failed"), "{why}");
    assert_eq!(lines, ["no login"]);
    assert_eq!(world.builds(), 0);
}

#[test]
fn a_hosted_mains_private_inputs_are_fetched_by_their_locks() {
    let mut world = World::hosted_from(World::new(), counted_token);
    world.config.hosted.as_mut().unwrap().inputs = vec!["private".to_owned(), "pinned".to_owned()];
    let lock = r#"{
      "root": "root",
      "nodes": {
        "root": { "inputs": { "private": "private_2", "pinned": "pinned", "public": "public" } },
        "private_2": { "locked": { "type": "git", "url": "https://example.com/private", "rev": "abc" } },
        "pinned": { "locked": { "type": "path", "path": "/nix/store/abc-pinned" } },
        "public": { "locked": { "type": "github", "owner": "o", "repo": "r" } }
      },
      "version": 7
    }"#;
    let head = world.hosted_commit("one", Some(lock));
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let fetched = world.read("fetched");
    let fetched: Vec<&str> = fetched.lines().collect();
    assert_eq!(fetched.len(), 2, "{fetched:?}");
    let private: serde_json::Value = serde_json::from_str(fetched[0]).unwrap();
    assert_eq!(private["url"], "https://example.com/private");
    assert_eq!(fetched[1], "/nix/store/abc-pinned");
}

#[test]
fn a_hosted_main_rewritten_is_refused() {
    let world = World::hosted_from(World::new(), counted_token);
    let one = world.hosted_commit("one", None);
    world.system(&one);
    world.update();
    git(&world.origin, &["checkout", "--quiet", "--orphan", "other"]);
    let other = world.hosted_commit("another history", None);
    git(&world.origin, &["branch", "--quiet", "-f", "main", &other]);
    world.system(&other);
    let (_, ending) = world.update();
    let Line::Refused { why, .. } = ending else {
        panic!("not refused: {ending:?}")
    };
    assert!(why.contains("doesn't descend from"), "{why}");
    assert_eq!(world.builds(), 1);
}
