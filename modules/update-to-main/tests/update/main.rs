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

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use update_to_main::config::{
    Batches, Busy, Config, Follow, Here, Holds, Hosted, Kept, Kind, Says, Session, Signers, Tools,
};
use update_to_main::line::{Applied, Line};
use update_to_main::{ask, held, serve, update};

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

        let head = stand_in_head();
        // nix build --print-out-paths --out-link LINK git+file://...&rev=REV#OUT,
        // or path:STATE/source/REV/FOLDER#OUT for a hosted main; and nix
        // eval or nix build --no-link for an input fetched by its lock. A
        // build elsewhere first lists a light piece, a heavy one and a
        // wrapper built from a secret piece built already, and sends the
        // heavy ones away, which fails while no-builder is there; kept ones
        // go to their own builders, failing while no-kept-builder is there.
        // Either may be built here, small, which writes them in here. While
        // big is there, a big piece and modules built from it are needed
        // too; while big-fails is, the big one fails at once, and while
        // big-waits is, it waits up to five seconds for the heavy piece to
        // be sent beside it, failing if it never is. While drops-once is
        // there, the next build on the builders loses its builder, as a
        // corrupted tunnel did on 11 November. A dry run lists nothing sent
        // away or built here already, and while collected is there, every
        // dry run after the first lists a big piece too, as one collected
        // between them would be, which fails on the builders while
        // collected-fails is there. While lighter is there, a second light
        // piece is needed too. A batch built here is
        // written in batched, and the pieces linked to be kept in kept, with
        // the outputs of the secret piece the wrapper is built from. A system's
        // build starts a call while call-starts is there. Each input
        // overridden is written down, and a system made for a commit with
        // its input at a rev, as COMMIT+REV, is what they build.
        // Otherwise the system is the commit's own.
        write_program(
            &stand_ins.join("nix"),
            &format!(
                "{head}\nprev=\nbefore=\ninput=\nfor a; do\n\
                 [ \"$before\" = --override-input ] && echo \"$prev $a\" >> {overridden} \
                 && input=${{a##*rev=}} && input=${{input%%&*}}\n\
                 before=$prev; prev=$a; last=$a; done\n\
                 case \" $* \" in\n\
                 *' --dry-run '*) echo 'these derivations will be built:' >&2; \
                 for p in a-light b-heavy c-wrapper t-secret-tool \
                 $([ -e {big} ] && echo e-big d-modules) $([ -e {lighter} ] && echo a-lighter) \
                 $([ -e {collected} ] && [ -e {listed} ] && echo g-collected); do \
                 cat {away} {here} 2>/dev/null | grep -q \"/nix/store/$p.drv\" \
                 || echo \"  /nix/store/$p.drv\" >&2; done; touch {listed}; exit 0;;\n\
                 *' derivation show /nix/store/s-secret.drv '*) echo '{{\"derivations\": \
                 {{\"s-secret.drv\": {{\"outputs\": {{\"out\": {{\"path\": \"{secret_out}\"}}, \
                 \"doc\": {{\"path\": \"{secret_out}-doc\"}}}}}}}}}}'; exit 0;;\n\
                 *' derivation show '*) secret=\n\
                 collected=; [ -e {collected} ] && collected=', \"g-collected.drv\": \
                 {{\"name\": \"linux-7\", \"env\": {{\"requiredSystemFeatures\": \
                 \"big-parallel\"}}}}'\n\
                 lighter=; [ -e {lighter} ] && lighter=', \"a-lighter.drv\": \
                 {{\"env\": {{\"preferLocalBuild\": \"1\"}}}}'\n\
                 big=; [ -e {big} ] && big=', \"e-big.drv\": {{\"name\": \"big-1\", \
                 \"env\": {{\"requiredSystemFeatures\": \"big-parallel\"}}}}, \
                 \"d-modules.drv\": {{\"name\": \"modules\", \"env\": {{}}, \
                 \"inputs\": {{\"drvs\": {{\"e-big.drv\": {{}}}}}}}}'\n\
                 case \" $* \" in *' --recursive '*) secret=', \"s-secret.drv\": \
                 {{\"name\": \"secret-1.0\", \"env\": {{}}}}';; esac\n\
                 echo '{{\"derivations\": {{\"a-light.drv\": \
                 {{\"env\": {{\"preferLocalBuild\": \"1\"}}}}, \"b-heavy.drv\": {{\"env\": {{}}}}, \
                 \"c-wrapper.drv\": {{\"name\": \"wrapper\", \"env\": {{}}, \
                 \"inputs\": {{\"drvs\": {{\"s-secret.drv\": {{}}}}}}}}, \
                 \"t-secret-tool.drv\": {{\"name\": \"secret-tool-2\", \"env\": {{}}}}\
                 '\"$secret$big$collected$lighter\"'}}}}'; exit 0;;\n\
                 *'/evaluated/drv '*|*'/evaluated/built '*) echo \"$*\" >> {kept}; exit 0;;\n\
                 *' --cores '*) echo \"$*\" >> {here}; exit 0;;\n\
                 *' build --no-link -L '*|*' build --no-link --builders  -L '*) \
                 echo \"$*\" >> {batched}; exit 0;;\n\
                 *' --builders @/etc/kept-builders '*) if [ -e {no_kept_builder} ]; then \
                 echo 'no kept builder answers' >&2; exit 1; fi; echo \"$*\" >> {away}; exit 0;;\n\
                 *' --max-jobs 0 '*) case \" $* \" in *g-collected*)\n\
                 if [ -e {collected_fails} ]; then echo 'error: g-collected failed' >&2; \
                 exit 1; fi;; esac\n\
                 case \" $* \" in *e-big*)\n\
                 if [ -e {big_fails} ]; then echo 'error: e-big failed' >&2; exit 1; fi\n\
                 if [ -e {big_waits} ]; then n=0; while :; do \
                 case $(cat {away} 2>/dev/null) in *b-heavy*) break;; esac; n=$((n+1)); \
                 if [ $n -gt 50 ]; then echo 'nothing was built beside it' >&2; exit 1; fi; \
                 sleep 0.1; done; fi;; esac\n\
                 if [ -e {hung} ]; then sleep 30; fi\n\
                 if [ -e {drops_once} ]; then rm {drops_once}; \
                 echo 'Connection corrupted' >&2; \
                 echo 'Reason: local builds are disabled (max-jobs = 0)' >&2; exit 1; fi\n\
                 if [ -e {no_builder} ] || {{ [ -e {no_small_builder} ] \
                 && [ \"${{*#*e-big}}\" = \"$*\" ]; }}; then \
                 echo 'local builds are disabled' >&2; exit 1; fi; echo \"$*\" >> {away}; exit 0;;\n\
                 *' eval '*) if [ -e {fetch_fails} ]; then \
                 echo 'could not read Username' >&2; exit 1; fi; \
                 echo \"$UPDATE_TO_MAIN_LOCKED\" >> {fetched}; echo /nix/store/x-source; exit 0;;\n\
                 *' --no-link '*) echo \"$last\" >> {fetched}; exit 0;;\n\
                 esac\n\
                 flake=$last\n\
                 case $flake in\n\
                 *rev=*) rev=${{flake##*rev=}}; rev=${{rev%%#*}};;\n\
                 *) rev=${{flake##*/source/}}; rev=${{rev%%/*}}; rev=${{rev%%#*}}\n\
                 folder=${{flake#path:}}; cat \"${{folder%%#*}}/file\" >> {seen};;\n\
                 esac\n\
                 echo \"$*\" > {last_build}\n\
                 echo \"$rev\" >> {log}\n\
                 if [ -e {call_starts} ]; then touch {call}; fi\n\
                 if [ -e {systems}/$rev.fails ]; then echo 'error: builder failed' >&2; exit 1; fi\n\
                 if [ -n \"$input\" ] && [ -e {systems}/$rev+$input ]; then rev=$rev+$input; fi\n\
                 echo {systems}/$rev\n",
                log = root.join("builds").display(),
                fetched = root.join("fetched").display(),
                fetch_fails = root.join("fetch-fails").display(),
                overridden = root.join("overridden").display(),
                no_builder = root.join("no-builder").display(),
                hung = root.join("hung").display(),
                drops_once = root.join("drops-once").display(),
                no_small_builder = root.join("no-small-builder").display(),
                big = root.join("big").display(),
                lighter = root.join("lighter").display(),
                collected = root.join("collected").display(),
                collected_fails = root.join("collected-fails").display(),
                listed = root.join("listed").display(),
                big_fails = root.join("big-fails").display(),
                big_waits = root.join("big-waits").display(),
                no_kept_builder = root.join("no-kept-builder").display(),
                here = root.join("here").display(),
                batched = root.join("batched").display(),
                kept = root.join("kept").display(),
                secret_out = root.join("secret-out").display(),
                call_starts = root.join("call-starts").display(),
                call = root.join("call").display(),
                away = root.join("away").display(),
                seen = root.join("seen").display(),
                last_build = root.join("last-build").display(),
                systems = root.join("systems").display()
            ),
        );
        // nix-env --profile PROFILE --set SYSTEM
        write_program(
            &stand_ins.join("nix-env"),
            &format!("{head}\nln -sfn \"$4\" \"$2\"\n"),
        );
        // journalctl -u UNIT ...: the last lines a hung home-manager said
        // before systemd gave up on it, as on 16 November.
        write_program(
            &stand_ins.join("journalctl"),
            &format!(
                "{head}\ncase \" $* \" in *' -u home-manager-joel.service '*) \
                 echo 'Activating reloadSystemd'; echo 'start operation timed out';; esac\n"
            ),
        );
        let config = World::config_in(&root);
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
        std::os::unix::fs::symlink(&first, world.root.join("boots")).unwrap();
        world
    }

    /// The config of the world made in `root`, which names only paths in
    /// it, so a process apart can serve the same world.
    fn config_in(root: &Path) -> Config {
        let stand_ins = root.join("stand-ins");
        Config {
            repository: format!("file://{}", root.join("origin").display()),
            branch: "main".to_owned(),
            output: "system".to_owned(),
            kind: Kind::Nixos,
            state: root.join("state"),
            socket: root.join("ask.sock"),
            group: own_group(),
            ended: root.join("last.json"),
            running: root.join("now.json"),
            profile: root.join("profile"),
            current: root.join("running"),
            signers: Signers {
                ssh: Some(root.join("allowed_signers")),
                openpgp: Vec::new(),
            },
            hosted: None,
            follows: BTreeMap::new(),
            elsewhere: false,
            builders: None,
            kept: Kept::default(),
            here: None,
            health: vec![vec![
                shell(),
                "-c".to_owned(),
                format!("test ! -e {}/unhealthy", root.join("running").display()),
            ]],
            health_within: 1,
            boot_check: None,
            battery_floor: 20,
            refuse_while: Vec::new(),
            follower_tells: None,
            holds: None,
            power_supplies: root.join("power"),
            cpu_pressure: root.join("pressure"),
            batches: None,
            memory: root.join("meminfo"),
            silent_for: 2,
            tools: Tools {
                git: program("git"),
                gpg: program("gpg"),
                ssh_keygen: program("ssh-keygen"),
                nix: stand_ins.join("nix"),
                nix_env: stand_ins.join("nix-env"),
                tar: program("tar"),
                curl: program("curl"),
                pmset: None,
                journalctl: Some(stand_ins.join("journalctl")),
            },
            session: None,
            program: None,
            booted: root.join("booted"),
        }
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
            changed: None,
            api: None,
            may_fetch: None,
            fetched: None,
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
            fs::write(system.join("darwin-version"), "test\n").unwrap();
            write_program(
                &system.join("bin/switch-to-configuration"),
                &format!(
                    "{head}\nhere=$(cd \"$(dirname \"$0\")/..\" && pwd)\n\
                     if [ -e \"$here/breaks\" ]; then echo 'activation failed' >&2; exit 1; fi\n\
                     if [ -e \"$here/unit-fails\" ]; then echo 'warning: the following units \
                     failed: home-manager-joel.service' >&2; exit 4; fi\n\
                     echo \"$1\" >> {modes}\n\
                     case \"$1\" in switch|boot)\n\
                     ln -sfn \"$(readlink -f {profile})\" {boots};; esac\n\
                     case \"$1\" in switch|test) ln -sfn \"$here\" {running}\n\
                     echo \"restarting the following units: sshd.service\";; esac\n",
                    head = stand_in_head(),
                    modes = self.root.join("modes").display(),
                    profile = self.config.profile.display(),
                    boots = self.root.join("boots").display(),
                    running = self.root.join("running").display()
                ),
            );
            // A Mac's activate, which points `running` at itself last, as
            // nix-darwin's does. While stops-its-daemon is there it first
            // stops the update-to-main running it, as launchd does when
            // activation reloads that daemon, and says more after.
            write_program(
                &system.join("activate"),
                &format!(
                    "{head}\nhere=$(cd \"$(dirname \"$0\")\" && pwd)\n\
                     if [ -e \"$here/breaks\" ]; then echo 'activation failed' >&2; exit 1; fi\n\
                     if [ -e \"$here/stops-its-daemon\" ]; then kill -KILL \"$PPID\"; \
                     sleep 1; fi\n\
                     echo 'setting up launchd services...'\n\
                     echo 'reloading service org.jitsusama.update-to-main' >&2\n\
                     ln -sfn \"$here\" {running}\n",
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

    /// The system the boot loader starts next, which only
    /// switch-to-configuration's `switch` and `boot` set, to what the
    /// profile names then rather than to the system they were run from, as
    /// lanzaboote's loader starts the profile's newest generation.
    fn boots(&self) -> PathBuf {
        fs::read_link(self.root.join("boots")).unwrap()
    }

    /// How switch-to-configuration was run, in order.
    fn modes(&self) -> Vec<String> {
        let modes = fs::read_to_string(self.root.join("modes")).unwrap_or_default();
        modes.lines().map(str::to_owned).collect()
    }

    fn builds(&self) -> usize {
        fs::read_to_string(self.root.join("builds")).map_or(0, |s| s.lines().count())
    }

    fn update(&self) -> (Vec<Line>, Line) {
        self.asked_by(&|| false)
    }

    /// Updates as the follower asks, `following` saying whether only it
    /// has asked still.
    fn asked_by(&self, following: &dyn Fn() -> bool) -> (Vec<Line>, Line) {
        let mut told = Vec::new();
        let ending = update::update(&self.config, following, &mut |l| told.push(l.clone()));
        (told, ending)
    }

    /// Updates as a person asks, with the followed input `name` at
    /// `commit` rather than its main's head.
    fn update_at(&self, name: &str, commit: &str) -> Line {
        let at = BTreeMap::from([(name.to_owned(), commit.to_owned())]);
        update::update_at(&self.config, &|| false, &at, &mut |_| {})
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
        boots: None,
        followed: BTreeMap::new(),
    }
}

#[test]
fn nix_finds_roots_git_where_the_system_has_none() {
    let mut world = World::new();
    let head = world.signed("one");
    let system = world.system(&head);
    // Nix runs git to fetch main by its git+file reference, and fails as
    // Nix does when git isn't first on the path it's given.
    let nix = world.config.tools.nix.clone();
    let wrapped = world.root.join("stand-ins/nix-wanting-git");
    write_program(
        &wrapped,
        &format!(
            "#!{}\ncase \"$PATH\" in '{}':*) ;; *) echo 'executing \"git\": No such file or directory' >&2; exit 1 ;; esac\nexec '{}' \"$@\"\n",
            shell(),
            world.config.tools.git.parent().unwrap().display(),
            nix.display()
        ),
    );
    world.config.tools.nix = wrapped;
    assert_eq!(world.update().1, done(&head));
    assert_eq!(world.running(), system);
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
    assert_eq!(
        ending,
        Line::Unchanged {
            commit: head,
            followed: BTreeMap::new(),
            boots: None,
        }
    );
    assert_eq!(world.builds(), 1);
}

#[test]
fn a_system_the_profile_names_but_nothing_activated_is_switched_to() {
    // On 17 November a Mac's switch died after pointing the profile at the
    // new system and before activating it, and every update after said
    // nothing had changed, since the profile already named what it built.
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    world.update();
    let two = world.signed("two");
    let system = world.system(&two);
    fs::remove_file(&world.config.profile).unwrap();
    std::os::unix::fs::symlink(&system, &world.config.profile).unwrap();
    let (_, ending) = world.update();
    assert_eq!(ending, done(&two));
    assert_eq!(
        world.running(),
        system,
        "the system the profile named wasn't activated"
    );
}

#[test]
fn a_profile_left_behind_what_runs_is_switched_again() {
    // A machine running another system than its profile names, as one
    // activated by hand leaves, boots and rolls back by the wrong one, so
    // the next head, though it builds what runs, switches again.
    let world = World::new();
    let one = world.signed("one");
    let system = world.system(&one);
    world.update();
    let first = world.system("first");
    fs::remove_file(&world.config.profile).unwrap();
    std::os::unix::fs::symlink(&first, &world.config.profile).unwrap();
    let two = world.signed("two");
    std::os::unix::fs::symlink(&system, world.root.join("systems").join(&two)).unwrap();
    let (_, ending) = world.update();
    assert_eq!(ending, done(&two));
    assert_eq!(
        fs::canonicalize(world.booting()).unwrap(),
        fs::canonicalize(&system).unwrap(),
        "the profile was left behind"
    );
}

/// Not a test of its own: the daemon of the Mac's world made in
/// `UPDATE_TO_MAIN_WORLD`, served in a process apart by
/// `a_macs_switch_outlives_the_daemon_its_activation_stops`, so the
/// activation can stop it as launchd would.
#[test]
#[ignore = "a_macs_switch_outlives_the_daemon_its_activation_stops runs it apart"]
fn serves_a_macs_world_apart() {
    let Some(root) = std::env::var_os("UPDATE_TO_MAIN_WORLD") else {
        return;
    };
    let mut config = World::config_in(Path::new(&root));
    config.kind = Kind::Darwin;
    serve::serve(config).unwrap();
}

#[test]
fn a_macs_switch_outlives_the_daemon_its_activation_stops() {
    // On 17 November a Mac's activation reloaded the daemon running it, for
    // its new open-files limit. The daemon's end closed activation's output,
    // and activation died writing to it, before it moved the running system.
    let mut world = World::new();
    let head = world.signed("one");
    let system = world.system(&head);
    fs::write(system.join("stops-its-daemon"), "").unwrap();
    let mut apart = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "serves_a_macs_world_apart",
            "--ignored",
            "--nocapture",
        ])
        .env("UPDATE_TO_MAIN_WORLD", &world.root)
        .stdout(fs::File::create(world.root.join("apart.out")).unwrap())
        .stderr(fs::File::create(world.root.join("apart.err")).unwrap())
        .spawn()
        .unwrap();
    let by = Instant::now() + Duration::from_secs(30);
    while !world.config.socket.exists() && Instant::now() < by {
        thread::sleep(Duration::from_millis(20));
    }
    // The daemon is stopped mid-switch, so the ask ends without an ending.
    let socket = world.config.socket.clone();
    let asking = thread::spawn(move || ask::ask(&socket, &mut |_| {}));
    let stopped = loop {
        if let Some(status) = apart.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() > by {
            let _ = apart.kill();
            let _ = apart.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(50));
    };
    let told = || format!("{}{}", world.read("apart.out"), world.read("apart.err"));
    assert!(
        stopped.is_some_and(|status| status.signal() == Some(libc::SIGKILL)),
        "the daemon wasn't stopped by its switch, which ended {stopped:?} and asked {:?}: {}",
        asking.is_finished().then(|| asking.join().unwrap()),
        told()
    );
    while world.running() != system && Instant::now() < by {
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        world.running(),
        system,
        "the switch died with the daemon its activation stopped: {}",
        told()
    );
    let left: serde_json::Value =
        serde_json::from_str(&world.read("state/switching.json")).unwrap();
    assert_eq!(
        left["previous"].as_str().map(PathBuf::from),
        fs::canonicalize(world.system("first")).ok(),
        "the switch left didn't keep what ran before it"
    );
    // The daemon launchd starts again finds the switch it began unended,
    // and makes it again with its checks rather than saying nothing
    // changed.
    fs::remove_file(system.join("stops-its-daemon")).unwrap();
    world.config.kind = Kind::Darwin;
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Done { commit, .. } if *commit == head),
        "the switch the daemon left wasn't made again: {ending:?}"
    );
    assert!(
        !world.config.state.join("switching.json").exists(),
        "the switch made again left its marker"
    );
}

#[test]
fn a_macs_failed_activation_goes_back_keeping_what_it_said() {
    // A Mac's activation writes to a log, which its switch back writes to
    // as well, after it.
    let mut world = World::new();
    world.config.kind = Kind::Darwin;
    let one = world.signed("one");
    let first = world.system(&one);
    world.update();
    let two = world.signed("two");
    let system = world.system(&two);
    fs::write(system.join("breaks"), "").unwrap();
    let (_, ending) = world.update();
    let Line::Failed { why, .. } = ending else {
        panic!("a broken activation didn't fail: {ending:?}");
    };
    assert!(why.contains("went back"), "{why}");
    assert_eq!(world.running(), first, "it didn't go back: {why}");
    let log = world.read("state/activation.log");
    assert!(
        log.contains("activation failed"),
        "the switch back cleared what the failed one said: {log}"
    );
    assert!(
        log.rfind("setting up launchd services...") > log.find("activation failed"),
        "the switch back said nothing after the failed one: {log}"
    );
}

#[test]
fn a_switch_its_daemon_left_goes_back_to_what_ran_before_it() {
    // A daemon stopped mid-switch to one, whose activation ran on alone,
    // so one was never checked; a broken switch after goes back past it.
    let mut world = World::new();
    world.config.kind = Kind::Darwin;
    let one = world.signed("one");
    let left = world.system(&one);
    world.update();
    let first = world.system("first");
    fs::write(
        world.config.state.join("switching.json"),
        serde_json::json!({ "system": left, "previous": first }).to_string(),
    )
    .unwrap();
    let two = world.signed("two");
    fs::write(world.system(&two).join("breaks"), "").unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Failed { why, .. } if why.contains("went back")),
        "a broken activation didn't go back: {ending:?}"
    );
    assert_eq!(
        world.running(),
        first,
        "it went back to the system the left switch never checked"
    );
    assert!(
        !world.config.state.join("switching.json").exists(),
        "going back left the switch marked unended"
    );
}

#[test]
fn a_macs_activation_waits_for_one_still_holding_its_log() {
    // An activation a stopped daemon left still runs, so the one its
    // restarted daemon begins waits for it rather than running beside it.
    use std::os::fd::AsRawFd;
    let mut world = World::new();
    world.config.kind = Kind::Darwin;
    let one = world.signed("one");
    let system = world.system(&one);
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(world.config.state.join("activation.log"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(log.as_raw_fd(), libc::LOCK_EX) },
        0,
        "the test couldn't hold the log"
    );
    let profile = world.config.profile.clone();
    let switched_to = fs::canonicalize(&system).ok();
    let letting_go = thread::spawn(move || {
        let by = Instant::now() + Duration::from_secs(60);
        while fs::canonicalize(&profile).ok() != switched_to && Instant::now() < by {
            thread::sleep(Duration::from_millis(20));
        }
        // A Mac's profile moves just before activation begins, which with
        // nothing waiting writes to the log within this.
        thread::sleep(ACTIVATION_BEGINS);
        let written = log.metadata().unwrap().len();
        drop(log);
        written
    });
    let (_, ending) = world.update();
    assert_eq!(
        letting_go.join().unwrap(),
        0,
        "the activation wrote to its log while another held it"
    );
    assert!(
        matches!(&ending, Line::Done { commit, .. } if *commit == one),
        "{ending:?}"
    );
    assert_eq!(world.running(), system);
}

