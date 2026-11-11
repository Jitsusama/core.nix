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
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use update_to_main::config::{
    Batches, Busy, Config, Follow, Here, Holds, Hosted, Kept, Kind, Session, Signers, Tools,
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

        let sh = shell();
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
        // corrupted tunnel did on 11 November. A batch built here is
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
                 *' --dry-run '*) printf 'these 4 derivations will be built:\\n  \
                 /nix/store/a-light.drv\\n  /nix/store/b-heavy.drv\\n  \
                 /nix/store/c-wrapper.drv\\n  /nix/store/t-secret-tool.drv\\n' >&2; \
                 if [ -e {big} ]; then printf '  /nix/store/e-big.drv\\n  \
                 /nix/store/d-modules.drv\\n' >&2; fi; exit 0;;\n\
                 *' derivation show /nix/store/s-secret.drv '*) echo '{{\"derivations\": \
                 {{\"s-secret.drv\": {{\"outputs\": {{\"out\": {{\"path\": \"{secret_out}\"}}, \
                 \"doc\": {{\"path\": \"{secret_out}-doc\"}}}}}}}}}}'; exit 0;;\n\
                 *' derivation show '*) secret=\n\
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
                 '\"$secret$big\"'}}}}'; exit 0;;\n\
                 *'/evaluated/drv '*|*'/evaluated/built '*) echo \"$*\" >> {kept}; exit 0;;\n\
                 *' --cores '*) echo \"$*\" >> {here}; exit 0;;\n\
                 *' build --no-link -L '*) echo \"$*\" >> {batched}; exit 0;;\n\
                 *' --builders @/etc/kept-builders '*) if [ -e {no_kept_builder} ]; then \
                 echo 'no kept builder answers' >&2; exit 1; fi; echo \"$*\" >> {away}; exit 0;;\n\
                 *' --max-jobs 0 '*) case \" $* \" in *e-big*)\n\
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
        let config = Config {
            repository: format!("file://{}", origin.display()),
            branch: "main".to_owned(),
            output: "system".to_owned(),
            kind: Kind::Nixos,
            state: root.join("state"),
            socket: root.join("ask.sock"),
            group: own_group(),
            ended: root.join("last.json"),
            running: root.join("now.json"),
            profile: root.join("profile"),
            signers: Signers {
                ssh: Some(signers),
                openpgp: Vec::new(),
            },
            hosted: None,
            follows: BTreeMap::new(),
            elsewhere: false,
            builders: None,
            kept: Kept::default(),
            here: None,
            health: vec![vec![
                sh.clone(),
                "-c".to_owned(),
                format!("test ! -e {}/unhealthy", root.join("running").display()),
            ]],
            health_within: 1,
            battery_floor: 20,
            refuse_while: Vec::new(),
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
            },
            session: None,
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
            changed: None,
            api: None,
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
        self.asked_by(&|| false)
    }

    /// Updates as the follower asks, `following` saying whether only it
    /// has asked still.
    fn asked_by(&self, following: &dyn Fn() -> bool) -> (Vec<Line>, Line) {
        let mut told = Vec::new();
        let ending = update::update(&self.config, following, &mut |l| told.push(l.clone()));
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
            followed: BTreeMap::new()
        }
    );
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
            followed: BTreeMap::new(),
        }
    );
    assert_eq!(world.running(), system);
    assert_eq!(world.booting(), system);
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
    // The build waits for a gate, so the stop comes while it runs.
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
    let nothing = ask::stop(&socket).unwrap();
    assert!(
        matches!(&nothing, Line::Refused { why, .. } if why.contains("nothing was stopped")),
        "{nothing:?}"
    );
    let asking = {
        let socket = socket.clone();
        thread::spawn(move || ask::ask(&socket, &mut |_| {}).unwrap())
    };
    while !fs::read_to_string(&world.config.running).is_ok_and(|line| line.contains("building")) {
        thread::sleep(std::time::Duration::from_millis(20));
    }
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
            followed: BTreeMap::new()
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
            followed: BTreeMap::new()
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
        },
    );
    world
}

impl World {
    /// Commits to core's main, signed by the trusted key or not.
    fn core_commit(&self, message: &str, signed: bool) -> String {
        let core = self.root.join("core");
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
        followed: BTreeMap::from([("core".to_owned(), core.to_owned())]),
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
                "test -e {}/cant-tell && exit 3; test -e {}/call",
                world.root.display(),
                world.root.display()
            ),
        ],
        why: "a call is on".to_owned(),
        follower_waits: false,
        follower_waits_at_most: None,
    }];
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
    }
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
    assert!(matches!(world.update().1, Line::Refused { .. }));
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
    assert!(matches!(world.update().1, Line::Refused { .. }));
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
    assert!(matches!(world.update().1, Line::Refused { .. }));
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
    assert!(matches!(world.update().1, Line::Refused { .. }));
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