/// How long a switch, marked begun, takes at most to begin activating.
const ACTIVATION_BEGINS: Duration = Duration::from_secs(2);

#[test]
fn a_macs_going_back_cut_off_is_finished_not_switched_again() {
    // Going back from a broken switch reloads a Mac's daemon as surely as
    // the switch did, and cut off it left its failure unsaid; switching
    // again would only flip between the two.
    let mut world = World::new();
    world.config.kind = Kind::Darwin;
    let one = world.signed("one");
    let broken = world.system(&one);
    fs::write(broken.join("breaks"), "").unwrap();
    let first = fs::canonicalize(world.system("first")).unwrap();
    // The broken system's activation may have finished before its daemon
    // was cut off, so it runs.
    fs::remove_file(&world.config.current).unwrap();
    std::os::unix::fs::symlink(&broken, &world.config.current).unwrap();
    fs::write(
        world.config.state.join("switching.json"),
        serde_json::json!({
            "system": broken,
            "previous": first,
            "failed": "its health check failed",
        })
        .to_string(),
    )
    .unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Failed { why, .. }
            if why.contains("its health check failed") && why.contains("went back")
                && why.contains("making again")),
        "going back cut off wasn't finished and said: {ending:?}"
    );
    assert_eq!(world.running(), first, "it didn't go back");
    assert!(
        !world
            .read("state/activation.log")
            .contains("activation failed"),
        "the broken system was switched to again"
    );
    assert!(
        world
            .read("state/activation.log")
            .contains("setting up launchd services..."),
        "going back wasn't made again"
    );
    assert!(
        !world.config.state.join("switching.json").exists(),
        "going back left the switch marked unended"
    );
}

#[test]
fn a_going_back_made_again_that_fails_keeps_why_unchanged() {
    // Each update makes it again, so the marker's reason would otherwise
    // grow by what each said.
    let mut world = World::new();
    world.config.kind = Kind::Darwin;
    let one = world.signed("one");
    let broken = world.system(&one);
    fs::write(broken.join("breaks"), "").unwrap();
    let first = world.system("first");
    fs::write(first.join("breaks"), "").unwrap();
    fs::write(
        world.config.state.join("switching.json"),
        serde_json::json!({
            "system": broken,
            "previous": fs::canonicalize(&first).unwrap(),
            "failed": "its health check failed",
        })
        .to_string(),
    )
    .unwrap();
    for _ in 0..2 {
        let (_, ending) = world.update();
        assert!(
            matches!(&ending, Line::Failed { why, .. } if why.contains("going back failed too")),
            "{ending:?}"
        );
    }
    let left: serde_json::Value =
        serde_json::from_str(&world.read("state/switching.json")).unwrap();
    assert_eq!(
        left["failed"], "its health check failed",
        "the marker's reason changed as going back was made again"
    );
}

#[test]
fn a_broken_system_left_running_goes_back_to_the_profiles() {
    // Activated by hand behind its profile, what runs was never checked, so
    // a switch to it that fails goes back to the system the profile names.
    let world = World::new();
    let one = world.signed("one");
    let system = world.system(&one);
    world.update();
    let first = world.system("first");
    fs::remove_file(&world.config.profile).unwrap();
    std::os::unix::fs::symlink(&first, &world.config.profile).unwrap();
    fs::write(system.join("breaks"), "").unwrap();
    let two = world.signed("two");
    std::os::unix::fs::symlink(&system, world.root.join("systems").join(&two)).unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Failed { why, .. } if why.contains("went back")),
        "{ending:?}"
    );
    assert_eq!(world.running(), first, "it went back to the broken system");
}

#[test]
fn a_failed_going_back_keeps_its_switch_marked() {
    // The next update finishes going back, so what it goes back to, and
    // why, outlive this one.
    let world = World::new();
    let one = world.signed("one");
    let system = world.system(&one);
    world.update();
    fs::write(system.join("breaks"), "").unwrap();
    let two = world.signed("two");
    fs::write(world.system(&two).join("breaks"), "").unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Failed { why, .. } if why.contains("going back failed too")),
        "{ending:?}"
    );
    let left: serde_json::Value =
        serde_json::from_str(&world.read("state/switching.json")).unwrap();
    assert_eq!(
        left["previous"].as_str().map(PathBuf::from),
        fs::canonicalize(&system).ok(),
        "a failed going back lost where it went back to"
    );
    assert!(
        left["failed"].is_string(),
        "it lost why the switch failed: {left}"
    );
}

#[test]
fn a_marker_that_wont_read_still_switches_again() {
    let world = World::new();
    let one = world.signed("one");
    let system = world.system(&one);
    world.update();
    fs::write(world.config.state.join("switching.json"), "{").unwrap();
    let two = world.signed("two");
    std::os::unix::fs::symlink(&system, world.root.join("systems").join(&two)).unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Done { commit, .. } if *commit == two),
        "a marker that won't read was taken as none: {ending:?}"
    );
    assert!(
        !world.config.state.join("switching.json").exists(),
        "the marker that won't read was kept, to switch again every update"
    );
}

#[test]
fn a_specialisation_going_back_leaves_the_profile_at_its_system() {
    // A specialisation's own system holds none, so a failed switch goes
    // back to the system it belongs to, which the machine boots.
    let world = World::new();
    let one = world.signed("one");
    let system = world.system(&one);
    world.update();
    let docked = system.join("specialisation/docked");
    fs::create_dir_all(&docked).unwrap();
    fs::remove_file(world.root.join("running")).unwrap();
    std::os::unix::fs::symlink(&docked, world.root.join("running")).unwrap();
    let two = world.signed("two");
    fs::write(world.system(&two).join("breaks"), "").unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Failed { why, .. } if why.contains("went back")),
        "{ending:?}"
    );
    assert_eq!(
        fs::canonicalize(world.booting()).unwrap(),
        fs::canonicalize(&system).unwrap(),
        "going back pointed the profile at a specialisation"
    );
}

#[test]
fn a_machine_running_a_specialisation_of_main_switches_nothing() {
    // NixOS runs a specialisation in its system's stead when switched to
    // one, so the running system is that, not what main builds.
    let world = World::new();
    let one = world.signed("one");
    let system = world.system(&one);
    world.update();
    let docked = system.join("specialisation/docked");
    fs::create_dir_all(&docked).unwrap();
    fs::remove_file(world.root.join("running")).unwrap();
    std::os::unix::fs::symlink(&docked, world.root.join("running")).unwrap();
    let two = world.signed("two");
    std::os::unix::fs::symlink(&system, world.root.join("systems").join(&two)).unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(&ending, Line::Unchanged { commit, .. } if *commit == two),
        "a machine in a specialisation was switched: {ending:?}"
    );
    assert_eq!(world.running(), docked, "it left the specialisation");
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
fn checking_an_openpgp_signature_leaves_no_agent_running() {
    let world = World::new();
    let (world, head) = openpgp_signed(world, "Merger");
    assert_eq!(world.update().1, done(&head));
    // The keyring is made afresh each time, so an agent left on it would
    // be one more left behind at every ask.
    let socket = Command::new(program("gpgconf"))
        .arg("--homedir")
        .arg(world.config.keyring())
        .args(["--list-dirs", "agent-socket"])
        .output()
        .unwrap();
    let socket = String::from_utf8_lossy(&socket.stdout).trim().to_owned();
    assert!(!socket.is_empty(), "gpgconf names no agent socket");
    assert!(
        !Path::new(&socket).exists(),
        "an agent still runs on {socket}"
    );
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
fn a_switch_whose_unit_failed_says_what_that_unit_said_last() {
    let world = World::new();
    let first = world.running();
    let head = world.signed("one");
    fs::write(world.system(&head).join("unit-fails"), "").unwrap();
    let (_, ending) = world.update();
    let Line::Failed { why, lines, .. } = ending else {
        panic!("not failed: {ending:?}")
    };
    assert!(why.contains("went back to what it ran"), "{why}");
    // On 16 November two switches said only that the machine went back,
    // and naming the hung unit took an hour.
    assert_eq!(
        lines,
        [
            "warning: the following units failed: home-manager-joel.service",
            "home-manager-joel.service: Activating reloadSystemd",
            "home-manager-joel.service: start operation timed out",
        ],
        "the failed unit's own last lines should follow the switch's"
    );
    assert_eq!(world.running(), first);
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

/// A machine booted from `first`'s kernel, and main's system for `head`
/// with another kernel, whose switch refuses any mode but a live one.
fn a_new_kernel(world: &World, head: &str) -> PathBuf {
    for system in [world.running(), world.root.join("booted")] {
        fs::create_dir_all(&system).unwrap();
        fs::write(system.join("kernel"), "one").unwrap();
    }
    let system = world.system(head);
    fs::write(system.join("kernel"), "two").unwrap();
    let switch = system.join("bin/switch-to-configuration");
    let head = stand_in_head();
    let rest = fs::read_to_string(&switch).unwrap();
    let rest = rest.strip_prefix(&head).unwrap();
    write_program(
        &switch,
        &format!("{head}\n[ \"$1\" = switch ] || exit 9{rest}"),
    );
    system
}

#[test]
fn a_new_kernels_userland_runs_now_and_its_kernel_from_the_next_boot() {
    let world = World::new();
    let head = world.signed("one");
    let system = a_new_kernel(&world, &head);
    let (_, ending) = world.update();
    assert_eq!(
        ending,
        Line::Done {
            commit: head,
            applied: Applied::Now,
            restart: Some("its kernel changed".to_owned()),
            restarted: vec!["sshd.service".to_owned()],
            boots: None,
            followed: BTreeMap::new(),
        }
    );
    assert_eq!(world.running(), system);
    assert_eq!(world.booting(), system);
}

/// A machine booted from `first`'s initrd, and main's system for `head`
/// with another, whose boot check runs `check` with the system's path as
/// its `$1`.
fn a_new_initrd(world: &mut World, head: &str, check: &str) -> PathBuf {
    // The booted system is `first` itself, so going back can make it the
    // boot default again.
    let first = world.running();
    fs::write(first.join("initrd"), "one").unwrap();
    let booted = world.root.join("booted");
    if fs::symlink_metadata(&booted).is_err() {
        std::os::unix::fs::symlink(&first, &booted).unwrap();
    }
    let system = world.system(head);
    fs::write(system.join("initrd"), "two").unwrap();
    world.config.boot_check = Some(vec![
        shell(),
        "-c".to_owned(),
        check.to_owned(),
        "boot-check".to_owned(),
    ]);
    system
}

#[test]
fn a_new_initrd_runs_now_and_is_booted_next_once_its_boot_check_passes() {
    // On 15 November switch968 made a new initrd what the machine boots,
    // and nothing had booted it: a broken one leaves a laptop that can't
    // unlock its disk at the next boot.
    let mut world = World::new();
    let head = world.signed("one");
    let checked = world.root.join("checked");
    let check = format!("echo \"$1\" > {}", checked.display());
    let system = a_new_initrd(&mut world, &head, &check);
    let (_, ending) = world.update();
    assert_eq!(
        ending,
        Line::Done {
            commit: head,
            applied: Applied::Now,
            restart: Some("its initrd changed".to_owned()),
            restarted: vec!["sshd.service".to_owned()],
            boots: None,
            followed: BTreeMap::new(),
        },
        "a passed boot check should end as a switch does"
    );
    assert_eq!(
        fs::read_to_string(&checked).unwrap_or_default().trim(),
        system.display().to_string(),
        "the boot check wasn't given the new system"
    );
    assert_eq!(
        world.modes(),
        ["test", "boot"],
        "the new system should run first, and be booted only after its check"
    );
    assert_eq!(world.running(), system, "the new system isn't running");
    assert_eq!(
        world.boots(),
        system,
        "the checked system isn't booted next"
    );
}

#[test]
fn the_boot_check_is_told_the_commit_flake_and_overrides_its_system_was_built_from() {
    // A check that builds its own test of the system needs the tree the
    // system came from, which the system's path alone doesn't name.
    let mut world = following();
    let pinned = world.core_commit("pinned", true);
    let newer = world.core_commit("newer", true);
    let head = world.signed_pinning("one", &pinned);
    let told = world.root.join("told");
    let check = format!(
        "printf '%s\\n' \"$UPDATE_TO_MAIN_COMMIT\" \"$UPDATE_TO_MAIN_FLAKE\" \"$PATH\" > {told}; \
         printf '%s' \"$UPDATE_TO_MAIN_OVERRIDES\" > {told}.overrides",
        told = told.display()
    );
    a_new_initrd(&mut world, &head, &check);
    let (_, ending) = world.update();
    assert!(
        matches!(ending, Line::Done { boots: None, .. }),
        "the boot check should pass: {ending:?}"
    );
    let said = fs::read_to_string(&told).unwrap_or_default();
    let said: Vec<&str> = said.lines().collect();
    assert_eq!(
        said.first().copied(),
        Some(head.as_str()),
        "the boot check wasn't told the commit it checks: {said:?}"
    );
    let flake = said.get(1).copied().unwrap_or_default();
    assert!(
        flake.starts_with("git+file://")
            && flake.ends_with(&format!("rev={head}"))
            && !flake.contains('#'),
        "the boot check wasn't told a flake locked at the commit, with no output: {flake}"
    );
    // A nix it runs fetches a git+file flake with git, which the system's
    // own path may hold none of, as update-to-main's own nix does.
    let git = world
        .config
        .tools
        .git
        .parent()
        .unwrap()
        .display()
        .to_string();
    let path = said.get(2).copied().unwrap_or_default();
    assert_eq!(
        path.split(':').next(),
        Some(git.as_str()),
        "the boot check's PATH doesn't start with root's git: {path}"
    );
    let overrides = fs::read_to_string(told.with_extension("overrides")).unwrap_or_default();
    let overrides: Vec<&str> = overrides.lines().collect();
    assert!(
        overrides.len() == 3
            && overrides[..2] == ["--override-input", "core"]
            && overrides[2].ends_with(&format!("rev={newer}")),
        "the boot check wasn't told the inputs its system was built with: {overrides:?}"
    );
}

#[test]
fn a_new_initrd_whose_boot_check_fails_runs_but_isnt_booted_next() {
    let mut world = World::new();
    let first = world.running();
    let head = world.signed("one");
    let check = "echo 'no passphrase prompt in 300 s' >&2; exit 1";
    let system = a_new_initrd(&mut world, &head, check);
    let (_, ending) = world.update();
    let Line::Done { restart, boots, .. } = ending else {
        panic!("a failed boot check went back, though the new system runs: {ending:?}")
    };
    let boots = boots.expect("a failed boot check wasn't said");
    assert!(
        boots.contains("the boot check") && boots.contains("no passphrase prompt in 300 s"),
        "the failure doesn't say which check or what it said: {boots}"
    );
    assert_eq!(
        restart, None,
        "no restart brings in a system that isn't booted next"
    );
    assert_eq!(
        world.modes(),
        ["test"],
        "an unchecked system was made the boot default"
    );
    assert_eq!(world.running(), system, "the new system should still run");
    assert_eq!(
        world.boots(),
        first,
        "the boot default should stay where it was"
    );
}

/// The machine whose boot check failed, its why, and the system it runs.
fn a_failed_boot_check(world: &mut World) -> (String, PathBuf) {
    let head = world.signed("one");
    let system = a_new_initrd(world, &head, "echo 'no passphrase prompt' >&2; exit 1");
    let (_, ending) = world.update();
    let Line::Done {
        boots: Some(boots), ..
    } = ending
    else {
        panic!("the failed boot check didn't say why: {ending:?}")
    };
    (boots, system)
}

#[test]
fn an_unchanged_ask_after_a_failed_boot_check_says_why_the_old_system_boots() {
    // Before, the ask after the failed check said nothing of it, so the
    // system page read the machine as booting what it ran.
    let mut world = World::new();
    let (boots, _) = a_failed_boot_check(&mut world);
    for ask in ["the next", "the one after"] {
        let (_, again) = world.update();
        let Line::Unchanged { boots: still, .. } = again else {
            panic!("{ask} ask at the main it runs wasn't unchanged: {again:?}")
        };
        assert_eq!(
            still.as_deref(),
            Some(boots.as_str()),
            "{ask} unchanged ask forgot why the machine boots its old system"
        );
    }
}

#[test]
fn a_boot_check_is_said_unended_while_it_runs() {
    // A daemon cut off in its boot check never ends it, and the boot
    // default hasn't moved, so the asks after it must say so. The check
    // copies what was kept while it ran, which the test puts back after,
    // as a daemon cut off there would have left it.
    let mut world = World::new();
    let head = world.signed("one");
    let seen = world.root.join("seen");
    let kept = world.config.boots_file();
    // The build sandbox has no cat, so the shell reads the file itself.
    let check = format!("printf %s \"$(< {})\" > {}", kept.display(), seen.display());
    a_new_initrd(&mut world, &head, &check);
    let (_, ending) = world.update();
    assert!(
        matches!(ending, Line::Done { boots: None, .. }),
        "{ending:?}"
    );
    assert!(!kept.exists(), "the passing check left the why kept");
    fs::copy(&seen, &kept).unwrap();
    let (_, again) = world.update();
    let Line::Unchanged {
        boots: Some(said), ..
    } = again
    else {
        panic!("the ask after a daemon cut off in its check said nothing: {again:?}")
    };
    assert_eq!(
        said, "the boot check didn't end, so the machine boots what it booted before",
        "{said}"
    );
}

#[test]
fn going_back_to_a_system_whose_boot_check_failed_keeps_the_old_boot_default() {
    // Going back used to switch, which made the system whose boot check
    // failed the boot default, and then ended the only word of it.
    let mut world = World::new();
    let first = world.boots();
    let (boots, _) = a_failed_boot_check(&mut world);
    let one = world.head();
    let two = world.signed("two");
    let broken = world.system(&two);
    fs::write(broken.join("initrd"), "two").unwrap();
    fs::write(broken.join("breaks"), "").unwrap();
    let (_, failed) = world.update();
    let Line::Failed { why, .. } = &failed else {
        panic!("the broken switch didn't fail: {failed:?}")
    };
    assert!(why.contains("went back to what it ran"), "{why}");
    assert_eq!(
        fs::canonicalize(world.boots()).unwrap(),
        fs::canonicalize(&first).unwrap(),
        "going back made a system whose boot check failed the boot default"
    );
    git(&world.origin, &["reset", "--quiet", "--hard", &one]);
    let (_, again) = world.update();
    let Line::Unchanged { boots: still, .. } = again else {
        panic!("the ask at the main gone back to wasn't unchanged: {again:?}")
    };
    assert_eq!(
        still.as_deref(),
        Some(boots.as_str()),
        "going back forgot why the machine boots its old system"
    );
}

/// Makes the machine boot `system` and run it, as a reboot into it would.
fn reboot_into(world: &World, system: &Path) {
    // A file's time comes from a clock that moves a few milliseconds at a
    // time, so a boot made in the same moment as the why kept before it
    // would read as no newer; a real boot comes long after, as here.
    if let Ok(kept) = fs::File::options()
        .write(true)
        .open(world.config.boots_file())
    {
        let before = std::time::SystemTime::now() - Duration::from_secs(1);
        kept.set_modified(before).unwrap();
    }
    let booted = &world.config.booted;
    if booted.is_dir() && !booted.is_symlink() {
        fs::remove_dir_all(booted).unwrap();
    } else {
        fs::remove_file(booted).unwrap();
    }
    std::os::unix::fs::symlink(system, booted).unwrap();
    let running = world.root.join("running");
    fs::remove_file(&running).unwrap();
    std::os::unix::fs::symlink(system, &running).unwrap();
}

#[test]
fn a_reboot_into_the_old_system_says_it_left_the_machine_there() {
    // A reboot after a failed boot check starts the old boot default, and
    // the asks after it read healthy at main though main's system ran no
    // more.
    let mut world = World::new();
    let first = world.boots();
    let (boots, _) = a_failed_boot_check(&mut world);
    reboot_into(&world, &first);
    let (_, again) = world.update();
    let Line::Unchanged {
        boots: Some(said), ..
    } = again
    else {
        panic!("the ask after the reboot said nothing of it: {again:?}")
    };
    assert!(
        said.starts_with("a boot left it on another system than main's, since ")
            && said.ends_with(&boots),
        "{said}"
    );
}

#[test]
fn going_back_to_a_checked_system_makes_it_the_boot_default_again() {
    // A machine booted many switches ago reads every newer initrd as one
    // waiting on its check, though this one passed it: going back to it
    // switches, so the failed switch isn't left what the machine boots.
    let mut world = World::new();
    let head = world.signed("one");
    let checked = a_new_initrd(&mut world, &head, "true");
    let (_, passed) = world.update();
    assert!(
        matches!(passed, Line::Done { boots: None, .. }),
        "{passed:?}"
    );
    let two = world.signed("two");
    let system = world.system(&two);
    std::os::unix::fs::symlink(world.config.booted.join("initrd"), system.join("initrd")).unwrap();
    fs::write(system.join("unhealthy"), "").unwrap();
    let (_, failed) = world.update();
    assert!(
        matches!(&failed, Line::Failed { why, .. } if why.contains("went back to what it ran")),
        "{failed:?}"
    );
    assert_eq!(
        fs::canonicalize(world.boots()).unwrap(),
        fs::canonicalize(&checked).unwrap(),
        "going back left a switch that failed its health check the boot default"
    );
}

/// A system of main's `commit` that boots as the booted system does, so
/// its switch makes it the boot default at once, and whose health fails.
fn unhealthy_booting_as_booted(world: &World, commit: &str) -> PathBuf {
    let system = world.system(commit);
    std::os::unix::fs::symlink(world.config.booted.join("initrd"), system.join("initrd")).unwrap();
    fs::write(system.join("unhealthy"), "").unwrap();
    system
}

/// A machine whose update-to-main made main's `zero` its boot default,
/// then ran main's `one`, whose boot check failed, so it boots `zero`
/// still; and main's `two`, whose switch makes it the boot default and
/// whose health then fails. Gives `zero` and `two`.
fn a_switch_failing_after_a_failed_boot_check(world: &mut World) -> (PathBuf, PathBuf) {
    let booted = world.system("booted");
    fs::write(booted.join("initrd"), "one").unwrap();
    std::os::unix::fs::symlink(&booted, &world.config.booted).unwrap();
    let zero = world.signed("zero");
    std::os::unix::fs::symlink(booted.join("initrd"), world.system(&zero).join("initrd")).unwrap();
    let (_, switched) = world.update();
    assert!(
        matches!(switched, Line::Done { boots: None, .. }),
        "main's zero didn't switch: {switched:?}"
    );
    a_failed_boot_check(world);
    let two = world.signed("two");
    let failing = unhealthy_booting_as_booted(world, &two);
    (world.system(&zero), failing)
}

/// The why of `ending`, an update that failed and went back.
fn went_back(ending: &Line) -> &str {
    let Line::Failed { why, .. } = ending else {
        panic!("the unhealthy switch didn't fail: {ending:?}")
    };
    assert!(why.contains("went back to what it ran"), "{why}");
    why
}

#[test]
fn going_back_unchecked_puts_back_what_the_machine_booted_before() {
    // Going back to a system whose boot check failed only activates it,
    // so a switch that failed after making itself the boot default stayed
    // it: the machine would next boot what had just failed.
    let mut world = World::new();
    let (zero, _) = a_switch_failing_after_a_failed_boot_check(&mut world);
    let (_, failed) = world.update();
    let why = went_back(&failed);
    assert_eq!(
        fs::canonicalize(world.boots()).unwrap(),
        fs::canonicalize(&zero).unwrap(),
        "going back left the switch that failed what the machine boots: {why}"
    );
    assert_eq!(
        world.modes().last().map(String::as_str),
        Some("boot"),
        "what the machine booted before wasn't made the boot default again"
    );
    assert_eq!(
        fs::canonicalize(&world.config.profile).unwrap(),
        fs::canonicalize(&zero).unwrap(),
        "the profile, which a loader boots the newest of, didn't name what it booted before"
    );
}

#[test]
fn a_healthy_switch_is_what_going_back_unchecked_puts_back() {
    let mut world = World::new();
    a_switch_failing_after_a_failed_boot_check(&mut world);
    world.update();
    let three = world.signed("three");
    let healthy = world.system(&three);
    std::os::unix::fs::symlink(world.config.booted.join("initrd"), healthy.join("initrd")).unwrap();
    let (_, switched) = world.update();
    assert!(
        matches!(switched, Line::Done { boots: None, .. }),
        "main's three didn't switch: {switched:?}"
    );
    let four = world.signed("four");
    a_new_initrd(&mut world, &four, "exit 1");
    let (_, unchecked) = world.update();
    assert!(
        matches!(unchecked, Line::Done { boots: Some(_), .. }),
        "main's four didn't fail its boot check: {unchecked:?}"
    );
    let five = world.signed("five");
    unhealthy_booting_as_booted(&world, &five);
    let (_, failed) = world.update();
    let why = went_back(&failed);
    assert_eq!(
        fs::canonicalize(world.boots()).unwrap(),
        fs::canonicalize(&healthy).unwrap(),
        "going back didn't put back the boot default the healthy switch made: {why}"
    );
}

#[test]
fn a_boot_default_set_by_hand_since_is_left_be_and_said() {
    // A switch by hand moves the boot default to the person's choice, so
    // the one update-to-main kept before it is no longer what to go back to.
    let mut world = World::new();
    let (_, two) = a_switch_failing_after_a_failed_boot_check(&mut world);
    let by_hand = world.system("by-hand");
    for link in [world.config.profile.clone(), world.root.join("boots")] {
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&by_hand, &link).unwrap();
    }
    let (_, failed) = world.update();
    let why = went_back(&failed);
    assert!(
        why.contains("the profile moved outside update-to-main"),
        "going back didn't say why it left the boot default be: {why}"
    );
    assert_eq!(
        fs::canonicalize(world.boots()).unwrap(),
        fs::canonicalize(&two).unwrap(),
        "going back moved a boot default the profile no longer named: {why}"
    );
}

#[test]
fn a_switch_cut_off_after_it_moved_the_profile_still_puts_back_the_boot_default() {
    // update-to-main moves the profile itself before a switch ends, so a
    // daemon cut off between the two left the profile its own, not a hand's.
    let mut world = World::new();
    let (zero, two) = a_switch_failing_after_a_failed_boot_check(&mut world);
    let one = world.running();
    let moved = [
        world.config.profile.clone(),
        world.root.join("boots"),
        world.root.join("running"),
    ];
    for link in moved {
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&two, &link).unwrap();
    }
    fs::write(
        world.config.state.join("switching.json"),
        serde_json::json!({ "system": two, "previous": one }).to_string(),
    )
    .unwrap();
    let (_, failed) = world.update();
    let why = went_back(&failed);
    assert_eq!(
        fs::canonicalize(world.boots()).unwrap(),
        fs::canonicalize(&zero).unwrap(),
        "a switch cut off after moving the profile left the one that failed the boot default: {why}"
    );
}

#[test]
fn a_boot_default_collected_since_is_said_left_to_the_switch_that_failed() {
    let mut world = World::new();
    let (zero, two) = a_switch_failing_after_a_failed_boot_check(&mut world);
    fs::remove_dir_all(&zero).unwrap();
    let (_, failed) = world.update();
    let why = went_back(&failed);
    let next = format!("boots {} next", two.display());
    assert!(
        why.contains(&next) && why.contains("is gone from the store"),
        "going back didn't say the machine boots the switch that failed next: {why}"
    );
    assert_eq!(
        fs::canonicalize(world.boots()).unwrap(),
        fs::canonicalize(&two).unwrap(),
        "{why}"
    );
}

#[test]
fn a_rollback_by_hand_to_what_it_booted_is_no_boot() {
    let mut world = World::new();
    let first = world.boots();
    a_failed_boot_check(&mut world);
    let running = world.root.join("running");
    fs::remove_file(&running).unwrap();
    std::os::unix::fs::symlink(&first, &running).unwrap();
    let (_, again) = world.update();
    assert!(
        matches!(again, Line::Unchanged { boots: None, .. }),
        "a rollback by hand was said to be a boot: {again:?}"
    );
}

#[test]
fn a_boot_into_a_specialisation_of_main_s_system_ends_the_why() {
    let mut world = World::new();
    let (_, system) = a_failed_boot_check(&mut world);
    let docked = system.join("specialisation/docked");
    fs::create_dir_all(&docked).unwrap();
    reboot_into(&world, &docked);
    let (_, again) = world.update();
    assert!(
        matches!(again, Line::Unchanged { boots: None, .. }),
        "an ask after main's specialisation booted said the old system boots: {again:?}"
    );
}

#[test]
fn a_reboot_into_main_s_system_ends_the_why_the_old_system_boots() {
    // However its boot default moved, main's system booted is the why over.
    let mut world = World::new();
    let (_, system) = a_failed_boot_check(&mut world);
    reboot_into(&world, &system);
    let (_, again) = world.update();
    assert!(
        matches!(again, Line::Unchanged { boots: None, .. }),
        "an ask after main's system booted said the old one boots: {again:?}"
    );
}

#[test]
fn a_switch_needing_no_boot_check_ends_the_why_the_old_system_boots() {
    // Its switch makes it the boot default, so a reboot into it later is
    // no boot that left the machine on an older system.
    let mut world = World::new();
    a_failed_boot_check(&mut world);
    let two = world.signed("two");
    let system = world.system(&two);
    // The booted system's own initrd, which is what needs no boot check.
    std::os::unix::fs::symlink(world.config.booted.join("initrd"), system.join("initrd")).unwrap();
    let (_, switched) = world.update();
    assert!(
        matches!(switched, Line::Done { boots: None, .. }),
        "a switch needing no boot check said one failed: {switched:?}"
    );
    reboot_into(&world, &system);
    let (_, again) = world.update();
    assert!(
        matches!(again, Line::Unchanged { boots: None, .. }),
        "a reboot into what a plain switch made the boot default was said to \
         leave the old system: {again:?}"
    );
}

#[test]
fn a_system_switched_to_by_hand_isnt_said_to_boot_the_old_one() {
    // The why speaks of the system the boot check failed for, not of one
    // a person switched to, or rolled back to, since.
    let mut world = World::new();
    a_failed_boot_check(&mut world);
    let by_hand = world.root.join("by-hand");
    fs::create_dir_all(&by_hand).unwrap();
    fs::remove_file(&world.config.current).unwrap();
    std::os::unix::fs::symlink(&by_hand, &world.config.current).unwrap();
    let (_, again) = world.update();
    assert!(
        matches!(again, Line::Unchanged { boots: None, .. }),
        "an unchanged ask said why a system no longer run boots old: {again:?}"
    );
}

#[test]
fn a_boot_check_that_passes_later_ends_the_why_the_old_system_boots() {
    let mut world = World::new();
    let head = world.signed("one");
    a_new_initrd(&mut world, &head, "echo 'no passphrase prompt' >&2; exit 1");
    let (_, failed) = world.update();
    assert!(
        matches!(failed, Line::Done { boots: Some(_), .. }),
        "{failed:?}"
    );
    let newer = world.signed("two");
    let system = world.system(&newer);
    fs::write(system.join("initrd"), "three").unwrap();
    world.config.boot_check = Some(vec![shell(), "-c".to_owned(), "true".to_owned()]);
    let (_, passed) = world.update();
    assert!(
        matches!(passed, Line::Done { boots: None, .. }),
        "the passing boot check still said the old system boots: {passed:?}"
    );
    assert_eq!(world.boots(), system, "the checked system boots next");
    let (_, again) = world.update();
    assert!(
        matches!(again, Line::Unchanged { boots: None, .. }),
        "an unchanged ask kept a why the passing check ended: {again:?}"
    );
}

#[test]
fn a_new_kernels_userland_not_whole_goes_back() {
    let world = World::new();
    let first = world.running();
    let head = world.signed("one");
    let system = a_new_kernel(&world, &head);
    fs::write(system.join("unhealthy"), "").unwrap();
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
    assert_eq!(told, [] as [update_to_main::line::Line; 0]);
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
    // Both askers join the update's run within this.
    thread::sleep(std::time::Duration::from_millis(300));
    // While it runs, what it's doing is kept for another program to see.
    let running: Line =
        serde_json::from_str(&fs::read_to_string(&world.config.running).unwrap()).unwrap();
    assert!(
        matches!(running, Line::Fetching | Line::Building { .. }),
        "{running:?}"
    );
    fs::write(&gate, "").unwrap();
    for asker in askers {
        let (told, ending) = asker.join().unwrap();
        assert_eq!(told.first(), Some(&Line::Fetching));
        assert_eq!(ending, done(&main));
    }
    assert_eq!(world.builds(), 1);
}

#[test]
fn a_stop_ends_an_update_still_building_and_the_next_builds_again() {
    let world = World::new();
    let main = world.signed("one");
    world.system(&main);
    let gate = builds_wait_for_a_gate(&world);
    let socket = serving(&world);
    let nothing = ask::stop(&socket).unwrap();
    assert!(
        matches!(&nothing, Line::Refused { why, .. } if why.contains("nothing was stopped")),
        "{nothing:?}"
    );
    let asking = building(&world, &socket);
    let stopped = ask::stop(&socket).unwrap();
    assert!(
        matches!(&stopped, Line::Refused { why, .. }
            if why.starts_with("stopped the update building")),
        "{stopped:?}"
    );
    let ending = asking.join().unwrap();
    assert!(matches!(ending, Line::Failed { .. }), "{ending:?}");
    assert_eq!(world.builds(), 0);

    // What stopped that one doesn't stop the next.
    fs::write(&gate, "").unwrap();
    assert_eq!(ask::ask(&socket, &mut |_| {}).unwrap(), done(&main));
}

#[test]
fn a_persons_stop_ends_an_update_a_hold_was_given_for_and_one_for_a_newer_head_doesnt() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    let main = world.signed("one");
    daemon_unit(&world.system(&main), "same");
    fs::write(world.root.join("given"), "").unwrap();
    builds_wait_for_a_gate(&world);
    let socket = serving(&world);
    let asking = building(&world, &socket);
    let newer = ask::stop_for_newer(&socket).unwrap();
    assert!(
        matches!(&newer, Line::Refused { why, .. } if why.ends_with("so it runs on")),
        "a stop for a newer head ended an update a hold was given for: {newer:?}"
    );
    let stopped = ask::stop(&socket).unwrap();
    assert!(
        matches!(&stopped, Line::Refused { why, .. }
            if why.starts_with("stopped the update building")
                && why.contains("the hold given for it is spent")),
        "a person's stop didn't end an update a hold was given for: {stopped:?}"
    );
    let ending = asking.join().unwrap();
    assert!(
        matches!(ending, Line::Failed { .. }),
        "the stopped update didn't fail: {ending:?}"
    );
    assert_eq!(world.builds(), 0, "the stopped update built a system");
}

/// Makes every Nix command wait for the world's `gate` file, so a stop
/// comes while the build runs, and gives the gate.
fn builds_wait_for_a_gate(world: &World) -> PathBuf {
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
    gate
}

/// Serves the world's update-to-main, giving its socket once it's there.
fn serving(world: &World) -> PathBuf {
    let config = world.config.clone();
    thread::spawn(move || serve::serve(config));
    let socket = world.config.socket.clone();
    while !socket.exists() {
        thread::sleep(std::time::Duration::from_millis(20));
    }
    socket
}

/// Asks for an update, giving its ending's thread once it builds.
fn building(world: &World, socket: &Path) -> thread::JoinHandle<Line> {
    let asking = {
        let socket = socket.to_path_buf();
        thread::spawn(move || ask::ask(&socket, &mut |_| {}).unwrap())
    };
    while !fs::read_to_string(&world.config.running).is_ok_and(|line| line.contains("building")) {
        thread::sleep(std::time::Duration::from_millis(20));
    }
    asking
}

#[test]
fn how_an_update_ended_is_kept_for_its_group_to_read() {
    use std::os::unix::fs::MetadataExt;
    let world = World::new();
    let main = world.signed("one");
    world.system(&main);
    let config = world.config.clone();
    thread::spawn(move || serve::serve(config));
    while !world.config.socket.exists() {
        thread::sleep(std::time::Duration::from_millis(20));
    }
    let ending = ask::ask(&world.config.socket, &mut |_| {}).unwrap();
    assert_eq!(ending, done(&main));
    let kept: Line =
        serde_json::from_str(&fs::read_to_string(&world.config.ended).unwrap()).unwrap();
    assert_eq!(kept, ending, "the file says how it ended");
    assert!(
        !world.config.running.exists(),
        "once it has ended, none is running"
    );
    let meta = fs::metadata(&world.config.ended).unwrap();
    assert_eq!(meta.mode() & 0o777, 0o640);
    // SAFETY: getgid can't fail.
    assert_eq!(meta.gid(), unsafe { libc::getgid() });

    // A second ask, with main where it was, says so in the file in its place.
    let again = ask::ask(&world.config.socket, &mut |_| {}).unwrap();
    assert!(matches!(again, Line::Unchanged { .. }), "{again:?}");
    let kept: Line =
        serde_json::from_str(&fs::read_to_string(&world.config.ended).unwrap()).unwrap();
    assert_eq!(kept, again);
    assert!(
        !world.root.join("last.json.new").exists(),
        "nothing is left beside it"
    );
}

#[test]
fn a_pause_asked_on_the_socket_refuses_the_followers_updates_until_resumed() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    let config = world.config.clone();
    thread::spawn(move || serve::serve(config));
    while !world.config.socket.exists() {
        // The daemon makes its socket on its own thread; there's nothing
        // to wait on but the file.
        thread::sleep(Duration::from_millis(20));
    }
    let paused = ask::once(&world.config.socket, held::PAUSE).unwrap();
    let Line::Refused { why, .. } = &paused else {
        panic!("a pause wasn't answered with what holds: {paused:?}")
    };
    assert!(held::says(why), "a pause wasn't taken: {why}");
    let (_, ending) = world.asked_by(&|| true);
    let Line::Refused { why, .. } = &ending else {
        panic!("a paused machine didn't refuse the follower: {ending:?}")
    };
    assert!(
        why.contains("updates are paused"),
        "refused for another reason: {why}"
    );
    assert_eq!(world.builds(), 0, "a paused machine built for the follower");
    assert_eq!(
        world.update().1,
        done(&one),
        "a paused machine refused a person's ask"
    );
    ask::once(&world.config.socket, held::RESUME).unwrap();
    let two = world.signed("two");
    world.system(&two);
    assert_eq!(
        world.asked_by(&|| true).1,
        done(&two),
        "a resumed machine didn't update for the follower"
    );
}

#[test]
fn a_pause_taken_while_the_followers_switch_waits_refuses_it() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    world.update();
    let two = world.signed("two");
    world.system(&two);
    // Who asked is looked at first while the switch waits, since nothing
    // is paused before it builds.
    let (_, ending) = world.asked_by(&|| {
        held::ask(&world.config, held::PAUSE, Some(1000));
        true
    });
    let Line::Refused { why, .. } = &ending else {
        panic!("a pause taken while it waited didn't refuse the switch: {ending:?}")
    };
    assert!(
        why.contains("updates are paused"),
        "refused for another reason: {why}"
    );
}

#[test]
fn a_main_holding_a_held_commit_is_refused_until_it_is_lifted() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    world.update();
    let bad = world.signed("two");
    world.system(&bad);
    let three = world.signed("three");
    world.system(&three);
    let said = held::ask(&world.config, &format!("hold {bad}"), Some(1000));
    assert!(
        said.is_some_and(|said| said.is_ok()),
        "a hold on {bad} wasn't taken"
    );
    let (_, ending) = world.update();
    let Line::Refused { commit, why } = &ending else {
        panic!("a main holding a held commit wasn't refused: {ending:?}")
    };
    assert_eq!(
        commit.as_deref(),
        Some(one.as_str()),
        "the refusal named another commit"
    );
    assert!(why.contains("is held"), "refused for another reason: {why}");
    assert_eq!(world.builds(), 1, "a main holding a held commit was built");
    held::ask(&world.config, &format!("lift {bad}"), Some(1000));
    assert_eq!(
        world.update().1,
        done(&three),
        "a lifted hold still refused"
    );
}

#[test]
fn a_hold_on_a_commit_main_doesnt_hold_lets_it_switch() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    held::ask(
        &world.config,
        &format!("hold {}", "0".repeat(40)),
        Some(1000),
    );
    assert_eq!(
        world.update().1,
        done(&one),
        "a hold main doesn't hold refused it"
    );
}

/// A hold taken while the switch waits, as it may for an hour on someone
/// at the machine, is seen before it switches, not only before it builds.
#[test]
fn a_hold_taken_while_the_switch_waits_refuses_it() {
    let world = World::new();
    let one = world.signed("one");
    world.system(&one);
    world.update();
    let two = world.signed("two");
    world.system(&two);
    let (_, ending) = world.asked_by(&|| {
        held::ask(&world.config, &format!("hold {two}"), Some(1000));
        false
    });
    let Line::Refused { why, .. } = &ending else {
        panic!("a hold taken while it waited didn't refuse the switch: {ending:?}")
    };
    assert!(why.contains("is held"), "refused for another reason: {why}");
}

/// A hold on a followed input's commit is judged in that input's copy
/// alone. Main's host doesn't know core's commits, and asking it refused
/// every head while one of core's was held.
#[test]
fn a_hold_on_a_followed_inputs_commit_is_judged_in_its_repository_alone() {
    let world = api_on(World::hosted_from(following(), counted_token));
    let pinned = world.core_commit("pinned", true);
    let one = world.hosted_commit("one", Some(&pinning(&pinned)));
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, following_done(&one, &pinned));
    // Root's copy of core holds a commit core's main doesn't, as it would
    // one main left behind.
    let core = world.root.join("core");
    git(&core, &["checkout", "--quiet", "-b", "aside"]);
    let aside = world.core_commit("aside", true);
    git(&core, &["checkout", "--quiet", "main"]);
    let from = format!("file://{}", core.display());
    let copy = world.config.follows_folder().join("core.git");
    git(&copy, &["fetch", "--quiet", &from, "aside"]);
    held::ask(&world.config, &format!("hold {aside}"), Some(1000));
    let two = world.hosted_commit("two", Some(&pinning(&pinned)));
    world.answer();
    world.system(&two);
    assert_eq!(
        world.update().1,
        following_done(&two, &pinned),
        "a hold on a commit core's main doesn't hold refused main"
    );
    let newer = world.core_commit("newer", true);
    held::ask(&world.config, &format!("hold {newer}"), Some(1000));
    let three = world.hosted_commit("three", Some(&pinning(&pinned)));
    world.answer();
    world.system(&three);
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a core holding a held commit wasn't refused")
    };
    assert!(why.contains("is held"), "refused for another reason: {why}");
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

/// A 404 from main's host says it doesn't know a held commit only when the
/// same token shows it main's head: a token that can't see the repository
/// is answered 404 for every commit, so a token command giving one to the
/// hold's ask alone would lift a hold of main's own commit.
#[test]
fn a_token_that_cant_see_main_lifts_no_hold_of_main_s_commit() {
    let world = api_on(World::hosted_from(following(), blinded_token));
    let pinned = world.core_commit("pinned", true);
    let zero = world.hosted_commit("zero", Some(&pinning(&pinned)));
    let one = world.hosted_commit("one", Some(&pinning(&pinned)));
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, following_done(&one, &pinned));
    // Core's copy holds main's history, as a push of it into core's
    // repository would leave it, so the hold is one an input's copy has.
    let origin = format!("file://{}", world.origin.display());
    let copy = world.config.follows_folder().join("core.git");
    git(&copy, &["fetch", "--quiet", &origin, "main"]);
    held::ask(&world.config, &format!("hold {zero}"), Some(1000));
    let two = world.hosted_commit("two", Some(&pinning(&pinned)));
    world.answer();
    world.system(&two);
    let kept = world.config.reads().join(&two);
    let blind = format!("{zero} {two} {}", kept.display());
    fs::write(world.root.join("blind"), blind).unwrap();
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a token that couldn't see main lifted the hold of main's own commit")
    };
    assert!(
        why.contains("couldn't be told"),
        "a hold judged with a token that couldn't see main was refused for another reason: {why}"
    );
}

/// A token command that gives a token, and once the world's `blind` file
/// names a held commit, a head and where the read of that head's folder is
/// kept, gives every ask after the read a token that can't see main: the
/// host's answers for that head, and for how it stands with the held
/// commit, are gone.
fn blinded_token(world_root: &Path) -> String {
    let root = world_root.display();
    format!(
        "if [ -e {root}/blind ]; then read -r held head kept < {root}/blind; \
         if [ -e \"$kept\" ]; then \
         rm -f {root}/api/git/commits/$head {root}/api/compare/$held...$head; \
         fi; fi; echo a-token"
    )
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

/// The folder's own tree is archived, not the commit with the folder as a
/// pathspec, which reads the commit's whole tree into an index for its
/// attributes first: a large repository's took minutes and gigabytes. So what the
/// rest of main says about the folder's files never leaves one out.
#[test]
fn a_hosted_mains_folder_is_unpacked_from_its_own_tree() {
    let world = World::hosted_from(World::new(), counted_token);
    fs::write(
        world.origin.join(".gitattributes"),
        "machine/file export-ignore\n",
    )
    .unwrap();
    git(&world.origin, &["add", ".gitattributes"]);
    let head = world.hosted_commit("one", None);
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert_eq!(world.read("seen"), "one");
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

/// Main is fetched into a ref of the ask's own, so a git that checks only
/// what a fetch adds to the refs it moves walks back to the last ask's main
/// rather than through the whole repository: in a large one, 10 s rather
/// than 2 minutes.
#[test]
fn a_hosted_main_is_fetched_into_a_ref_of_its_own() {
    let world = World::hosted_from(World::new(), counted_token);
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    let first = world.hosted_commit("one", None);
    world.system(&first);
    assert_eq!(world.update().1, done(&first));
    assert_eq!(
        git(&checkout, &["rev-parse", "refs/update-to-main/main"]).trim(),
        first
    );
    let second = world.hosted_commit("two", None);
    world.system(&second);
    assert_eq!(world.update().1, done(&second));
    assert_eq!(
        git(&checkout, &["rev-parse", "refs/update-to-main/main"]).trim(),
        second
    );
}

/// The checkout is someone's own, so each move of main's ref is on record
/// in its reflog, though git keeps none for such a ref by default, and
/// none at all here, where the checkout's config says to keep none.
#[test]
fn a_hosted_mains_every_move_is_logged() {
    let world = World::hosted_from(World::new(), counted_token);
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    git(&checkout, &["config", "core.logAllRefUpdates", "false"]);
    let first = world.hosted_commit("one", None);
    world.system(&first);
    assert_eq!(world.update().1, done(&first));
    let second = world.hosted_commit("two", None);
    world.system(&second);
    assert_eq!(world.update().1, done(&second));
    assert_eq!(
        git(
            &checkout,
            &["reflog", "show", "--format=%H", "refs/update-to-main/main"]
        ),
        format!("{second}\n{first}"),
        "the ref's reflog should hold both fetches, newest first"
    );
}

/// A fetch runs none of git's upkeep in someone's checkout, whatever its
/// config asks: a large repository's repack took its disk for an hour.
/// Here each fetch keeps a pack, and the checkout's config asks, in the
/// foreground, both for gc past one pack and for maintenance's own
/// repack, which runs without gc: a second fetch would leave fewer packs
/// were gc run, and a multi-pack index, maintenance's repack's first step,
/// were that run. Its config is left as it was.
#[test]
fn a_hosted_fetch_runs_no_upkeep_in_the_checkout() {
    let world = World::hosted_from(World::new(), counted_token);
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    for (key, value) in [
        ("fetch.unpackLimit", "1"),
        ("gc.auto", "6700"),
        ("gc.autoPackLimit", "1"),
        ("gc.autoDetach", "false"),
        ("maintenance.auto", "true"),
        ("maintenance.incremental-repack.enabled", "true"),
        ("maintenance.incremental-repack.auto", "1"),
    ] {
        git(&checkout, &["config", key, value]);
    }
    let config = || git(&checkout, &["config", "--local", "--list"]);
    let config_before = config();
    let packs = || {
        fs::read_dir(checkout.join(".git/objects/pack"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "pack"))
            .count()
    };
    let before = packs();
    let first = world.hosted_commit("one", None);
    world.system(&first);
    assert_eq!(world.update().1, done(&first));
    let second = world.hosted_commit("two", None);
    world.system(&second);
    assert_eq!(world.update().1, done(&second));
    assert_eq!(
        packs(),
        before + 2,
        "each fetch should leave its own pack, none rolled together"
    );
    assert!(
        !checkout.join(".git/objects/pack/multi-pack-index").exists(),
        "no fetch should run maintenance's repack, which writes a multi-pack index"
    );
    assert_eq!(
        config(),
        config_before,
        "the checkout's config should be left as it was"
    );
}

/// A checkout whose git writes a commit graph after each fetch walks every
/// ref for it, so a side branch missing an old commit failed every fetch of
/// main, though main needs nothing of it: a machine went unupdated for a day.
#[test]
fn a_hosted_main_is_fetched_past_a_side_branch_missing_a_commit() {
    let world = World::hosted_from(World::new(), counted_token);
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    git(&checkout, &["config", "fetch.writeCommitGraph", "true"]);
    let first = world.hosted_commit("one", None);
    world.system(&first);
    assert_eq!(world.update().1, done(&first));
    let tree = git(&checkout, &["rev-parse", &format!("{first}^{{tree}}")]);
    let lost = git(
        &checkout,
        &["commit-tree", &tree, "-p", &first, "-m", "side one"],
    );
    let kept = git(
        &checkout,
        &["commit-tree", &tree, "-p", &lost, "-m", "side two"],
    );
    git(
        &checkout,
        &["update-ref", "refs/remotes/origin/side", &kept],
    );
    fs::remove_file(
        checkout
            .join(".git/objects")
            .join(&lost[..2])
            .join(&lost[2..]),
    )
    .unwrap();
    let head = world.hosted_commit("two", None);
    // Each on a copy, since a fetch failing at its graph has landed main
    // first: the same fetch fails with the graph's write and passes
    // without it, so the write alone is what fails it.
    let fetched_on_a_copy = |name: &str, before: &[&str]| {
        let copy = world.root.join(name);
        let copied = Command::new("cp")
            .arg("-R")
            .arg(&checkout)
            .arg(&copy)
            .status()
            .unwrap();
        assert!(copied.success(), "the checkout couldn't be copied");
        Command::new(program("git"))
            .arg("-C")
            .arg(&copy)
            .args(before)
            .args([
                "fetch",
                "--quiet",
                "origin",
                "+refs/heads/main:refs/plain/main",
            ])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("LC_ALL", "C")
            .output()
            .unwrap()
    };
    let plain = fetched_on_a_copy("plain", &[]);
    assert!(
        !plain.status.success()
            && String::from_utf8_lossy(&plain.stderr).contains(&format!("Could not read {lost}")),
        "a plain fetch got past the lost commit, so this proves nothing: {plain:?}"
    );
    let unwritten = fetched_on_a_copy("unwritten", &["-c", "fetch.writeCommitGraph=false"]);
    assert!(
        unwritten.status.success(),
        "the fetch failed without writing the graph too, so the write isn't what fails it: \
         {unwritten:?}"
    );
    assert!(
        !world.checkout_has(&head),
        "main was in the checkout before the update fetched it"
    );
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert!(
        world.checkout_has(&head),
        "main wasn't fetched into the checkout"
    );
    assert_eq!(
        git(&checkout, &["config", "fetch.writeCommitGraph"]),
        "true",
        "the checkout's own config changed"
    );
}

/// A hosted world whose host has a REST API, its answers files under the
/// world's own folder that curl reads as it would the host's over HTTPS.
/// `World::answer` writes them for everything main holds.
fn with_api() -> World {
    api_on(World::hosted_from(World::new(), counted_token))
}

/// `world`, hosted, reading its folder through the host's API.
fn api_on(mut world: World) -> World {
    let api = format!("file://{}", world.root.join("api").display());
    world.config.hosted.as_mut().unwrap().api = Some(api);
    world
}

impl World {
    /// Writes the host's API answers for every commit, tree and blob the
    /// remote holds, and how each two of its commits stand.
    fn answer(&self) {
        let api = self.root.join("api");
        for d in ["git/commits", "git/trees", "git/blobs", "compare"] {
            fs::create_dir_all(api.join(d)).unwrap();
        }
        let objects = git(
            &self.origin,
            &[
                "cat-file",
                "--batch-all-objects",
                "--batch-check=%(objecttype) %(objectname)",
            ],
        );
        let mut commits = Vec::new();
        for line in objects.lines() {
            let (kind, id) = line.split_once(' ').unwrap();
            let answer = match kind {
                "commit" => {
                    commits.push(id.to_owned());
                    let tree = git(&self.origin, &["rev-parse", &format!("{id}^{{tree}}")]);
                    serde_json::json!({"sha": id, "tree": {"sha": tree.trim()}})
                }
                "tree" => {
                    let entries: Vec<_> = git(&self.origin, &["ls-tree", id])
                        .lines()
                        .map(|entry| {
                            let (about, name) = entry.split_once('\t').unwrap();
                            let about: Vec<&str> = about.split(' ').collect();
                            serde_json::json!({
                                "path": name, "mode": about[0], "type": about[1], "sha": about[2]
                            })
                        })
                        .collect();
                    serde_json::json!({"sha": id, "truncated": false, "tree": entries})
                }
                "blob" => {
                    let out = Command::new(program("git"))
                        .arg("-C")
                        .arg(&self.origin)
                        .args(["cat-file", "blob", id])
                        .output()
                        .unwrap();
                    serde_json::json!({
                        "sha": id, "encoding": "base64", "content": base64(&out.stdout)
                    })
                }
                _ => continue,
            };
            let folder = match kind {
                "commit" => "git/commits",
                "tree" => "git/trees",
                _ => "git/blobs",
            };
            fs::write(api.join(folder).join(id), answer.to_string()).unwrap();
        }
        for older in &commits {
            for newer in &commits {
                let is_ancestor = |a: &str, b: &str| {
                    Command::new(program("git"))
                        .arg("-C")
                        .arg(&self.origin)
                        .args(["merge-base", "--is-ancestor", a, b])
                        .status()
                        .unwrap()
                        .success()
                };
                let status = if older == newer {
                    "identical"
                } else if is_ancestor(older, newer) {
                    "ahead"
                } else if is_ancestor(newer, older) {
                    "behind"
                } else {
                    "diverged"
                };
                let answer = serde_json::json!({
                    "status": status, "merge_base_commit": {"sha": older}
                });
                fs::write(
                    api.join("compare").join(format!("{older}...{newer}")),
                    answer.to_string(),
                )
                .unwrap();
            }
        }
    }

    /// Whether the account's checkout holds `commit`.
    fn checkout_has(&self, commit: &str) -> bool {
        let checkout = &self.config.hosted.as_ref().unwrap().checkout;
        Command::new(program("git"))
            .arg("-C")
            .arg(checkout)
            .args(["cat-file", "-e", &format!("{commit}^{{commit}}")])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    }
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::new();
    for (n, chunk) in bytes.chunks(3).enumerate() {
        // The host breaks its lines every 60 characters.
        if n > 0 && n % 15 == 0 {
            text.push('\n');
        }
        let bits = chunk
            .iter()
            .enumerate()
            .fold(0u32, |bits, (i, b)| bits | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                text.push(ALPHABET[((bits >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// A fetch that moves main in a large repository checks the new head's
/// whole tree is there, which took six minutes of seven on 10 November.
/// Where the host has an API, the folder is read from it, and the checkout
/// is never fetched.
#[test]
fn a_hosted_main_is_read_from_its_hosts_api_unfetched() {
    let world = with_api();
    let head = world.hosted_commit("one", None);
    world.answer();
    let system = world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert_eq!(world.running(), system);
    assert_eq!(world.read("seen"), "one");
    assert!(!world.checkout_has(&head), "the checkout was fetched");
    let two = world.hosted_commit("two", None);
    world.answer();
    world.system(&two);
    assert_eq!(world.update().1, done(&two));
    assert_eq!(world.read("seen"), "onetwo");
    assert!(!world.checkout_has(&two), "the checkout was fetched");
}

/// A folder read from the host where it was needs no word on how main
/// moved, since the machine stays where it is: on 12 November the host
/// refused that ask with 429 and 422, and each time main was fetched for
/// minutes though its folder hadn't moved.
#[test]
fn a_folder_read_from_the_host_where_it_was_never_asks_how_main_moved() {
    let world = with_api();
    let one = world.hosted_commit("one", None);
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    let elsewhere = world.unsigned("outside the folder");
    world.answer();
    let compare = world.root.join("api").join("compare");
    fs::remove_dir_all(&compare).unwrap();
    fs::create_dir_all(&compare).unwrap();
    assert_eq!(
        world.update().1,
        Line::Unchanged {
            commit: one.clone(),
            followed: BTreeMap::new(),
            boots: None,
        }
    );
    assert!(
        !world.checkout_has(&elsewhere),
        "main was fetched though its folder hadn't moved"
    );
    assert_eq!(world.builds(), 1);
}

/// Main moving outside the folder read from the host builds nothing, as
/// with a folder the account says hasn't moved.
#[test]
fn a_folder_read_from_the_host_where_it_was_is_not_built() {
    let world = with_api();
    let one = world.hosted_commit("one", None);
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    world.unsigned("outside the folder");
    world.answer();
    assert_eq!(
        world.update().1,
        Line::Unchanged {
            commit: one.clone(),
            followed: BTreeMap::new(),
            boots: None,
        }
    );
    assert_eq!(world.builds(), 1);
}

/// Main's folder where it was, read from the host, with a followed input
/// moved on is built again with the input: main where it was descends from
/// itself, though root's copy never held it.
#[test]
fn a_folder_read_from_the_host_where_it_was_with_its_input_moved_on_is_built_again() {
    let world = api_on(World::hosted_from(following(), counted_token));
    let pinned = world.core_commit("pinned", true);
    let one = world.hosted_commit("one", Some(&pinning(&pinned)));
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, following_done(&one, &pinned));
    assert!(!world.checkout_has(&one), "main was fetched");
    world.unsigned("outside the folder");
    let newer = world.core_commit("newer", true);
    world.answer();
    world.system(&format!("{one}+{newer}"));
    assert_eq!(world.update().1, following_done(&one, &newer));
    assert_eq!(world.builds(), 2);
}

/// A file the host gives with contents its tree doesn't name is never
/// built: main is fetched instead, and built from the checkout's objects.
#[test]
fn a_host_whose_api_gives_another_file_is_fetched_from() {
    let world = with_api();
    let head = world.hosted_commit("one", None);
    world.answer();
    let file = git(
        &world.origin,
        &["rev-parse", &format!("{head}:machine/file")],
    );
    let answer = world.root.join("api/git/blobs").join(file.trim());
    let lie = serde_json::json!({"sha": file.trim(), "encoding": "base64", "content": base64(b"the host's own")});
    fs::write(&answer, lie.to_string()).unwrap();
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert_eq!(world.read("seen"), "one");
    assert!(world.checkout_has(&head), "main wasn't fetched");
}

/// The host's API answering for another commit than the head `ls-remote`
/// named is never read from.
#[test]
fn a_host_whose_api_answers_for_another_commit_is_fetched_from() {
    let world = with_api();
    let one = world.hosted_commit("one", None);
    let two = world.hosted_commit("two", None);
    world.answer();
    let commits = world.root.join("api/git/commits");
    fs::copy(commits.join(&one), commits.join(&two)).unwrap();
    world.system(&two);
    assert_eq!(world.update().1, done(&two));
    assert_eq!(world.read("seen"), "two");
    assert!(world.checkout_has(&two), "main wasn't fetched");
}

/// An API that doesn't answer leaves main to be fetched, as without one.
#[test]
fn a_host_whose_api_doesnt_answer_is_fetched_from() {
    let world = with_api();
    let head = world.hosted_commit("one", None);
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert!(world.checkout_has(&head), "main wasn't fetched");
}

/// A hosted world reading its folder through the host's API, whose account
/// says main mayn't be fetched now, and names the commit its checkout
/// holds at `refs/kept`.
fn unfetchable() -> World {
    let mut world = with_api();
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    let hosted = world.config.hosted.as_mut().unwrap();
    hosted.may_fetch = Some(vec![shell(), "-c".to_owned(), "exit 1".to_owned()]);
    hosted.fetched = Some(vec![
        shell(),
        "-c".to_owned(),
        format!("git -C {} rev-parse refs/kept", checkout.display()),
    ]);
    world
}

impl World {
    /// Keeps main as it is now in the checkout at `refs/kept`, as another
    /// fetch would.
    fn keep_main(&self) {
        let checkout = &self.config.hosted.as_ref().unwrap().checkout;
        git(checkout, &["fetch", "--quiet", "origin", "+main:refs/kept"]);
    }

    /// Takes the host's API answer for `commit` away, so reading its
    /// folder fails as the host's did on 12 November.
    fn unanswer(&self, commit: &str) {
        fs::remove_file(self.root.join("api/git/commits").join(commit)).unwrap();
    }
}

/// A fetch that moves a large repository's main took 10 minutes and 17.7 GB
/// on 12 November, by day, when reading the folder failed. While the
/// account says main mayn't be fetched, a head the checkout holds nothing
/// newer than is left for a later ask, and the machine stays where it is.
#[test]
fn a_head_that_mayn_t_be_fetched_leaves_the_machine_where_it_is() {
    let world = unfetchable();
    let one = world.hosted_commit("one", None);
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    world.keep_main();
    let two = world.hosted_commit("two", None);
    world.answer();
    world.unanswer(&two);
    world.system(&two);
    assert_eq!(
        world.update().1,
        Line::Unchanged {
            commit: one.clone(),
            followed: BTreeMap::new(),
            boots: None,
        }
    );
    assert!(!world.checkout_has(&two), "main was fetched");
    assert_eq!(world.builds(), 1);
    // The next ask reads the folder again, and takes the head once it can.
    world.answer();
    assert_eq!(world.update().1, done(&two));
    assert!(!world.checkout_has(&two), "main was fetched");
}

/// While main mayn't be fetched, the commit the checkout already holds is
/// built, once the host says main holds it.
#[test]
fn a_head_that_mayn_t_be_fetched_takes_the_commit_the_checkout_holds() {
    let world = unfetchable();
    let one = world.hosted_commit("one", None);
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    let two = world.hosted_commit("two", None);
    world.keep_main();
    let three = world.hosted_commit("three", None);
    world.answer();
    world.unanswer(&three);
    world.system(&two);
    assert_eq!(world.update().1, done(&two));
    assert_eq!(world.read("seen"), "onetwo");
    assert!(!world.checkout_has(&three), "main was fetched");
}

/// The checkout's commit is the account's word, so one main doesn't hold
/// is never built: the machine stays where it is.
#[test]
fn a_commit_the_checkout_holds_that_main_doesnt_is_never_taken() {
    let world = unfetchable();
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    let one = world.hosted_commit("one", None);
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    world.keep_main();
    git(&checkout, &["checkout", "--quiet", "refs/kept"]);
    fs::write(checkout.join("machine/file"), "the account's own").unwrap();
    git(
        &checkout,
        &["commit", "--quiet", "-am", "the account's own"],
    );
    let own = git(&checkout, &["rev-parse", "HEAD"]);
    git(&checkout, &["update-ref", "refs/kept", &own]);
    let two = world.hosted_commit("two", None);
    world.answer();
    world.unanswer(&two);
    world.system(&own);
    assert_eq!(
        world.update().1,
        Line::Unchanged {
            commit: one,
            followed: BTreeMap::new(),
            boots: None,
        }
    );
    assert_eq!(world.read("seen"), "one");
}

/// Whether main may be fetched is asked only once main must come from the
/// checkout, so an ask whose folder the host reads runs nothing of the
/// account's.
#[test]
fn a_head_the_host_reads_never_asks_whether_main_may_be_fetched() {
    let mut world = unfetchable();
    let asked = world.root.join("asked");
    world.config.hosted.as_mut().unwrap().may_fetch = Some(vec![
        shell(),
        "-c".to_owned(),
        format!("touch {}; exit 1", asked.display()),
    ]);
    let one = world.hosted_commit("one", None);
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    let two = world.hosted_commit("two", None);
    world.answer();
    world.system(&two);
    assert_eq!(world.update().1, done(&two));
    assert!(!asked.exists(), "the account was asked whether to fetch");
}

/// A machine that has never switched has nowhere to stay, so its ask fails
/// rather than fetching main.
#[test]
fn a_first_head_that_mayn_t_be_fetched_fails_the_ask() {
    let world = unfetchable();
    let one = world.hosted_commit("one", None);
    world.system(&one);
    let (_, ending) = world.update();
    let Line::Failed { why, .. } = &ending else {
        panic!("not failed: {ending:?}")
    };
    assert!(why.contains("mayn't be fetched now"), "{why}");
    assert!(!world.checkout_has(&one), "main was fetched");
}

/// A main the host says doesn't descend from what the machine last ran is
/// refused, as one root's copy shows doesn't.
#[test]
fn a_main_the_host_says_doesnt_descend_is_refused() {
    let world = with_api();
    let start = world.unsigned("start");
    let one = world.hosted_commit("one", None);
    world.answer();
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    git(&world.origin, &["reset", "--quiet", "--hard", &start]);
    let two = world.hosted_commit("two", None);
    world.answer();
    world.system(&two);
    let (_, ending) = world.update();
    let Line::Refused { why, .. } = &ending else {
        panic!("not refused: {ending:?}")
    };
    assert!(why.contains("doesn't descend"), "{why}");
    assert!(!world.checkout_has(&two), "the checkout was fetched");
}

/// A hosted world whose account names the commit that last changed the
/// folder with the shell command `changed` gives for the world's folder.
fn with_changed(changed: impl Fn(&Path) -> String) -> World {
    let mut world = World::hosted_from(World::new(), counted_token);
    let changed = changed(&world.root);
    world.config.hosted.as_mut().unwrap().changed = Some(vec![shell(), "-c".to_owned(), changed]);
    world
}

/// The commit that last changed the folder on the remote's main, as the
/// host would name it.
fn last_changed(world_root: &Path) -> String {
    format!(
        "git -C {} log -1 --format=%H main -- machine",
        world_root.join("origin").display()
    )
}

/// Main moves every few minutes in a large repository, mostly outside the
/// flake's folder, and fetching it took minutes each ask. While the folder
/// is as it was last built, nothing is fetched or built.
#[test]
fn a_hosted_folder_where_it_was_is_neither_fetched_nor_built() {
    let world = with_changed(last_changed);
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    let one = world.hosted_commit("one", None);
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    let elsewhere = world.unsigned("outside the folder");
    assert_eq!(
        world.update().1,
        Line::Unchanged {
            commit: one.clone(),
            followed: BTreeMap::new(),
            boots: None,
        }
    );
    assert_eq!(
        git(&checkout, &["rev-parse", "refs/update-to-main/main"]),
        one,
        "main at {elsewhere} was fetched"
    );
    assert_eq!(world.builds(), 1);
    let two = world.hosted_commit("two", None);
    world.system(&two);
    assert_eq!(world.update().1, done(&two));
    assert_eq!(world.read("seen"), "onetwo");
}

/// The account's answer is only ever a reason not to fetch: a commit of its
/// own, whose folder isn't the one last built, is never built, and main is
/// fetched as the host names it.
#[test]
fn a_hosted_folder_the_account_names_wrongly_is_fetched_from_main() {
    let world =
        with_changed(|root| format!("git -C {} rev-parse HEAD", root.join("checkout").display()));
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    let one = world.hosted_commit("one", None);
    world.system(&one);
    assert_eq!(world.update().1, done(&one));
    git(&checkout, &["checkout", "--quiet", &one]);
    fs::write(checkout.join("machine/file"), "the account's own").unwrap();
    git(
        &checkout,
        &["commit", "--quiet", "-am", "the account's own"],
    );
    let two = world.hosted_commit("two", None);
    world.system(&two);
    assert_eq!(world.update().1, done(&two));
    assert_eq!(world.read("seen"), "onetwo");
}

#[test]
fn a_hosted_checkout_only_its_accounts_git_reads_is_read_with_that_git() {
    let mut world = World::hosted_from(World::new(), counted_token);
    let head = world.hosted_commit("one", None);
    let system = world.system(&head);
    // Root's git refuses the checkout, as a stock git refuses a repository
    // marked with an extension only the account's git knows, and reads
    // everything else as git does.
    let checkout = world.config.hosted.as_ref().unwrap().checkout.clone();
    let root_git = world.root.join("stand-ins/root-git");
    write_program(
        &root_git,
        &format!(
            "{}\nif [ \"$PWD\" = '{}' ]; then\n  echo 'fatal: unknown repository extension found:' >&2\n  exit 128\nfi\nexec '{}' \"$@\"\n",
            stand_in_head(),
            checkout.display(),
            program("git").display()
        ),
    );
    world.config.tools.git = root_git;
    assert_eq!(world.update().1, done(&head));
    assert_eq!(world.running(), system);
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

/// A lock with a private input, one in the store already and a public one.
const PRIVATE_LOCK: &str = r#"{
  "root": "root",
  "nodes": {
    "root": { "inputs": { "private": "private_2", "pinned": "pinned", "public": "public" } },
    "private_2": { "locked": { "type": "git", "url": "https://example.com/private", "rev": "abc" } },
    "pinned": { "locked": { "type": "path", "path": "/nix/store/abc-pinned" } },
    "public": { "locked": { "type": "github", "owner": "o", "repo": "r" } }
  },
  "version": 7
}"#;

/// A hosted world that fetches the lock's private and pinned inputs.
fn with_private_inputs() -> World {
    let mut world = World::hosted_from(World::new(), counted_token);
    world.config.hosted.as_mut().unwrap().inputs = vec!["private".to_owned(), "pinned".to_owned()];
    world
}

impl World {
    /// Says the inputs fetched were the lock's private one, by its lock, and
    /// its pinned one, by its path.
    fn fetched_the_private_inputs(&self) {
        let fetched = self.read("fetched");
        let fetched: Vec<&str> = fetched.lines().collect();
        assert_eq!(fetched.len(), 2, "{fetched:?}");
        let private: serde_json::Value = serde_json::from_str(fetched[0]).unwrap();
        assert_eq!(private["url"], "https://example.com/private");
        assert_eq!(fetched[1], "/nix/store/abc-pinned");
    }

    /// A Mac's launchctl, stood in for: it writes down what it's asked, knows
    /// the account's login session while `session` is there, and runs a job
    /// it's given as launchd would, apart, by the plist's last two strings,
    /// with the update-to-main these tests build. Each plist it's given is
    /// written down in `plists`.
    fn on_a_mac(&mut self) {
        let launchctl = self.root.join("launchctl");
        write_program(
            &launchctl,
            &format!(
                "{head}\necho \"$1 $2\" >> {asked}\n\
                 case $1 in\n\
                 print) [ -e {session} ] || exit 113;;\n\
                 bootstrap) echo \"$3\" >> {plists}\n\
                 strings=$(sed -n 's|.*<string>\\(.*\\)</string>.*|\\1|p' \"$3\")\n\
                 [ \"$(echo \"$strings\" | tail -n 2 | head -n 1)\" = job ] || exit 2\n\
                 {job} job \"$(echo \"$strings\" | tail -n 1)\" > /dev/null 2>&1 &;;\n\
                 esac\n",
                head = stand_in_head(),
                asked = self.root.join("launchctl-asked").display(),
                plists = self.root.join("plists").display(),
                session = self.root.join("session").display(),
                job = env!("CARGO_BIN_EXE_update-to-main"),
            ),
        );
        self.config.session = Some(Session {
            launchctl,
            jobs: self.root.join("jobs"),
        });
    }
}

#[test]
fn a_hosted_mains_private_inputs_are_fetched_by_their_locks() {
    let world = with_private_inputs();
    let head = world.hosted_commit("one", Some(PRIVATE_LOCK));
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    world.fetched_the_private_inputs();
}

#[test]
fn on_a_mac_private_inputs_are_fetched_in_the_login_session() {
    let mut world = with_private_inputs();
    world.on_a_mac();
    fs::write(world.root.join("session"), "").unwrap();
    let head = world.hosted_commit("one", Some(PRIVATE_LOCK));
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    world.fetched_the_private_inputs();
    // SAFETY: getuid only reads.
    let domain = format!("gui/{}", unsafe { libc::getuid() });
    let asked = world.read("launchctl-asked");
    let started = asked
        .lines()
        .filter(|l| *l == format!("bootstrap {domain}"))
        .count();
    let stopped = asked
        .lines()
        .filter(|l| l.starts_with(&format!("bootout {domain}/")))
        .count();
    assert_eq!((started, stopped), (2, 2), "{asked}");
    // Each job's folder is made in the jobs folder, which the account can
    // pass through, not in root's own temporary folder, which on a Mac it
    // can't.
    let jobs = world.root.join("jobs");
    let plists = world.read("plists");
    assert!(
        plists.lines().all(|p| Path::new(p).starts_with(&jobs)),
        "{plists}"
    );
    let mode = fs::metadata(&jobs).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o711);
}

#[test]
fn on_a_mac_nobody_logged_in_private_inputs_are_fetched_as_they_are() {
    let mut world = with_private_inputs();
    world.on_a_mac();
    let head = world.hosted_commit("one", Some(PRIVATE_LOCK));
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    world.fetched_the_private_inputs();
    let asked = world.read("launchctl-asked");
    assert!(asked.lines().all(|l| l.starts_with("print ")), "{asked}");
}

#[test]
fn on_a_mac_a_fetch_that_fails_in_the_login_session_says_why() {
    let mut world = with_private_inputs();
    world.on_a_mac();
    fs::write(world.root.join("session"), "").unwrap();
    fs::write(world.root.join("fetch-fails"), "").unwrap();
    let head = world.hosted_commit("one", Some(PRIVATE_LOCK));
    world.system(&head);
    let Line::Failed { why, lines, .. } = world.update().1 else {
        panic!("the update didn't fail")
    };
    assert_eq!(why, "fetching the input private failed (exit status: 1)");
    assert_eq!(lines, ["could not read Username"]);
    assert_eq!(world.builds(), 0);
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

/// A world whose flake has an input, core, that follows its own main in a
/// repository of its own, signed by the machine's trusted key.
fn following() -> World {
    let mut world = World::new();
    let core = world.root.join("core");
    fs::create_dir_all(&core).unwrap();
    git(&core, &["init", "--quiet", "--initial-branch=main"]);
    world.config.follows.insert(
        "core".to_owned(),
        Follow {
            repository: format!("file://{}", core.display()),
            branch: "main".to_owned(),
            signers: world.config.signers.clone(),
            asked: false,
        },
    );
    world
}

/// A world, `world`, whose input core has an input of its own, mycelium,
/// in a repository of its own, that moves only when an ask names it.
fn asking_nested(mut world: World) -> World {
    let mycelium = world.root.join("mycelium");
    fs::create_dir_all(&mycelium).unwrap();
    git(&mycelium, &["init", "--quiet", "--initial-branch=main"]);
    world.config.follows.insert(
        NESTED.to_owned(),
        Follow {
            repository: format!("file://{}", mycelium.display()),
            branch: "main".to_owned(),
            signers: world.config.signers.clone(),
            asked: true,
        },
    );
    world
}

/// The input of core's that `asking_nested` follows, by its path.
const NESTED: &str = "core/mycelium";

impl World {
    /// Commits to core's main, signed by the trusted key or not.
    fn core_commit(&self, message: &str, signed: bool) -> String {
        self.commit_in("core", message, signed)
    }

    /// Commits to core's main a signed commit whose lock pins mycelium at
    /// `pinned`.
    fn core_pinning(&self, message: &str, pinned: &str) -> String {
        let lock = serde_json::json!({
            "nodes": {
                "root": { "inputs": { "mycelium": "mycelium" } },
                "mycelium": { "locked": { "type": "github", "rev": pinned } },
            },
            "root": "root",
            "version": 7,
        });
        let core = self.root.join("core");
        fs::write(core.join("flake.lock"), lock.to_string()).unwrap();
        git(&core, &["add", "flake.lock"]);
        self.core_commit(message, true)
    }

    /// Commits to the repository `name` under the world's root, signed by
    /// the trusted key or not.
    fn commit_in(&self, name: &str, message: &str, signed: bool) -> String {
        let core = self.root.join(name);
        fs::write(core.join("file"), message).unwrap();
        git(&core, &["add", "file"]);
        let key = format!("user.signingkey={}", self.key.display());
        let mut args = vec!["-c", "gpg.format=ssh", "-c", &key, "commit", "--quiet"];
        if signed {
            args.push("-S");
        }
        args.extend(["-m", message]);
        git(&core, &args);
        git(&core, &["rev-parse", "HEAD"])
    }

    /// Commits a signed main whose lock pins core at `pinned`.
    fn signed_pinning(&self, message: &str, pinned: &str) -> String {
        fs::write(self.origin.join("flake.lock"), pinning(pinned)).unwrap();
        git(&self.origin, &["add", "flake.lock"]);
        self.signed(message)
    }
}

/// A lock pinning core at `core` and core's own mycelium at `mycelium`.
fn pinning_nested(core: &str, mycelium: &str) -> String {
    serde_json::json!({
        "nodes": {
            "root": { "inputs": { "core": "core" } },
            "core": {
                "inputs": { "mycelium": "mycelium" },
                "locked": { "type": "github", "rev": core },
            },
            "mycelium": { "locked": { "type": "github", "rev": mycelium } },
        },
        "root": "root",
        "version": 7,
    })
    .to_string()
}

/// A lock pinning core at `pinned`.
fn pinning(pinned: &str) -> String {
    serde_json::json!({
        "nodes": {
            "root": { "inputs": { "core": "core" } },
            "core": { "locked": { "type": "github", "rev": pinned } },
        },
        "root": "root",
        "version": 7,
    })
    .to_string()
}

fn following_done(commit: &str, core: &str) -> Line {
    done_following(commit, &[("core", core)])
}

/// The line ending an update of main at `commit` with each input named in
/// `followed` at its commit.
fn done_following(commit: &str, followed: &[(&str, &str)]) -> Line {
    let Line::Done {
        commit,
        applied,
        restart,
        restarted,
        ..
    } = done(commit)
    else {
        unreachable!()
    };
    Line::Done {
        commit,
        applied,
        restart,
        restarted,
        boots: None,
        followed: followed
            .iter()
            .map(|(name, commit)| ((*name).to_owned(), (*commit).to_owned()))
            .collect(),
    }
}

#[test]
fn a_followed_input_is_built_at_its_signed_main_past_the_lock() {
    let world = following();
    let pinned = world.core_commit("pinned", true);
    let newer = world.core_commit("newer", true);
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    assert_eq!(world.update().1, following_done(&head, &newer));
    let overridden = world.read("overridden");
    assert!(overridden.starts_with("core git+file://"), "{overridden}");
    assert!(
        overridden.trim().ends_with(&format!("rev={newer}")),
        "{overridden}"
    );
}

#[test]
fn main_where_it_was_with_its_input_moved_on_is_built_again() {
    let world = following();
    let pinned = world.core_commit("pinned", true);
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    world.update();
    assert_eq!(
        world.update().1,
        Line::Unchanged {
            commit: head.clone(),
            followed: BTreeMap::from([("core".to_owned(), pinned)]),
            boots: None,
        }
    );
    let newer = world.core_commit("newer", true);
    world.system(&format!("{head}+{newer}"));
    assert_eq!(world.update().1, following_done(&head, &newer));
    assert_eq!(world.builds(), 2);
}

#[test]
fn a_followed_input_unsigned_is_refused_and_nothing_is_built() {
    let world = following();
    let pinned = world.core_commit("pinned", true);
    world.core_commit("unsigned", false);
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(why.starts_with("core's main: "), "{why}");
    assert!(
        why.contains("isn't signed by a key this machine trusts"),
        "{why}"
    );
    assert_eq!(world.builds(), 0);
}

#[test]
fn a_followed_input_behind_its_lock_is_refused() {
    let world = following();
    let older = world.core_commit("older", true);
    let pinned = world.core_commit("pinned", true);
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    git(
        &world.root.join("core"),
        &["reset", "--quiet", "--hard", &older],
    );
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(why.contains("which the flake's lock pins"), "{why}");
    assert_eq!(world.builds(), 0);
}

#[test]
fn an_ask_at_a_followed_commit_builds_it_rather_than_its_mains_head() {
    let world = following();
    let pinned = world.core_commit("pinned", true);
    let wanted = world.core_commit("wanted", true);
    let newer = world.core_commit("newer", true);
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    assert_eq!(
        world.update_at("core", &wanted),
        following_done(&head, &wanted),
        "an ask at a commit on core's main didn't build that commit"
    );
    let overridden = world.read("overridden");
    assert!(
        overridden.trim().ends_with(&format!("rev={wanted}")),
        "the build wasn't given the commit asked for: {overridden}"
    );
    world.system(&format!("{head}+{newer}"));
    assert_eq!(
        world.update().1,
        following_done(&head, &newer),
        "the next plain ask didn't take core's main again"
    );
}

#[test]
fn an_ask_at_a_commit_before_a_held_one_switches_while_the_hold_stands() {
    let world = following();
    let pinned = world.core_commit("pinned", true);
    let before = world.core_commit("before", true);
    let walk = world.core_commit("the walk", true);
    world.core_commit("after", true);
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    held::ask(&world.config, &format!("hold {walk}"), Some(1000));
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("core's main holding a held commit wasn't refused")
    };
    assert!(why.contains("is held"), "refused for another reason: {why}");
    assert_eq!(
        world.update_at("core", &before),
        following_done(&head, &before),
        "an ask at a commit before the held one didn't switch"
    );
    let Line::Refused { why, .. } = world.update_at("core", &walk) else {
        panic!("an ask at the held commit itself wasn't refused")
    };
    assert!(why.contains("is held"), "refused for another reason: {why}");
}

#[test]
fn an_ask_at_a_commit_off_its_main_is_refused_and_nothing_is_built() {
    let world = following();
    let pinned = world.core_commit("pinned", true);
    let core = world.root.join("core");
    git(&core, &["checkout", "--quiet", "-b", "aside"]);
    let aside = world.core_commit("aside", true);
    git(&core, &["checkout", "--quiet", "main"]);
    let from = format!("file://{}", core.display());
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    world.update();
    let copy = world.config.follows_folder().join("core.git");
    git(&copy, &["fetch", "--quiet", &from, "aside"]);
    let built = world.builds();
    let Line::Refused { why, .. } = world.update_at("core", &aside) else {
        panic!("an ask at a commit off core's main wasn't refused")
    };
    assert!(
        why.contains("isn't on its main"),
        "refused for another reason: {why}"
    );
    assert_eq!(world.builds(), built, "a commit off core's main was built");
}

#[test]
fn an_ask_at_a_commit_behind_what_was_followed_or_of_no_followed_input_is_refused() {
    let world = following();
    let pinned = world.core_commit("pinned", true);
    let older = world.core_commit("older", true);
    let newer = world.core_commit("newer", true);
    let head = world.signed_pinning("one", &pinned);
    world.system(&head);
    assert_eq!(world.update().1, following_done(&head, &newer));
    let built = world.builds();
    let Line::Refused { why, .. } = world.update_at("core", &older) else {
        panic!("an ask behind what the machine last followed wasn't refused")
    };
    assert!(
        why.contains("which this machine last followed"),
        "refused for another reason: {why}"
    );
    let Line::Refused { why, .. } = world.update_at("nixpkgs", &newer) else {
        panic!("an ask at an input this machine doesn't follow wasn't refused")
    };
    assert!(
        why.contains("isn't an input this machine follows"),
        "refused for another reason: {why}"
    );
    assert_eq!(world.builds(), built, "a refused ask built something");
}

#[test]
fn an_input_asked_for_moves_only_when_asked_and_stays_till_the_lock_passes_it() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(
        world.update().1,
        done(&head),
        "an input asked for moved with no ask naming it"
    );
    assert!(
        !world.root.join("overridden").exists(),
        "a plain ask overrode an input asked for"
    );
    world.system(&format!("{head}+{wanted}"));
    assert_eq!(
        world.update_at(NESTED, &wanted),
        done_following(&head, &[(NESTED, &wanted)])
    );
    let overridden = world.read("overridden");
    assert!(
        overridden.starts_with(&format!("{NESTED} git+file://"))
            && overridden.trim().ends_with(&format!("rev={wanted}")),
        "the build wasn't given core's mycelium at the commit asked for: {overridden}"
    );
    world.commit_in("mycelium", "newer", true);
    // Matched on what it judges, so a field the line gains elsewhere doesn't
    // change what this case asks.
    let kept = BTreeMap::from([(NESTED.to_owned(), wanted.clone())]);
    let ending = world.update().1;
    assert!(
        matches!(&ending, Line::Unchanged { commit, followed, .. }
            if *commit == head && *followed == kept),
        "a plain ask didn't keep mycelium where it was asked, past the lock's pin: {ending:?}"
    );
    // A lock gone off the line of what was asked rules again, rather than
    // refusing every update with no ask able to clear it.
    let mycelium = world.root.join("mycelium");
    git(&mycelium, &["checkout", "--quiet", "-b", "side", &pinned]);
    let side = world.commit_in("mycelium", "side", true);
    git(&mycelium, &["checkout", "--quiet", "main"]);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &side),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let aside = world.signed("aside");
    world.system(&aside);
    assert_eq!(
        world.update().1,
        done(&aside),
        "mycelium was held where it was asked once the lock pinned it off that line"
    );
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &wanted),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let caught_up = world.signed("two");
    world.system(&caught_up);
    assert_eq!(
        world.update().1,
        done(&caught_up),
        "mycelium was still overridden once the lock pinned what was asked"
    );
}

#[test]
fn an_input_asked_for_is_let_go_once_the_lock_no_longer_has_it() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let head = world.signed("one");
    world.system(&format!("{head}+{wanted}"));
    assert_eq!(
        world.update_at(NESTED, &wanted),
        done_following(&head, &[(NESTED, &wanted)]),
        "the ask at mycelium wasn't built"
    );
    fs::write(world.origin.join("flake.lock"), pinning(&core)).unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let dropped = world.signed("two");
    world.system(&dropped);
    assert_eq!(
        world.update().1,
        done(&dropped),
        "a lock without mycelium didn't let go of the ask kept for it"
    );
}

/// A lock that no longer has a followed input builds without it, so the
/// follow is left out rather than failing every update, the one that would
/// switch to a config without it included; an ask at it is refused.
#[test]
fn a_follow_its_lock_no_longer_has_is_left_out_and_no_ask_may_name_it() {
    let mut world = asking_nested(World::new());
    if let Some(follow) = world.config.follows.get_mut(NESTED) {
        follow.asked = false;
    }
    let mycelium = world.commit_in("mycelium", "one", true);
    let core = "c".repeat(40);
    fs::write(world.origin.join("flake.lock"), pinning(&core)).unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(
        world.update().1,
        done(&head),
        "a follow its lock doesn't have failed the update"
    );
    let Line::Refused { why, .. } = world.update_at(NESTED, &mycelium) else {
        panic!("an ask at an input the lock doesn't have wasn't refused")
    };
    assert!(
        why.contains("isn't in the lock"),
        "an ask at an input the lock doesn't have was refused for another reason: {why}"
    );
}

/// An ask at an input that moves only when asked, whose switch waits for a
/// hold, is built again by the next update rather than undone by it: its
/// commit is remembered only once it switches.
#[test]
fn an_ask_waiting_for_a_hold_is_kept_by_the_next_update_and_switched_when_granted() {
    let mut world = asking_nested(World::new());
    hold_the_daemon(&mut world, "old");
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    daemon_unit(&world.system(&format!("{one}+{wanted}")), "new");
    let Line::Refused { why, .. } = world.update_at(NESTED, &wanted) else {
        panic!("an ask restarting a held daemon didn't wait for a hold")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    let two = world.signed("two");
    daemon_unit(&world.system(&two), "old");
    let waits = world.system(&format!("{two}+{wanted}"));
    daemon_unit(&waits, "new");
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("the next update undid the ask that waited for a hold")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    fs::write(world.root.join("given"), "").unwrap();
    assert_eq!(
        world.update().1,
        done_following(&two, &[(NESTED, &wanted)]),
        "the grant didn't switch to mycelium at the commit asked for"
    );
    assert_eq!(
        world.running(),
        waits,
        "the grant switched to another build"
    );
}

/// A hold of a commit of an input left at its lock's pin is judged in its
/// own repository, by whether the pin holds it: main's host knows no other
/// repository's commits.
#[test]
fn a_hold_on_an_input_left_at_its_pin_is_judged_in_its_own_repository() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let later = world.commit_in("mycelium", "later", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    world.system(&format!("{one}+{wanted}"));
    assert_eq!(
        world.update_at(NESTED, &wanted),
        done_following(&one, &[(NESTED, &wanted)]),
        "the ask at mycelium wasn't built"
    );
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &later),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let two = world.signed("two");
    world.system(&two);
    held::ask(&world.config, &format!("hold {wanted}"), Some(1000));
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a hold of mycelium's commit, under a pin that holds it, let main through")
    };
    assert!(
        why.contains(&format!("main at {} holds it", &two[..12])),
        "a hold of mycelium's commit wasn't said held by its pin: {why}"
    );
}

/// A hold of an input's commit its pin doesn't hold yet keeps that commit
/// off the machine, not main: every merge of main's still lands while the
/// input waits for its fix.
#[test]
fn a_hold_on_an_input_ahead_of_its_pin_stops_no_update() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let ahead = world.commit_in("mycelium", "ahead", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    held::ask(&world.config, &format!("hold {ahead}"), Some(1000));
    let one = world.signed("one");
    world.system(&one);
    assert_eq!(
        world.update().1,
        done(&one),
        "a hold of a commit mycelium's pin doesn't hold stopped main"
    );
}

/// A hold of one of main's own commits is judged in main, whatever an
/// input's repository has been made to hold: anyone who can push there,
/// signed or not, could otherwise put the commit in it and have main never
/// asked.
#[test]
fn a_hold_on_main_s_commit_pushed_into_an_input_s_repository_still_stops_main() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    world.system(&format!("{one}+{wanted}"));
    assert_eq!(
        world.update_at(NESTED, &wanted),
        done_following(&one, &[(NESTED, &wanted)]),
        "the ask at mycelium wasn't built"
    );
    held::ask(&world.config, &format!("hold {one}"), Some(1000));
    let mycelium = world.root.join("mycelium");
    let origin = world.origin.display().to_string();
    git(&mycelium, &["fetch", "--quiet", &origin, "main"]);
    let tree = format!("{wanted}^{{tree}}");
    let parents = ["-p", &wanted, "-p", &one];
    let pushed = git(
        &mycelium,
        &[&["commit-tree", &tree][..], &parents, &["-m", "pushed"]].concat(),
    );
    git(&mycelium, &["update-ref", "refs/heads/main", &pushed]);
    let two = world.signed("two");
    world.system(&format!("{two}+{wanted}"));
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a held commit of main's, pushed into mycelium's repository, let main through")
    };
    assert!(
        why.contains(&format!("main at {} holds it", &two[..12])),
        "a held commit of main's wasn't judged in main: {why}"
    );
}

/// A hold of an input's commit is judged in that input's repository even
/// once main's copy has the commit too: getting it into main's objects,
/// as a hosted copy's borrowed checkout or a refused head can, mustn't
/// get it past the hold either.
#[test]
fn a_hold_on_an_input_s_commit_main_s_copy_also_has_still_stops_main() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let later = world.commit_in("mycelium", "later", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    world.system(&format!("{one}+{wanted}"));
    assert_eq!(
        world.update_at(NESTED, &wanted),
        done_following(&one, &[(NESTED, &wanted)]),
        "the ask at mycelium wasn't built"
    );
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &later),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let two = world.signed("two");
    world.system(&two);
    held::ask(&world.config, &format!("hold {wanted}"), Some(1000));
    let mycelium = world.root.join("mycelium").display().to_string();
    git(
        &world.config.repository_copy(),
        &["fetch", "--quiet", &mycelium, "main:refs/heads/side"],
    );
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a held commit of mycelium's, also in main's copy, let main through")
    };
    assert!(
        why.contains(&format!("main at {} holds it", &two[..12])),
        "a held commit of mycelium's in main's copy wasn't judged in mycelium: {why}"
    );
}

/// A hold of what an ask waited for lets that ask go, rather than stopping
/// every update of main's: the ask never ran, and the hold keeps it off.
#[test]
fn a_hold_on_what_an_ask_waited_for_lets_the_ask_go_and_main_through() {
    let mut world = asking_nested(World::new());
    hold_the_daemon(&mut world, "old");
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    daemon_unit(&world.system(&one), "old");
    daemon_unit(&world.system(&format!("{one}+{wanted}")), "new");
    let Line::Refused { why, .. } = world.update_at(NESTED, &wanted) else {
        panic!("an ask restarting a held daemon didn't wait for a hold")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    held::ask(&world.config, &format!("hold {wanted}"), Some(1000));
    assert_eq!(
        world.update().1,
        done(&one),
        "a hold of what an ask only waited for stopped main"
    );
}

/// An ask back at the pin, after one that waited for a hold, calls that
/// wait off: what then runs is the later ask's, so the next update mustn't
/// build the wait again.
#[test]
fn an_ask_back_at_the_pin_calls_off_one_that_waited_for_a_hold() {
    let mut world = asking_nested(World::new());
    hold_the_daemon(&mut world, "old");
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    let ran = world.system(&one);
    daemon_unit(&ran, "old");
    assert_eq!(world.update().1, done(&one), "main wasn't built");
    daemon_unit(&world.system(&format!("{one}+{wanted}")), "new");
    let Line::Refused { why, .. } = world.update_at(NESTED, &wanted) else {
        panic!("an ask restarting a held daemon didn't wait for a hold")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    let at_pin = world.root.join("systems").join(format!("{one}+{pinned}"));
    std::os::unix::fs::symlink(&ran, &at_pin).unwrap();
    let back = world.update_at(NESTED, &pinned);
    assert!(matches!(back, Line::Unchanged { .. }), "{back:?}");
    let next = world.update().1;
    assert!(
        matches!(next, Line::Unchanged { .. }),
        "the update after an ask back at the pin built the wait it called off: {next:?}"
    );
}

/// An ask that only waited for a hold never ran, so a later ask may step
/// behind it: only what the machine followed is a floor.
#[test]
fn an_ask_may_step_behind_one_that_only_waited_for_a_hold() {
    let mut world = asking_nested(World::new());
    hold_the_daemon(&mut world, "old");
    let pinned = world.commit_in("mycelium", "pinned", true);
    let earlier = world.commit_in("mycelium", "earlier", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    daemon_unit(&world.system(&format!("{one}+{wanted}")), "new");
    let Line::Refused { why, .. } = world.update_at(NESTED, &wanted) else {
        panic!("an ask restarting a held daemon didn't wait for a hold")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    daemon_unit(&world.system(&format!("{one}+{earlier}")), "new");
    let Line::Refused { why, .. } = world.update_at(NESTED, &earlier) else {
        panic!("an ask behind one that waited wasn't built to wait in its place")
    };
    assert!(
        why.starts_with(update::NEEDS_HOLD),
        "an ask behind one that only waited for a hold was refused: {why}"
    );
}

/// A kept ask's lock dropping the input, and its repository going with it,
/// fails no update: the lock is read before anything is fetched.
#[test]
fn a_kept_ask_whose_lock_and_repository_drop_it_fails_no_update() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    world.system(&format!("{one}+{wanted}"));
    assert_eq!(
        world.update_at(NESTED, &wanted),
        done_following(&one, &[(NESTED, &wanted)]),
        "the ask at mycelium wasn't built"
    );
    fs::write(world.origin.join("flake.lock"), pinning(&core)).unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    fs::remove_dir_all(world.root.join("mycelium")).unwrap();
    let two = world.signed("two");
    world.system(&two);
    assert_eq!(
        world.update().1,
        done(&two),
        "a kept ask whose lock and repository dropped it failed the update"
    );
}

/// A hold is judged only in what the build reads: an input its lock
/// dropped, whose repository went with it, is neither asked nor fetched,
/// so a hold of its commit stops no update.
#[test]
fn a_hold_on_an_input_the_lock_dropped_stops_no_update() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let wanted = world.commit_in("mycelium", "wanted", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    world.system(&format!("{one}+{wanted}"));
    assert_eq!(
        world.update_at(NESTED, &wanted),
        done_following(&one, &[(NESTED, &wanted)]),
        "the ask at mycelium wasn't built"
    );
    held::ask(&world.config, &format!("hold {wanted}"), Some(1000));
    let gone = world.commit_in("mycelium", "gone", true);
    held::ask(&world.config, &format!("hold {gone}"), Some(1000));
    fs::write(world.origin.join("flake.lock"), pinning(&core)).unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    fs::remove_dir_all(world.root.join("mycelium")).unwrap();
    let two = world.signed("two");
    world.system(&two);
    assert_eq!(
        world.update().1,
        done(&two),
        "a hold of a commit of an input the lock dropped stopped main"
    );
}

/// A hold of a commit an input's copy hasn't fetched yet is still its
/// repository's: the copy is fetched before the hold is judged, so a pin
/// that moved past it is seen to hold it.
#[test]
fn a_hold_on_a_commit_a_stale_copy_lacks_is_judged_in_its_own_repository() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    world.system(&one);
    assert_eq!(world.update().1, done(&one), "main wasn't built");
    let later = world.commit_in("mycelium", "later", true);
    held::ask(&world.config, &format!("hold {later}"), Some(1000));
    let after = world.commit_in("mycelium", "after", true);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &after),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let two = world.signed("two");
    world.system(&two);
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a hold of a commit mycelium's copy hadn't fetched let main through")
    };
    assert!(
        why.contains(&format!("main at {} holds it", &two[..12])),
        "a hold of a commit the copy lacked wasn't said held by the pin past it: {why}"
    );
}

/// A held commit of an input's that another copy has been made to hold
/// still has the input's stale copy fetched: what one copy holds never
/// settles whether another repository builds it.
#[test]
fn a_held_commit_planted_in_main_s_copy_still_fetches_the_stale_input() {
    let world = asking_nested(World::new());
    let pinned = world.commit_in("mycelium", "pinned", true);
    let core = "c".repeat(40);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &pinned),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let one = world.signed("one");
    world.system(&one);
    assert_eq!(world.update().1, done(&one), "main wasn't built");
    let later = world.commit_in("mycelium", "later", true);
    held::ask(&world.config, &format!("hold {later}"), Some(1000));
    let mycelium = world.root.join("mycelium").display().to_string();
    git(
        &world.config.repository_copy(),
        &["fetch", "--quiet", &mycelium, "main:refs/heads/planted"],
    );
    let after = world.commit_in("mycelium", "after", true);
    fs::write(
        world.origin.join("flake.lock"),
        pinning_nested(&core, &after),
    )
    .unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    let two = world.signed("two");
    world.system(&two);
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a held commit planted in main's copy kept mycelium's stale copy unfetched")
    };
    assert!(
        why.contains(&format!("main at {} holds it", &two[..12])),
        "a held commit planted in main's copy wasn't judged in mycelium: {why}"
    );
}

/// Makes a branch, side, off mycelium's main with two signed commits, and
/// gives them oldest first, with main checked out again, so a copy that
/// fetches mycelium's main never has them.
fn mycelium_aside(world: &World) -> (String, String) {
    let mycelium = world.root.join("mycelium");
    git(&mycelium, &["checkout", "--quiet", "-b", "side"]);
    let first = world.commit_in("mycelium", "aside", true);
    let second = world.commit_in("mycelium", "pinned aside", true);
    git(&mycelium, &["checkout", "--quiet", "main"]);
    (first, second)
}

/// Commits a signed main whose lock pins core's mycelium at `pin`.
fn main_pinning_mycelium(world: &World, pin: &str) -> String {
    let lock = pinning_nested(&"c".repeat(40), pin);
    fs::write(world.origin.join("flake.lock"), lock).unwrap();
    git(&world.origin, &["add", "flake.lock"]);
    world.signed("one")
}

/// A lock may pin an input at a commit off its main, as one taken from a
/// branch is, which fetching main never brings. The copy fetches that
/// commit by its id, so a hold the input doesn't build stops no update,
/// where it would otherwise stop every one while the hold stood.
#[test]
fn a_lock_pinning_a_commit_off_its_main_is_fetched_by_sha_to_judge_a_hold() {
    let world = asking_nested(World::new());
    world.commit_in("mycelium", "on main", true);
    let (_, aside) = mycelium_aside(&world);
    let one = main_pinning_mycelium(&world, &aside);
    world.system(&one);
    let elsewhere = "e".repeat(40);
    held::ask(&world.config, &format!("hold {elsewhere}"), Some(1000));
    assert_eq!(
        world.update().1,
        done(&one),
        "a hold of a commit nothing builds stopped main, its input pinned off its main"
    );
}

/// A commit a lock pins off its input's main is fetched with its history,
/// so a hold of a commit before it, on that branch alone, stops main as
/// held rather than as one that couldn't be told.
#[test]
fn a_hold_on_what_a_pin_off_its_main_builds_stops_main_as_held() {
    let world = asking_nested(World::new());
    world.commit_in("mycelium", "on main", true);
    let (before, aside) = mycelium_aside(&world);
    let one = main_pinning_mycelium(&world, &aside);
    world.system(&one);
    held::ask(&world.config, &format!("hold {before}"), Some(1000));
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a hold of a commit a pin off mycelium's main builds let main through")
    };
    let said = format!(
        "{} is held, and main at {} holds it",
        &before[..12],
        &one[..12]
    );
    assert!(
        why.starts_with(&said),
        "a hold of a commit a pin off mycelium's main builds wasn't said held: {why}"
    );
}

/// A pin no fetch can bring, as one whose commit has left its repository,
/// leaves a hold untold, saying why the fetch failed: a hold is never let
/// through on a fetch that failed.
#[test]
fn a_pin_its_repository_no_longer_has_leaves_a_hold_untold() {
    let world = asking_nested(World::new());
    world.commit_in("mycelium", "on main", true);
    let (_, gone) = mycelium_aside(&world);
    let mycelium = world.root.join("mycelium");
    git(&mycelium, &["branch", "--quiet", "-D", "side"]);
    git(&mycelium, &["reflog", "expire", "--expire=now", "--all"]);
    git(&mycelium, &["gc", "--quiet", "--prune=now"]);
    let one = main_pinning_mycelium(&world, &gone);
    world.system(&one);
    let elsewhere = "e".repeat(40);
    held::ask(&world.config, &format!("hold {elsewhere}"), Some(1000));
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("a hold was let through beside a pin no fetch could bring")
    };
    assert!(
        why.contains("couldn't be told"),
        "a hold beside a pin no fetch could bring was refused for another reason: {why}"
    );
    assert!(
        why.contains(&format!("fetching {} for", &gone[..12])),
        "a hold left untold didn't say why the fetch of its pin failed: {why}"
    );
}

#[test]
fn an_input_asked_for_is_held_to_the_lock_of_the_core_built_with_it() {
    let world = asking_nested(following());
    let older = world.commit_in("mycelium", "older", true);
    let middle = world.commit_in("mycelium", "middle", true);
    let later = world.commit_in("mycelium", "later", true);
    let core_at_older = world.core_pinning("pins older", &older);
    let core_at_later = world.core_pinning("pins later", &later);
    let head = world.signed_pinning("one", &core_at_older);
    // main's own lock doesn't pin core's mycelium, so only core's lock at
    // the core asked for can refuse the middle.
    let at = |core: &str| {
        BTreeMap::from([
            ("core".to_owned(), core.to_owned()),
            (NESTED.to_owned(), middle.clone()),
        ])
    };
    let built = world.builds();
    let ask = |at: &BTreeMap<String, String>| {
        update::update_at(&world.config, &|| false, at, &mut |_| {})
    };
    let Line::Refused { why, .. } = ask(&at(&core_at_later)) else {
        panic!("mycelium behind what core's lock pins wasn't refused")
    };
    assert!(
        why.contains(&format!(
            "which core's lock at {} pins",
            &core_at_later[..12]
        )),
        "refused for another reason: {why}"
    );
    assert_eq!(world.builds(), built, "a refused ask built something");
    world.system(&format!("{head}+{middle}"));
    assert_eq!(
        ask(&at(&core_at_older)),
        done_following(&head, &[("core", &core_at_older), (NESTED, &middle)]),
        "an ask at both inputs, mycelium past core's pin, didn't build both"
    );
    let overridden = world.read("overridden");
    for (input, commit) in [("core", &core_at_older), (NESTED, &middle)] {
        assert!(
            overridden
                .lines()
                .any(|line| line.starts_with(&format!("{input} git+file://"))
                    && line.ends_with(&format!("rev={commit}"))),
            "the build wasn't given {input} at the commit asked for: {overridden}"
        );
    }
}

#[test]
fn building_elsewhere_sends_only_the_heavy_pieces_away() {
    let mut world = World::new();
    world.config.elsewhere = true;
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    assert!(away.contains("/nix/store/b-heavy.drv^*"), "{away}");
    assert!(!away.contains("a-light"), "{away}");
    // What's left is built here, with no builders to send it to.
    let last = world.read("last-build");
    assert!(last.contains("--builders  "), "{last}");
    assert_eq!(world.builds(), 1);
}

#[test]
fn building_elsewhere_in_batches_sends_a_batch_away_at_a_time_then_builds_the_rest_so() {
    let mut world = World::new();
    world.config.elsewhere = true;
    in_batches(&mut world, 1, 600_000);
    fs::write(world.root.join("lighter"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    let sent: Vec<&str> = away.lines().collect();
    assert_eq!(
        sent.len(),
        3,
        "three heavy pieces in batches of one: {away}"
    );
    for (line, piece) in sent.iter().zip(["b-heavy", "c-wrapper", "t-secret-tool"]) {
        assert_eq!(line.matches(".drv^*").count(), 1, "{away}");
        assert!(
            line.contains(&format!("/nix/store/{piece}.drv^*")),
            "{away}"
        );
    }
    let batched = world.read("batched");
    let built: Vec<&str> = batched.lines().collect();
    assert_eq!(
        built.len(),
        2,
        "two light pieces in batches of one: {batched}"
    );
    for (line, piece) in built.iter().zip(["a-light", "a-lighter"]) {
        assert!(
            line.contains(&format!("/nix/store/{piece}.drv^*")),
            "{batched}"
        );
        assert!(
            line.contains("--builders  "),
            "a light piece left a builder: {batched}"
        );
    }
    assert_eq!(
        world.builds(),
        1,
        "the system is built once the batches are"
    );
}

#[test]
fn a_heavy_piece_missing_again_before_the_build_here_goes_to_the_builders() {
    let mut world = World::new();
    world.config.elsewhere = true;
    let head = world.signed("one");
    world.system(&head);
    fs::write(world.root.join("collected"), "").unwrap();
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    assert!(
        away.contains("/nix/store/g-collected.drv^*"),
        "a piece missing again by the build here wasn't sent to the builders: {away}"
    );
    assert!(
        away.contains("/nix/store/b-heavy.drv^*"),
        "the first listing's heavy piece wasn't sent: {away}"
    );
    assert_eq!(
        away.matches("b-heavy").count(),
        1,
        "a piece the first listing sent was sent again: {away}"
    );
    assert_eq!(world.builds(), 1, "the system wasn't built once");
}

#[test]
fn a_heavy_piece_missing_again_that_no_builder_builds_fails_the_ask() {
    let mut world = World::new();
    world.config.elsewhere = true;
    let head = world.signed("one");
    world.system(&head);
    fs::write(world.root.join("collected"), "").unwrap();
    fs::write(world.root.join("collected-fails"), "").unwrap();
    let Line::Failed { why, .. } = world.update().1 else {
        panic!("an ask whose heavy piece no builder built didn't fail")
    };
    assert!(
        why.contains("on the machine's builders"),
        "the failure didn't say the builders failed: {why}"
    );
    assert_eq!(world.builds(), 0, "the system was built here regardless");
}

#[test]
fn what_the_system_needs_is_kept_from_being_collected_while_it_builds() {
    let mut world = World::new();
    world.config.elsewhere = true;
    let head = world.signed("one");
    world.system(&head);
    let evaluated = world.config.state.join("evaluated");
    fs::create_dir_all(&evaluated).unwrap();
    fs::write(evaluated.join("drv-9"), "").unwrap();
    // The secret piece's output is in the store and its doc isn't.
    let secret = world.root.join("secret-out");
    fs::write(&secret, "").unwrap();
    assert_eq!(world.update().1, done(&head));
    let kept = world.read("kept");
    let mut links = kept.lines();
    assert_eq!(
        links
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>(),
        [
            "--extra-experimental-features",
            "nix-command",
            "flakes",
            "build",
            "--out-link",
            &evaluated.join("drv").display().to_string(),
            "/nix/store/a-light.drv",
            "/nix/store/b-heavy.drv",
            "/nix/store/c-wrapper.drv",
            "/nix/store/t-secret-tool.drv",
        ],
        "each piece the dry run listed wasn't linked to be kept"
    );
    assert_eq!(
        links
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>(),
        [
            "--extra-experimental-features",
            "nix-command",
            "flakes",
            "build",
            "--out-link",
            &evaluated.join("built").display().to_string(),
            &secret.display().to_string(),
        ],
        "what a needed piece is built from, built already, wasn't kept: {kept}"
    );
    assert!(
        !evaluated.join("drv-9").exists(),
        "what the last build kept was kept still"
    );
    assert_eq!(world.builds(), 1);
}

#[test]
fn building_elsewhere_with_no_builder_builds_nothing_here() {
    let mut world = World::new();
    world.config.elsewhere = true;
    let head = world.signed("one");
    world.system(&head);
    fs::write(world.root.join("no-builder"), "").unwrap();
    let Line::Failed { why, lines, .. } = world.update().1 else {
        panic!("not failed")
    };
    assert!(why.contains("on the machine's builders"), "{why}");
    assert_eq!(lines, ["local builds are disabled"]);
    assert_eq!(world.builds(), 0);
}

/// Builds here in batches of `pieces`, once 1,000 MiB is free, with
/// `free_kib` of memory available and as much swap free.
fn in_batches(world: &mut World, pieces: usize, free_kib: u64) {
    world.config.batches = Some(Batches {
        pieces,
        room_mib: 1000,
        within: 0,
    });
    fs::write(
        &world.config.memory,
        format!("MemTotal: 8000000 kB\nMemAvailable: {free_kib} kB\nSwapFree: {free_kib} kB\n"),
    )
    .unwrap();
}

#[test]
fn a_system_of_more_pieces_than_a_batch_builds_a_batch_at_a_time() {
    let mut world = World::new();
    in_batches(&mut world, 3, 600_000);
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(
        world.update().1,
        done(&head),
        "with room, the batches and the system build"
    );
    let batched = world.read("batched");
    let builds: Vec<&str> = batched.lines().collect();
    assert_eq!(
        builds.len(),
        2,
        "four pieces in batches of three: {batched}"
    );
    assert!(
        builds[0].contains("/nix/store/a-light.drv^*")
            && builds[0].contains("/nix/store/c-wrapper.drv^*")
            && !builds[0].contains("t-secret-tool"),
        "the first batch takes the first three by name: {batched}"
    );
    assert!(
        builds[1].contains("/nix/store/t-secret-tool.drv^*"),
        "the second takes the last: {batched}"
    );
    assert_eq!(
        world.builds(),
        1,
        "the system is built once the batches are"
    );
}

#[test]
fn a_system_a_batch_holds_builds_in_one() {
    let mut world = World::new();
    in_batches(&mut world, 4, 600_000);
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(
        world.update().1,
        done(&head),
        "with room, the system builds"
    );
    assert_eq!(world.read("batched"), "", "no batch is built apart");
    assert_eq!(world.builds(), 1, "the system's own build builds it");
}

#[test]
fn a_build_with_no_room_for_it_never_starts() {
    let mut world = World::new();
    in_batches(&mut world, 3, 400_000);
    let head = world.signed("one");
    world.system(&head);
    let Line::Failed { why, .. } = world.update().1 else {
        panic!("an update with too little memory didn't fail")
    };
    assert!(
        why.contains("only 781 MiB of memory and swap was free here, not the 1000"),
        "{why}"
    );
    assert_eq!(world.read("batched"), "", "no batch started");
    assert_eq!(world.builds(), 0, "nor the system's build");
}

#[test]
fn a_kept_piece_goes_only_to_its_own_builders_and_what_is_built_from_one_stays() {
    let mut world = World::new();
    world.config.elsewhere = true;
    world.config.kept = Kept {
        names: vec!["secret*".to_owned()],
        builders: Some("@/etc/kept-builders".to_owned()),
    };
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    let sent: Vec<&str> = away.lines().collect();
    assert_eq!(sent.len(), 2, "{away}");
    // The secret tool goes first and alone to the kept builders, and the
    // open piece to the daemon's. The wrapper, built from a secret piece,
    // goes to neither, and is built here with no builders at all.
    assert!(sent[0].contains("--builders @/etc/kept-builders"), "{away}");
    assert!(sent[0].contains("/nix/store/t-secret-tool.drv^*"), "{away}");
    assert!(!sent[0].contains("b-heavy"), "{away}");
    assert!(!sent[1].contains("--builders"), "{away}");
    assert!(sent[1].contains("/nix/store/b-heavy.drv^*"), "{away}");
    assert!(!away.contains("c-wrapper"), "{away}");
    let last = world.read("last-build");
    assert!(last.contains("--builders  "), "{last}");
    assert_eq!(world.builds(), 1);
}

#[test]
fn a_kept_piece_with_no_builders_of_its_own_builds_nothing() {
    let mut world = World::new();
    world.config.elsewhere = true;
    world.config.kept = Kept {
        names: vec!["secret*".to_owned()],
        builders: None,
    };
    let head = world.signed("one");
    world.system(&head);
    let Line::Failed { why, lines, .. } = world.update().1 else {
        panic!("not failed")
    };
    assert!(why.contains("builders of their own"), "{why}");
    assert_eq!(lines, ["/nix/store/t-secret-tool.drv"]);
    // Not even the open piece was sent anywhere.
    assert_eq!(world.read("away"), "");
    assert_eq!(world.builds(), 0);
}

/// Builds elsewhere, letting what its builders don't build be built here,
/// small, while CPU pressure is under 15%, and says it is `waited`.
fn elsewhere_with_here(world: &mut World, waited: &str) {
    world.config.elsewhere = true;
    world.config.here = Some(Here {
        max_jobs: 1,
        cores: 4,
        busy_at: 15.0,
    });
    if !waited.is_empty() {
        fs::write(
            &world.config.cpu_pressure,
            format!(
                "some avg10=90.00 avg60={waited} avg300=1.00 total=1\n\
                 full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n"
            ),
        )
        .unwrap();
    }
}

/// Keeps the secret pieces to builders of their own, which may be built
/// here, small, while CPU pressure is under 15%, and says it is `waited`.
fn kept_with_here(world: &mut World, waited: &str) {
    elsewhere_with_here(world, waited);
    world.config.kept = Kept {
        names: vec!["secret*".to_owned()],
        builders: Some("@/etc/kept-builders".to_owned()),
    };
}

#[test]
fn building_elsewhere_goes_to_the_builders_named() {
    let mut world = World::new();
    world.config.elsewhere = true;
    world.config.builders = Some("@/etc/builders-for-anything".to_owned());
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    assert!(
        away.contains("--max-jobs 0 --builders @/etc/builders-for-anything"),
        "{away}"
    );
    assert!(away.contains("/nix/store/b-heavy.drv^*"), "{away}");
}

#[test]
fn a_piece_the_builders_dont_take_is_built_here_small() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    fs::write(world.root.join("no-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let here = world.read("here");
    assert!(here.contains("/nix/store/b-heavy.drv^*"), "{here}");
    // The daemon's builders stay, for what only they take.
    assert!(!here.contains("--builders"), "{here}");
    assert!(here.contains("--max-jobs 1 --cores 4"), "{here}");
}

#[test]
fn a_big_piece_is_left_for_a_builder_rather_than_built_here() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    fs::write(world.root.join("no-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    fs::write(world.root.join("big"), "").unwrap();
    let Line::Failed { why, .. } = world.update().1 else {
        panic!("not failed")
    };
    // A piece asking for big-parallel, as the kernel does, goes to the
    // builders first and alone, so nothing here builds it as another's
    // input.
    assert!(
        why.contains("building the big pieces on the machine's builders"),
        "{why}"
    );
    assert!(why.contains("which never build here"), "{why}");
    assert_eq!(world.read("here"), "");
    assert_eq!(world.builds(), 0);
}

#[test]
fn the_small_pieces_are_built_here_once_the_builders_build_the_big() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    fs::write(world.root.join("no-small-builder"), "").unwrap();
    fs::write(world.root.join("big"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    assert!(away.contains("/nix/store/e-big.drv^*"), "{away}");
    assert!(!away.contains("b-heavy"), "{away}");
    let here = world.read("here");
    assert!(here.contains("/nix/store/b-heavy.drv^*"), "{here}");
}

#[test]
fn the_small_pieces_build_beside_the_big_and_what_is_built_from_it_after() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    fs::write(world.root.join("big"), "").unwrap();
    // The big piece builds only once the heavy one is sent beside it.
    fs::write(world.root.join("big-waits"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    let sent: Vec<&str> = away.lines().collect();
    assert_eq!(sent.len(), 3, "{away}");
    let big = sent
        .iter()
        .position(|s| s.contains("e-big"))
        .expect("the big piece sent");
    assert!(!sent[big].contains("b-heavy"), "{away}");
    let heavy = sent
        .iter()
        .position(|s| s.contains("b-heavy"))
        .expect("the heavy piece sent");
    assert!(!sent[heavy].contains("d-modules"), "{away}");
    // The modules, built from the big piece, go only once it's built.
    let modules = sent
        .iter()
        .position(|s| s.contains("d-modules"))
        .expect("the modules sent");
    assert!(modules > big, "{away}");
    assert_eq!(world.read("here"), "");
}

#[test]
fn a_big_piece_that_fails_ends_what_builds_beside_it() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    // Every build but the big one hangs, and nothing is stopped for
    // saying nothing before the test would end.
    world.config.silent_for = 60;
    fs::write(world.root.join("hung"), "").unwrap();
    fs::write(world.root.join("big"), "").unwrap();
    fs::write(world.root.join("big-fails"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let started = std::time::Instant::now();
    let Line::Failed { why, .. } = world.update().1 else {
        panic!("not failed")
    };
    assert!(why.contains("building the big pieces"), "{why}");
    // The hung build beside it was ended rather than waited on for its
    // 30 seconds.
    let took = started.elapsed();
    assert!(took < std::time::Duration::from_secs(20), "{took:?}");
    assert_eq!(world.read("away"), "");
    assert_eq!(world.read("here"), "");
}

#[test]
fn a_build_on_a_builder_that_goes_quiet_is_stopped_and_built_here() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    world.config.silent_for = 1;
    // The builder takes the build and never answers again, as a lent
    // builder whose tunnel kept resetting did on 8 November.
    fs::write(world.root.join("hung"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let started = std::time::Instant::now();
    assert_eq!(world.update().1, done(&head));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "it waited for the builder"
    );
    let here = world.read("here");
    assert!(here.contains("/nix/store/b-heavy.drv^*"), "{here}");
}

#[test]
fn a_build_that_loses_its_builder_once_is_tried_there_again_not_built_here() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    fs::write(world.root.join("drops-once"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let away = world.read("away");
    assert!(
        away.contains("/nix/store/b-heavy.drv^*"),
        "the second try didn't go to the builders: {away}"
    );
    assert_eq!(
        world.read("here"),
        "",
        "a builder lost once sent the pieces here"
    );
}

#[test]
fn a_piece_built_here_keeps_the_builders_named() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "14.99");
    world.config.builders = Some("@/etc/builders-for-anything".to_owned());
    fs::write(world.root.join("no-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let here = world.read("here");
    assert!(
        here.contains("--builders @/etc/builders-for-anything --max-jobs 1 --cores 4"),
        "{here}"
    );
}

#[test]
fn a_piece_the_builders_dont_take_waits_rather_than_build_here_under_load() {
    let mut world = World::new();
    // A busy last ten seconds counts for nothing; the minute does.
    elsewhere_with_here(&mut world, "15.00");
    fs::write(world.root.join("no-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let Line::Failed { why, lines, .. } = world.update().1 else {
        panic!("not failed")
    };
    assert!(why.contains("on the machine's builders"), "{why}");
    assert!(
        why.contains("CPU pressure here is 15.0%, not below 15%, so they wait"),
        "{why}"
    );
    assert_eq!(lines, ["local builds are disabled"]);
    assert_eq!(world.read("here"), "");
    assert_eq!(world.builds(), 0);
}

#[test]
fn a_machine_long_behind_main_builds_what_is_left_here_under_load() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "50.00");
    fs::write(world.root.join("no-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    // An earlier update began building a main the machine still doesn't run.
    fs::create_dir_all(&world.config.state).unwrap();
    let behind = world.config.behind_file();
    fs::File::create(&behind)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_mins(21))
        .unwrap();
    assert_eq!(world.update().1, done(&head));
    let here = world.read("here");
    assert!(here.contains("--max-jobs 1 --cores 4"), "{here}");
    // The machine runs main now, so the next one starts its own wait.
    assert!(!behind.exists());
}

#[test]
fn a_machine_just_behind_main_still_waits_under_load() {
    let mut world = World::new();
    elsewhere_with_here(&mut world, "50.00");
    fs::write(world.root.join("no-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let Line::Failed { why, .. } = world.update().1 else {
        panic!("not failed")
    };
    assert!(why.contains("so they wait"), "{why}");
    // Its wait is marked from this build.
    assert!(world.config.behind_file().exists());
    assert_eq!(world.read("here"), "");
}

#[test]
fn a_kept_piece_its_builders_dont_build_is_built_here_small() {
    let mut world = World::new();
    kept_with_here(&mut world, "14.99");
    fs::write(world.root.join("no-kept-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    let here = world.read("here");
    assert!(
        here.contains("--builders  --max-jobs 1 --cores 4"),
        "{here}"
    );
    assert!(here.contains("/nix/store/t-secret-tool.drv^*"), "{here}");
    assert!(!here.contains("b-heavy"), "{here}");
    // The open piece still goes to the daemon's builders.
    let away = world.read("away");
    assert!(away.contains("/nix/store/b-heavy.drv^*"), "{away}");
    assert!(!away.contains("secret"), "{away}");
}

#[test]
fn a_kept_piece_its_builders_build_is_not_built_here() {
    let mut world = World::new();
    kept_with_here(&mut world, "14.99");
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert_eq!(world.read("here"), "");
    assert!(
        world
            .read("away")
            .contains("--builders @/etc/kept-builders")
    );
}

#[test]
fn a_kept_piece_waits_rather_than_build_here_under_load() {
    let mut world = World::new();
    // Where CPU pressure can't be read, as on a Mac, nothing builds here.
    kept_with_here(&mut world, "");
    fs::write(world.root.join("no-kept-builder"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let Line::Failed { why, lines, .. } = world.update().1 else {
        panic!("not failed")
    };
    assert!(
        why.contains("CPU pressure here can't be read, so they wait"),
        "{why}"
    );
    assert_eq!(lines, ["no kept builder answers"]);
    assert_eq!(world.read("here"), "");
    assert_eq!(world.builds(), 0);
}

#[test]
fn a_kept_piece_with_no_builders_named_is_built_here_when_it_may_be() {
    let mut world = World::new();
    kept_with_here(&mut world, "14.99");
    world.config.kept.builders = None;
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert!(world.read("here").contains("t-secret-tool"));
    assert!(!world.read("away").contains("secret"));
}

/// Refuses while the world's `call` file is there, and can't tell when
/// `cant-tell` is.
fn refuse_during_calls(world: &mut World) {
    world.config.refuse_while = vec![Busy {
        command: vec![
            shell(),
            "-c".to_owned(),
            format!(
                "test -e {}/cant-tell && {{ echo looking >&2; echo 'the calendar was not read' >&2; \
                 exit 3; }}; test -e {}/call",
                world.root.display(),
                world.root.display()
            ),
        ],
        why: "a call is on".to_owned(),
        follower_waits: false,
        follower_waits_at_most: None,
        every_ask_waits: false,
        refuses_the_ask_alone: false,
        refuses_at_most: None,
        says: Says::Beside,
    }];
}

/// The machine is over its load while the world's `call` file is there,
/// which a build makes while `call-starts` is: refusing the ask alone, and
/// for `at_most` seconds of asks if given.
fn over_its_load(world: &mut World, at_most: Option<u64>) {
    world.config.refuse_while = vec![Busy {
        command: vec![
            shell(),
            "-c".to_owned(),
            format!("test -e {}/call", world.root.display()),
        ],
        why: "the machine is over its load".to_owned(),
        follower_waits: false,
        follower_waits_at_most: None,
        every_ask_waits: false,
        refuses_the_ask_alone: true,
        refuses_at_most: at_most,
        says: Says::Beside,
    }];
}

/// A load a build raises is its own, so a busy check that refuses the ask
/// alone holds no switch of what was built: the five-minute load stays
/// high for minutes after a build.
#[test]
fn a_check_refusing_the_ask_alone_holds_no_switch() {
    let mut world = World::new();
    over_its_load(&mut world, None);
    fs::write(world.root.join("call-starts"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(
        world.asked_by(&|| true).1,
        done(&head),
        "a load the build raised shouldn't hold its own switch"
    );
    let next = world.signed("two");
    world.system(&next);
    let Line::Refused { why, .. } = world.asked_by(&|| true).1 else {
        panic!("an ask while over its load should be refused")
    };
    assert_eq!(why, "the machine is over its load");
}

/// A refusal carries what its check last said, as a Mac short of memory
/// says which readings it judged, so its log shows why without a rerun.
#[test]
fn a_refusal_carries_what_its_check_said() {
    let mut world = World::new();
    over_its_load(&mut world, None);
    world.config.refuse_while[0].says = Says::Why;
    world.config.refuse_while[0].command[2] = format!(
        "echo 'mac-room: 30% then 20% free' >&2; test -e {}/call",
        world.root.display()
    );
    fs::write(world.root.join("call"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let Line::Refused { why, .. } = world.asked_by(&|| true).1 else {
        panic!("an ask while over its load should be refused")
    };
    assert_eq!(
        why,
        "the machine is over its load, saying mac-room: 30% then 20% free"
    );
}

/// A check that doesn't say why it's busy keeps its words out of the
/// refusal, as the call check's caution that a peer went unheard is no
/// reason a call is on.
#[test]
fn a_refusal_leaves_out_what_a_check_said_beside() {
    let mut world = World::new();
    over_its_load(&mut world, None);
    world.config.refuse_while[0].command[2] = format!(
        "echo 'caution: a peer went unheard' >&2; test -e {}/call",
        world.root.display()
    );
    fs::write(world.root.join("call"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let Line::Refused { why, .. } = world.asked_by(&|| true).1 else {
        panic!("an ask while over its load should be refused")
    };
    assert_eq!(why, "the machine is over its load");
}

/// A person's ask builds though the machine is over its load, since the
/// build is theirs to choose, and may be the one that mends the check.
#[test]
fn a_person_s_ask_builds_though_the_machine_is_over_its_load() {
    let mut world = World::new();
    over_its_load(&mut world, None);
    fs::write(world.root.join("call"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    assert!(
        matches!(world.asked_by(&|| true).1, Line::Refused { .. }),
        "the follower's ask over its load should be refused"
    );
    assert_eq!(
        world.update().1,
        done(&head),
        "a person's ask should build though over its load"
    );
}

/// Switching what waited for a grant builds nothing, so a busy check that
/// refuses the ask alone refuses no grant's switch, nor spends the grant.
#[test]
fn a_check_refusing_the_ask_alone_refuses_no_granted_switch() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "old");
    over_its_load(&mut world, None);
    let one = world.signed("one");
    let waited = world.system(&one);
    daemon_unit(&waited, "new");
    let Line::Refused { why, .. } = world.asked_by(&|| true).1 else {
        panic!("a switch restarting a held daemon should wait for a hold")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    // The build's own load is still high as the grant comes.
    fs::write(world.root.join("call"), "").unwrap();
    fs::write(world.root.join("given"), "").unwrap();
    assert_eq!(
        world.asked_by(&|| true).1,
        done(&one),
        "the grant's switch shouldn't be refused by the load its build raised"
    );
    assert_eq!(world.running(), waited);
}

/// A busy check that may refuse asks only so long lets one through once
/// it has refused that long, and refuses again after: one that can never
/// tell would otherwise refuse the update that mends it.
#[test]
fn a_check_refusing_for_at_most_a_while_lets_an_ask_through_after() {
    let mut world = World::new();
    over_its_load(&mut world, Some(HOUR));
    fs::write(world.root.join("call"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let mut first = None;
    for ask in 0..2 {
        let (told, ending) = world.asked_by(&|| true);
        let Line::Refused { why, .. } = ending else {
            panic!("ask {ask} within the most it may refuse should be refused")
        };
        assert_eq!(why, "the machine is over its load");
        assert!(
            !told.contains(&Line::Fetching),
            "a refused ask shouldn't fetch"
        );
        let (began, _) = kept_refusals(&world);
        assert_eq!(
            *first.get_or_insert(began),
            began,
            "a stretch should keep its start across its refusals"
        );
        refused_since(&world, began - 60, now());
        first = Some(began - 60);
    }
    assert_eq!(world.builds(), 0, "a refused ask should build nothing");
    // Refusing since just short of the most it may, it still refuses.
    refused_since(&world, now() - HOUR + 60, now() - 60);
    assert!(
        matches!(world.asked_by(&|| true).1, Line::Refused { .. }),
        "an ask short of the most it may refuse should be refused"
    );
    refused_since(&world, now() - HOUR - 1, now() - 60);
    assert_eq!(
        world.asked_by(&|| true).1,
        done(&head),
        "an ask past the most it may refuse should go ahead"
    );
    assert!(!refusals(&world).exists(), "its count should start again");
    let next = world.signed("two");
    world.system(&next);
    assert!(
        matches!(world.asked_by(&|| true).1, Line::Refused { .. }),
        "the count should start again once an ask went ahead"
    );
    let mut never = World::new();
    over_its_load(&mut never, None);
    fs::write(never.root.join("call"), "").unwrap();
    let head = never.signed("one");
    never.system(&head);
    for ask in 0..2 {
        assert!(
            matches!(never.asked_by(&|| true).1, Line::Refused { .. }),
            "with no most, ask {ask} should be refused"
        );
    }
}

/// A stretch of refusals ends once an ask finds the machine under its load, or once none came
/// for half an hour, as through a call or a sleep, so a refusal hours old
/// lets no new busy spell through at once.
#[test]
fn a_stretch_of_refusals_ends_under_its_load_or_after_a_long_gap() {
    let mut world = World::new();
    over_its_load(&mut world, Some(HOUR));
    fs::write(world.root.join("call"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    refused_since(&world, now() - 2 * HOUR, now() - 60);
    fs::remove_file(world.root.join("call")).unwrap();
    assert_eq!(world.asked_by(&|| true).1, done(&head));
    assert!(
        !refusals(&world).exists(),
        "an ask under its load should end the stretch"
    );
    let next = world.signed("two");
    world.system(&next);
    fs::write(world.root.join("call"), "").unwrap();
    refused_since(&world, now() - 2 * HOUR, now() - 31 * 60);
    assert!(
        matches!(world.asked_by(&|| true).1, Line::Refused { .. }),
        "a refusal over half an hour ago shouldn't count toward this stretch"
    );
    let (began, _) = kept_refusals(&world);
    assert!(now() - began < 60, "the stretch should begin again now");
}

/// A refusal record it can't trust or can't keep never refuses for good:
/// one from the future, as a clock set back leaves, starts again, and one
/// it can't write lets the ask go ahead.
#[test]
fn a_refusal_record_it_cant_trust_or_keep_refuses_nothing_for_good() {
    let mut world = World::new();
    over_its_load(&mut world, Some(HOUR));
    fs::write(world.root.join("call"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    refused_since(&world, now() + 2 * HOUR, now() + 2 * HOUR);
    assert!(
        matches!(world.asked_by(&|| true).1, Line::Refused { .. }),
        "a record from the future should start a stretch that still refuses"
    );
    let (began, last) = kept_refusals(&world);
    assert!(
        began <= now() && last <= now(),
        "a record from the future should start the stretch again"
    );
    // One that ends before it began, as a hand or a torn write leaves,
    // starts again too, rather than counting from a start never refused.
    refused_since(&world, now() + 2 * HOUR, now() - 60);
    assert!(
        matches!(world.asked_by(&|| true).1, Line::Refused { .. }),
        "a record ending before it began should start a stretch that still refuses"
    );
    let (began, _) = kept_refusals(&world);
    assert!(
        began <= now(),
        "a record ending before it began should start the stretch again"
    );
    fs::remove_file(refusals(&world)).unwrap();
    fs::create_dir(refusals(&world)).unwrap();
    assert_eq!(
        world.asked_by(&|| true).1,
        done(&head),
        "an ask whose refusal couldn't be kept should go ahead"
    );
}

const HOUR: u64 = 60 * 60;

/// A Mac's logs carry no time of their own, so the daemon and the
/// follower stamp their lines with the time now, in UTC.
#[test]
fn a_log_stamp_is_the_time_now_in_utc() {
    use std::time::{Duration, UNIX_EPOCH};
    assert_eq!(
        update_to_main::stamp_at(UNIX_EPOCH + Duration::from_secs(86_400 + 3_661)),
        "1970-01-02T01:01:01Z",
        "the stamp's time isn't said in UTC, as RFC 3339 says it"
    );
    assert_eq!(
        update_to_main::stamp_at(UNIX_EPOCH - Duration::from_secs(1)),
        "1970-01-01T00:00:00Z",
        "a clock before 1970 wasn't stamped as 1970's start"
    );
    let before = now();
    let stamp = update_to_main::stamp();
    let after = now();
    assert!(
        (before..=after).any(|at| terminal_kit::time::rfc3339(i64::try_from(at).unwrap()) == stamp),
        "{stamp} isn't the time in UTC between {before} and {after}"
    );
}

/// A stamped line is the time, a space, then the line as it was; only a
/// Mac's lines are stamped, since the journal keeps the time and its
/// readers read a line as it comes.
#[test]
fn a_stamped_line_is_the_time_then_the_line() {
    let line = "{\"step\":\"done\"}";
    assert_eq!(
        update_to_main::stamped_with(Some("1970-01-01T00:00:00Z"), line),
        "1970-01-01T00:00:00Z {\"step\":\"done\"}",
        "a Mac's line wasn't the time in UTC and then the line"
    );
    assert_eq!(
        update_to_main::stamped_with(None, line),
        line,
        "a line with no stamp wasn't left as it was"
    );
    if !cfg!(target_os = "macos") {
        assert_eq!(
            update_to_main::stamped(line),
            line,
            "a line the journal keeps was stamped"
        );
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Where the load check keeps its stretch of refusals.
fn refusals(world: &World) -> PathBuf {
    world
        .config
        .state
        .join("refused-since-the-machine-is-over-its-load")
}

/// Records a stretch of the load check's refusals, from `began` to `last`.
fn refused_since(world: &World, began: u64, last: u64) {
    fs::write(refusals(world), format!("{began} {last}\n")).unwrap();
}

/// When the kept stretch of refusals began, and last refused.
fn kept_refusals(world: &World) -> (u64, u64) {
    let text = fs::read_to_string(refusals(world)).unwrap();
    let mut times = text.split_whitespace().map(|t| t.parse().unwrap());
    (times.next().unwrap(), times.next().unwrap())
}

#[test]
fn a_call_starts_nothing() {
    let mut world = World::new();
    refuse_during_calls(&mut world);
    fs::write(world.root.join("call"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let (told, ending) = world.update();
    assert_eq!(told, [] as [update_to_main::line::Line; 0]);
    let Line::Refused { why, .. } = ending else {
        panic!("not refused: {ending:?}")
    };
    assert_eq!(why, "a call is on");
    assert_eq!(world.builds(), 0);
}

#[test]
fn a_call_starting_during_the_build_holds_the_switch_until_the_next_ask() {
    let mut world = World::new();
    refuse_during_calls(&mut world);
    fs::write(world.root.join("call-starts"), "").unwrap();
    let head = world.signed("one");
    let system = world.system(&head);
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert_eq!(why, "a call is on");
    assert_eq!(world.builds(), 1);
    assert_ne!(fs::read_link(world.root.join("running")).ok(), Some(system));
    // Once the call is over, the next ask switches.
    fs::remove_file(world.root.join("call-starts")).unwrap();
    fs::remove_file(world.root.join("call")).unwrap();
    assert_eq!(world.update().1, done(&head));
}

#[test]
fn a_busy_check_that_cant_tell_counts_as_busy() {
    let mut world = World::new();
    refuse_during_calls(&mut world);
    fs::write(world.root.join("cant-tell"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(
        why.starts_with("it can't tell whether a call is on"),
        "{why}"
    );
    assert!(why.contains("exit status: 3"), "{why}");
    assert_eq!(world.builds(), 0);
}

#[test]
fn a_busy_check_that_cant_tell_says_its_last_word() {
    let mut world = World::new();
    refuse_during_calls(&mut world);
    fs::write(world.root.join("cant-tell"), "").unwrap();
    let head = world.signed("one");
    world.system(&head);
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(
        why.ends_with("exit status: 3, saying the calendar was not read"),
        "the refusal should end with the check's last line on stderr: {why}"
    );
    assert!(
        !why.contains("saying looking"),
        "only the check's last line belongs in the refusal: {why}"
    );
}

/// A follower waits on `why` the first `times` it's asked, and not after,
/// for `at_most` seconds if given, counting each ask in the world's file
/// `counted`.
fn waits_on(world: &World, why: &str, counted: &str, times: usize, at_most: Option<u64>) -> Busy {
    Busy {
        command: vec![
            shell(),
            "-c".to_owned(),
            format!(
                "n=$(cat {0} 2>/dev/null || echo 0); echo $((n + 1)) > {0}; test $n -lt {times}",
                world.root.join(counted).display()
            ),
        ],
        why: why.to_owned(),
        follower_waits: true,
        follower_waits_at_most: at_most,
        every_ask_waits: false,
        refuses_the_ask_alone: false,
        refuses_at_most: None,
        says: Says::Beside,
    }
}

/// Says the launcher is in use the first `times` it's asked, and not
/// after, for every ask, counting each ask in the world's `launcher` file.
fn the_launcher_in_use(world: &World, times: usize, at_most: Option<u64>) -> Busy {
    Busy {
        every_ask_waits: true,
        ..waits_on(world, "the launcher is in use", "launcher", times, at_most)
    }
}

fn launcher_asked(world: &World) -> usize {
    world.read("launcher").trim().parse().unwrap_or(0)
}

/// Says someone is at the machine the first `times` it's asked, and nobody
/// after, counting each ask in the world's `asked` file.
fn someone_at_the_machine(world: &mut World, times: usize) {
    world.config.refuse_while = vec![waits_on(
        world,
        "someone is at the machine",
        "asked",
        times,
        None,
    )];
}

fn asked(world: &World) -> usize {
    world.read("asked").trim().parse().unwrap_or(0)
}

#[test]
fn someone_who_stays_holds_the_switch_only_as_long_as_they_may() {
    let mut world = World::new();
    world.config.refuse_while = vec![
        waits_on(&world, "someone is at the machine", "asked", 8, Some(2)),
        waits_on(&world, "the launcher is open", "launcher", 4, None),
    ];
    let head = world.signed("one");
    world.system(&head);
    let began = Instant::now();
    assert_eq!(world.asked_by(&|| true).1, done(&head));
    assert!(
        began.elapsed() >= Duration::from_secs(2),
        "it switched before someone's two seconds were up"
    );
    assert!(
        asked(&world) < 8,
        "it waited past someone's two seconds until they left"
    );
    let launcher: usize = world.read("launcher").trim().parse().unwrap_or(0);
    assert_eq!(launcher, 5, "it switched while the launcher was open");
}

#[test]
fn a_switch_the_follower_asked_for_waits_while_someone_is_at_the_machine() {
    let mut world = World::new();
    someone_at_the_machine(&mut world, 2);
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.asked_by(&|| true).1, done(&head));
    assert_eq!(
        asked(&world),
        3,
        "it switched before nobody was at the machine"
    );
    assert_eq!(world.builds(), 1);
}

#[test]
fn a_persons_ask_waits_for_nobody_and_ends_the_followers_wait() {
    let mut world = World::new();
    someone_at_the_machine(&mut world, 1000);
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
    assert_eq!(
        asked(&world),
        0,
        "a person's ask asked who was at the machine"
    );
    // A person joins the follower's update while its switch waits.
    let head = world.signed("two");
    world.system(&head);
    let looks = std::cell::Cell::new(0);
    let following = || {
        looks.set(looks.get() + 1);
        looks.get() < 3
    };
    assert_eq!(world.asked_by(&following).1, done(&head));
    assert_eq!(asked(&world), 2);
}

#[test]
fn a_persons_ask_waits_while_the_launcher_is_in_use_and_switches_once_it_isnt() {
    let mut world = World::new();
    world.config.refuse_while = vec![
        waits_on(&world, "someone is at the machine", "asked", 1000, None),
        the_launcher_in_use(&world, 2, None),
    ];
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(
        world.update().1,
        done(&head),
        "a launcher in use refused a person's ask rather than holding its switch"
    );
    assert_eq!(
        launcher_asked(&world),
        3,
        "a person's ask switched while the launcher was in use"
    );
    assert_eq!(
        asked(&world),
        0,
        "a person's ask waited on what only the follower waits on"
    );
    assert_eq!(world.builds(), 1, "it didn't build once, ahead of the wait");
}

#[test]
fn a_persons_ask_waits_on_the_launcher_only_as_long_as_it_may() {
    let mut world = World::new();
    world.config.refuse_while = vec![the_launcher_in_use(&world, 1000, Some(1))];
    let head = world.signed("one");
    world.system(&head);
    let began = Instant::now();
    assert_eq!(
        world.update().1,
        done(&head),
        "a launcher in use past its second refused the switch"
    );
    assert!(
        began.elapsed() >= Duration::from_secs(1),
        "it switched before the launcher's second was up"
    );
    assert!(
        launcher_asked(&world) < 1000,
        "it waited past the launcher's second until the launcher was free"
    );
}

#[test]
fn a_machine_not_busy_updates() {
    let mut world = World::new();
    refuse_during_calls(&mut world);
    let head = world.signed("one");
    world.system(&head);
    assert_eq!(world.update().1, done(&head));
}

/// Holds the Nix daemon, granted by the world's `given` file, and gives
/// each system the daemon's unit with what it says.
fn hold_the_daemon(world: &mut World, first: &str) {
    world.config.holds = Some(Holds {
        units: vec!["nix-daemon.service".to_owned()],
        paths: vec!["kernel".to_owned()],
        account_units: BTreeMap::from([("joel".to_owned(), vec!["windows.service".to_owned()])]),
        given: world.root.join("given"),
    });
    daemon_unit(&world.system("first"), first);
}

/// Gives a system a home-manager generation for the account `joel`, as
/// NixOS's `home-manager-joel.service` names it, whose home has the unit
/// `windows.service` saying `says`.
fn account_unit(system: &Path, says: &str) {
    let generation = system.join("home-manager-generation");
    let units = generation.join("home-files/.config/systemd/user");
    fs::create_dir_all(&units).unwrap();
    fs::write(units.join("windows.service"), says).unwrap();
    let system_units = system.join("etc/systemd/system");
    fs::create_dir_all(&system_units).unwrap();
    fs::write(
        system_units.join("home-manager-joel.service"),
        format!(
            "[Service]\nExecStart=/hm-setup-env {}\n",
            generation.display()
        ),
    )
    .unwrap();
}

#[test]
fn a_switch_that_would_restart_a_held_account_unit_waits_for_a_hold() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    account_unit(&world.system("first"), "old");
    let head = world.signed("one");
    let system = world.system(&head);
    daemon_unit(&system, "same");
    account_unit(&system, "new");
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    assert!(why.contains("joel's windows.service"), "{why}");
    fs::write(world.root.join("given"), "").unwrap();
    assert_eq!(world.update().1, done(&head));
}

#[test]
fn an_account_unit_home_manager_keeps_running_needs_no_hold() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    account_unit(&world.system("first"), "old");
    let head = world.signed("one");
    let system = world.system(&head);
    daemon_unit(&system, "same");
    account_unit(&system, "[Unit]\nX-RestartIfChanged=false\n");
    assert_eq!(world.update().1, done(&head));
}

/// sd-switch keeps a changed unit whose new file says keep-old running as
/// it was, as the sessions' kitty is, so changing it closes no window and
/// needs no grant (2026-11-21). The new unit, its drop-ins with it, alone
/// decides, as sd-switch reads it, so one gaining keep-old is kept at the
/// switch it gains it in.
#[test]
fn an_account_unit_a_switch_keeps_needs_no_hold() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    account_unit(&world.system("first"), "[Service]\nExecStart=old\n");
    let head = world.signed("one");
    let system = world.system(&head);
    daemon_unit(&system, "same");
    account_unit(
        &system,
        "[Unit]\nX-SwitchMethod = keep-old\n\n[Service]\nExecStart=new\n",
    );
    assert_eq!(world.update().1, done(&head));
}

/// Gives a system's account unit a drop-in, `name`, saying `says`.
fn account_drop_in(system: &Path, name: &str, says: impl AsRef<[u8]>) {
    let drop_ins = system.join("home-manager-generation/home-files/.config/systemd/user");
    let drop_ins = drop_ins.join("windows.service.d");
    fs::create_dir_all(&drop_ins).unwrap();
    fs::write(drop_ins.join(name), says).unwrap();
}

/// Asserts the follow was refused for the hold on joel's windows.service.
fn held_for_windows(world: &World, case: &str) {
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("{case}: not refused")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{case}: {why}");
    assert!(why.contains("joel's windows.service"), "{case}: {why}");
}

/// One new account unit a switch is judged by: its file, or none, and its
/// drop-ins by name.
struct NewUnit {
    case: &'static str,
    file: Option<&'static str>,
    drop_ins: &'static [(&'static str, &'static str)],
}

const KEPT: &str = "[Unit]\nX-SwitchMethod=keep-old\n";
const RESTARTED: &str = "[Unit]\nX-SwitchMethod=restart\n";

/// Switches from a kept unit to `new`, giving how the follow ended.
fn switch_to(new: &NewUnit) -> (World, String) {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    account_unit(&world.system("first"), KEPT);
    let head = world.signed(new.case);
    let system = world.system(&head);
    daemon_unit(&system, "same");
    account_unit(&system, new.file.unwrap_or(""));
    if new.file.is_none() {
        fs::remove_file(
            system.join("home-manager-generation/home-files/.config/systemd/user/windows.service"),
        )
        .unwrap();
    }
    for (name, says) in new.drop_ins {
        account_drop_in(&system, name, says);
    }
    (world, head)
}

/// What doesn't keep the unit as sd-switch reads it is held: keep-old
/// dropped, as a revert drops it; said outside [Unit]; overridden by a
/// drop-in later by name; said only in a file systemd doesn't read as a
/// drop-in; a legacy keep under a method that restarts, or beside a legacy
/// reload; a unit refusing a start, which sd-switch stops; or the unit
/// gone from the new system.
#[test]
fn an_account_unit_a_switch_doesnt_keep_waits_for_a_hold() {
    let new = "[Service]\nExecStart=new\n";
    let cases = [
        NewUnit {
            case: "dropped",
            file: Some(new),
            drop_ins: &[],
        },
        NewUnit {
            case: "outside",
            file: Some("[Service]\nX-SwitchMethod=keep-old\nExecStart=new\n"),
            drop_ins: &[],
        },
        NewUnit {
            case: "overridden",
            file: Some(KEPT),
            drop_ins: &[("10-kept.conf", KEPT), ("20-later.conf", RESTARTED)],
        },
        NewUnit {
            case: "not-a-drop-in",
            file: Some(new),
            drop_ins: &[("kept.txt", KEPT)],
        },
        NewUnit {
            case: "method-over-legacy",
            file: Some("[Unit]\nX-RestartIfChanged=false\nX-SwitchMethod=restart\n"),
            drop_ins: &[],
        },
        NewUnit {
            case: "reloads",
            file: Some("[Unit]\nX-ReloadIfChanged=true\nX-RestartIfChanged=false\n"),
            drop_ins: &[],
        },
        NewUnit {
            case: "refuses-a-start",
            file: Some("[Unit]\nX-SwitchMethod=keep-old\nRefuseManualStart=yes\n"),
            drop_ins: &[],
        },
        NewUnit {
            case: "gone",
            file: None,
            drop_ins: &[],
        },
    ];
    for new in &cases {
        held_for_windows(&switch_to(new).0, new.case);
    }
}

/// A drop-in that won't read as text could say any method, and sd-switch
/// fails on it, so a unit with one is held though its file keeps.
#[test]
fn an_account_unit_with_an_unread_drop_in_waits_for_a_hold() {
    let unread = NewUnit {
        case: "unread",
        file: Some(KEPT),
        drop_ins: &[],
    };
    let (world, head) = switch_to(&unread);
    account_drop_in(&world.system(&head), "later.conf", [0xff, 0xfe, b'\n']);
    held_for_windows(&world, unread.case);
}

/// What keeps it as sd-switch reads it needs no hold: a drop-in adding
/// keep-old, the last by name winning over an earlier one, and the legacy
/// key in any of the words systemd reads as false.
#[test]
fn an_account_unit_its_drop_ins_keep_needs_no_hold() {
    let new = "[Service]\nExecStart=new\n";
    let cases = [
        NewUnit {
            case: "added",
            file: Some(new),
            drop_ins: &[("kept.conf", KEPT)],
        },
        NewUnit {
            case: "last-wins",
            file: Some(RESTARTED),
            drop_ins: &[("10-restarted.conf", RESTARTED), ("20-kept.conf", KEPT)],
        },
        NewUnit {
            case: "legacy-no",
            file: Some("[Unit]\nX-RestartIfChanged = No\n"),
            drop_ins: &[],
        },
    ];
    for new in &cases {
        let (world, head) = switch_to(new);
        assert_eq!(world.update().1, done(&head), "{}", new.case);
    }
}

fn daemon_unit(system: &Path, says: &str) {
    let units = system.join("etc/systemd/system");
    fs::create_dir_all(&units).unwrap();
    fs::write(units.join("nix-daemon.service"), says).unwrap();
}

#[test]
fn a_switch_that_would_restart_a_held_daemon_waits_for_a_hold() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "old");
    let head = world.signed("one");
    let system = world.system(&head);
    daemon_unit(&system, "new");
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    assert!(why.contains("nix-daemon.service"), "{why}");
    assert_ne!(world.running(), system);
    // Granted, the next ask switches, and takes the grant with it.
    fs::write(world.root.join("given"), "").unwrap();
    assert_eq!(world.update().1, done(&head));
    assert_eq!(world.running(), system);
    assert!(!world.root.join("given").exists());
}

#[test]
fn the_grant_switches_what_waited_for_it_though_main_moved_on() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "old");
    let one = world.signed("one");
    let waited = world.system(&one);
    daemon_unit(&waited, "new");
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "a held daemon's restart should be refused without a grant"
    );
    // Main moves on while the hold is arranged; the grant's ask switches
    // to what was built, rather than start over at the new head.
    let two = world.signed("two");
    daemon_unit(&world.system(&two), "newer");
    let builds = world.builds();
    fs::write(world.root.join("given"), "").unwrap();
    assert_eq!(world.update().1, done(&one));
    assert_eq!(world.running(), waited);
    assert_eq!(world.builds(), builds, "it built nothing again");
    // The next ask takes the newer head as ever.
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "the next ask should take the newer head, refused without a grant"
    );
}

#[test]
fn a_grant_whose_system_left_the_store_builds_again() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "old");
    let one = world.signed("one");
    let system = world.system(&one);
    daemon_unit(&system, "new");
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "a switch restarting a held daemon didn't wait for a hold"
    );
    // Collected while the hold was arranged, the system that waited is
    // built again rather than switched to where it no longer is.
    let waiting = world.config.state.join("waiting.json");
    let mut kept: serde_json::Value =
        serde_json::from_str(&world.read("state/waiting.json")).unwrap();
    kept["system"] = world.root.join("collected").display().to_string().into();
    fs::write(&waiting, kept.to_string()).unwrap();
    let builds = world.builds();
    fs::write(world.root.join("given"), "").unwrap();
    assert_eq!(world.update().1, done(&one));
    assert_eq!(world.running(), system);
    assert_eq!(world.builds(), builds + 1, "it didn't build again");
}

#[test]
fn a_system_that_waited_for_a_hold_tells_its_boot_check_its_flake_and_overrides() {
    // The grant's switch builds nothing, so the flake and overrides are
    // worked out again from the commit and inputs that waited.
    let mut world = following();
    hold_the_daemon(&mut world, "old");
    let pinned = world.core_commit("pinned", true);
    let newer = world.core_commit("newer", true);
    let head = world.signed_pinning("one", &pinned);
    let told = world.root.join("told");
    let check = format!(
        "printf '%s' \"$UPDATE_TO_MAIN_FLAKE\" > {told}; \
         printf '%s' \"$UPDATE_TO_MAIN_OVERRIDES\" > {told}.overrides",
        told = told.display()
    );
    let system = a_new_initrd(&mut world, &head, &check);
    daemon_unit(&system, "new");
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "a switch restarting a held daemon didn't wait for a hold"
    );
    fs::write(world.root.join("given"), "").unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(ending, Line::Done { boots: None, .. }),
        "the granted switch should pass its boot check: {ending:?}"
    );
    let flake = fs::read_to_string(&told).unwrap_or_default();
    assert!(
        flake.ends_with(&format!("rev={head}")) && !flake.contains('#'),
        "the granted switch's boot check wasn't told its flake: {flake}"
    );
    let overrides = fs::read_to_string(told.with_extension("overrides")).unwrap_or_default();
    let overrides: Vec<&str> = overrides.lines().collect();
    assert!(
        overrides.len() == 3 && overrides[2].ends_with(&format!("rev={newer}")),
        "the granted switch's boot check wasn't told the inputs it followed: {overrides:?}"
    );
}

#[test]
fn a_hosted_grant_whose_system_boots_the_same_unpacks_nothing_again() {
    // Only a boot check reads the flake, and it runs only for a system
    // that boots differently, so a grant of one that doesn't fetches
    // nothing while the person who granted it waits.
    let mut world = World::hosted_from(World::new(), counted_token);
    hold_the_daemon(&mut world, "old");
    let head = world.hosted_commit("one", None);
    fs::create_dir_all(world.root.join("booted")).unwrap();
    let told = world.root.join("told");
    let check = format!("printf ran > {}", told.display());
    world.config.boot_check = Some(vec![shell(), "-c".to_owned(), check]);
    daemon_unit(&world.system(&head), "new");
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "a switch restarting a held daemon didn't wait for a hold"
    );
    let _ = fs::remove_dir_all(world.config.source());
    fs::write(world.root.join("given"), "").unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(ending, Line::Done { .. }),
        "the granted switch should end done: {ending:?}"
    );
    assert!(!told.exists(), "a system booting the same was boot-checked");
    assert!(
        !world.config.source().exists(),
        "the grant unpacked main again, though no boot check reads it"
    );
}

#[test]
fn a_hosted_system_that_waited_tells_its_boot_check_a_folder_holding_its_commit() {
    // A hosted flake is a folder each unpack wipes, so the folder the
    // system was built from may be gone by the grant.
    let mut world = World::hosted_from(World::new(), counted_token);
    hold_the_daemon(&mut world, "old");
    let head = world.hosted_commit("one", None);
    let told = world.root.join("told");
    let check = format!(
        "printf '%s' \"$(< \"${{UPDATE_TO_MAIN_FLAKE#path:}}/file\")\" > {}",
        told.display()
    );
    let system = a_new_initrd(&mut world, &head, &check);
    daemon_unit(&system, "new");
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "a switch restarting a held daemon didn't wait for a hold"
    );
    let _ = fs::remove_dir_all(world.config.source());
    fs::write(world.root.join("given"), "").unwrap();
    let (_, ending) = world.update();
    assert!(
        matches!(ending, Line::Done { boots: None, .. }),
        "the granted switch should pass its boot check: {ending:?}"
    );
    assert_eq!(
        fs::read_to_string(&told).unwrap_or_default(),
        "one",
        "the granted switch's boot check wasn't told a folder holding its commit"
    );
}

#[test]
fn a_held_daemons_drop_in_changing_waits_for_a_hold_too() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    let head = world.signed("one");
    let system = world.system(&head);
    daemon_unit(&system, "same");
    let drop_ins = system.join("etc/systemd/system/nix-daemon.service.d");
    fs::create_dir_all(&drop_ins).unwrap();
    fs::write(drop_ins.join("overrides.conf"), "[Service]\n").unwrap();
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "a held daemon's new drop-in should be refused without a grant"
    );
}

#[test]
fn a_switch_leaving_held_daemons_as_they_are_needs_no_hold() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    let head = world.signed("one");
    daemon_unit(&world.system(&head), "same");
    assert_eq!(world.update().1, done(&head));
}

#[test]
fn a_grant_ends_with_the_ask_it_was_made_for_even_when_refused_as_busy() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "old");
    refuse_during_calls(&mut world);
    let head = world.signed("one");
    let system = world.system(&head);
    daemon_unit(&system, "new");
    fs::write(world.root.join("given"), "").unwrap();
    fs::write(world.root.join("call"), "").unwrap();
    assert!(
        matches!(world.update().1, Line::Refused { .. }),
        "an ask during a call should be refused though it was granted"
    );
    assert!(!world.root.join("given").exists());
    // Once the call is over, the next ask needs a grant of its own.
    fs::remove_file(world.root.join("call")).unwrap();
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
}

#[test]
fn a_grant_ends_with_an_ask_that_held_nothing() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    let head = world.signed("one");
    daemon_unit(&world.system(&head), "same");
    fs::write(world.root.join("given"), "").unwrap();
    assert_eq!(world.update().1, done(&head));
    assert!(!world.root.join("given").exists());
}

#[test]
fn heads_that_build_the_running_system_switch_nothing() {
    let world = World::new();
    let head = world.signed("one");
    let first = world.system("first");
    std::os::unix::fs::symlink(&first, world.root.join("systems").join(&head)).unwrap();
    assert_eq!(
        world.update().1,
        Line::Unchanged {
            commit: head.clone(),
            followed: BTreeMap::new(),
            boots: None,
        }
    );
    assert_eq!(world.builds(), 1);
    // Remembered, so the next ask at the same heads builds nothing.
    assert!(matches!(world.update().1, Line::Unchanged { .. }));
    assert_eq!(world.builds(), 1);
}

#[test]
fn a_new_kernel_waits_for_a_hold_when_the_kernel_is_held() {
    let mut world = World::new();
    hold_the_daemon(&mut world, "same");
    let head = world.signed("one");
    let system = a_new_kernel(&world, &head);
    daemon_unit(&system, "same");
    let Line::Refused { why, .. } = world.update().1 else {
        panic!("not refused")
    };
    assert!(why.starts_with(update::NEEDS_HOLD), "{why}");
    assert!(why.contains("kernel"), "{why}");
    assert_ne!(world.running(), system);
}
