//! `nix-offload there [-L] [--keep-going] [--apply FUNCTION] [--out DIR]
//! [--here] FLAKE ATTR...`: builds a flake's attributes on the lent
//! builder by sending it a command, or here with `--here` when none takes
//! it.
//!
//! The build hook builds a build's inputs here and copies each up, and a
//! VM test took 1025 s that way against 60 s here, nearly all of it the
//! upload. So a flake the builder can fetch itself, such as a pushed
//! commit of a public repository, goes as one command: the builder
//! fetches it and every input, evaluates and builds, and only the command
//! goes up while the log comes down. Evaluation leaves this machine too.
//! A flake that's a git tree here goes the same way at its commit when
//! GitHub has it, clean. Otherwise its commits since the newest one GitHub
//! has go up as a git bundle, a few kilobytes, with any change to its
//! tracked files as one more commit made without touching its branch or
//! index, and the builder fetches the rest from GitHub.
//!
//! With no builder standing, it asks the lender for one and waits, since
//! one stands in about 70 s, while a VM test here holds the person's
//! machine for minutes. With no lender running, none being made, or none
//! reached, it builds nothing and says so, unless `--here` asks it to
//! build here.
//!
//! `--apply` builds a Nix function applied to the one attribute, and
//! `--out` copies its output back into a folder, as a VM test's pictures.
//! `there --takes` says which of these it takes, a word a line.
//!
//! An attribute for a system a machine nearby takes, named by its second
//! word as in `checks.aarch64-darwin.x`, goes to that machine instead
//! while it answers, since this one can't build it at all. A flake that's
//! a folder here, which no machine could fetch, goes with it as a
//! tarball, and any input its lock pins to a store path is copied too, so
//! the nearby machine fetches the rest itself and can read inputs only it
//! has.
//!
//! A caller may rewrite any `nix build --no-link` into this without ever
//! failing for the routing: with no builder, or none ssh can reach, the
//! same build runs here, saying so. It exits with the build's own status,
//! or the first that failed, and journals where each part ran and what it
//! sent, in `there.jsonl` beside the lender's journal.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What a `there` was asked to build, and how.
#[derive(Debug, PartialEq, Eq)]
pub struct Asked {
    flake: String,
    attributes: Vec<String>,
    /// The flags passed through to `nix build`: `-L` and `--keep-going`.
    flags: Vec<String>,
    /// A Nix function applied to the one attribute before it's built, as
    /// `--apply` gives it.
    apply: Option<String>,
    /// Where the one attribute's output is copied back to, as plain files,
    /// as `--out` gives it.
    out: Option<PathBuf>,
    /// Whether it may build here when no builder takes it, as `--here`
    /// says.
    here: bool,
}

/// Reads `there`'s arguments: its flags, in any place, then the flake and
/// at least one attribute. `--apply FUNCTION` and `--out DIR` take one
/// attribute only. Anything else is refused, since only a build whose
/// every word is known can be sent whole.
pub fn asked(arguments: &[String]) -> Option<Asked> {
    let mut flags = Vec::new();
    let mut words = Vec::new();
    let (mut apply, mut out, mut here) = (None, None, false);
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-L" | "--print-build-logs" | "--keep-going" => {
                if !flags.contains(argument) {
                    flags.push(argument.clone());
                }
            }
            "--apply" => apply = Some(arguments.next()?.clone()),
            "--out" => out = Some(PathBuf::from(arguments.next()?)),
            "--here" => here = true,
            flag if flag.starts_with('-') => return None,
            word => words.push(word.to_owned()),
        }
    }
    let (flake, attributes) = words.split_first()?;
    let one = attributes.len() == 1;
    (!attributes.is_empty() && (one || (apply.is_none() && out.is_none()))).then(|| Asked {
        flake: flake.clone(),
        attributes: attributes.to_vec(),
        flags,
        apply,
        out,
        here,
    })
}

impl Asked {
    /// The same build of other attributes, or of the same flake named
    /// otherwise.
    fn with(&self, flake: &str, attributes: Vec<String>) -> Self {
        Self {
            flake: flake.to_owned(),
            attributes,
            flags: self.flags.clone(),
            apply: self.apply.clone(),
            out: self.out.clone(),
            here: self.here,
        }
    }

    /// `nix build`'s arguments for this, wherever it runs.
    fn build(&self) -> Vec<String> {
        self.build_of(&self.flake, false)
    }

    /// `nix build`'s arguments for this from `flake`. An applied function
    /// is built as an expression, which a flake that isn't locked, as one
    /// here is, needs `--impure` to read.
    fn build_of(&self, flake: &str, locked: bool) -> Vec<String> {
        let mut words: Vec<String> = ["build", "--no-link"]
            .iter()
            .map(|word| (*word).to_owned())
            .collect();
        if self.out.is_some() {
            words.push("--print-out-paths".to_owned());
        }
        words.extend(self.flags.iter().cloned());
        match &self.apply {
            Some(function) => {
                if !locked {
                    words.push("--impure".to_owned());
                }
                words.push("--expr".to_owned());
                words.extend(self.attributes.iter().map(|attribute| {
                    format!(
                        "({function}) (builtins.getFlake {}).{attribute}",
                        nix_string(flake)
                    )
                }));
            }
            None => words.extend(
                self.attributes
                    .iter()
                    .map(|attribute| format!("{flake}#{attribute}")),
            ),
        }
        words
    }

    /// The command the builder's shell is given.
    fn command(&self) -> String {
        self.command_of(&self.flake)
    }

    /// The command a builder's shell is given to build this from `flake`,
    /// which there is always locked. With `--out`, the output comes back
    /// as a tarball on its standard output, the log staying on its error.
    fn command_of(&self, flake: &str) -> String {
        self.command_in(flake, true)
    }

    /// The same, from a `flake` that's locked or, as a folder unpacked
    /// there is, not.
    fn command_in(&self, flake: &str, locked: bool) -> String {
        let build = std::iter::once("nix".to_owned())
            .chain(self.build_of(flake, locked).iter().map(|word| quoted(word)))
            .collect::<Vec<_>>()
            .join(" ");
        if self.out.is_some() {
            format!("out=$({build}) && tar -czf - -C \"$out\" .")
        } else {
            build
        }
    }
}

/// A string as Nix reads one, quoted.
fn nix_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', r"\\")
            .replace('"', "\\\"")
            .replace("${", "\\${")
    )
}

/// The lent builder, as nix-offload names its store: where ssh reaches it,
/// the key it takes and its host key.
#[derive(Debug, PartialEq, Eq)]
pub struct Builder {
    /// The host, as ssh is given it, with its user where the store names one.
    pub host: String,
    /// The key the builder takes, where the store names one.
    pub key: Option<String>,
    /// The builder's host key, as a line of a known hosts file has it.
    pub host_key: Option<String>,
}

/// The builder an ssh store's URI names, as Nix writes one:
/// `ssh-ng://host?ssh-key=...&base64-ssh-public-host-key=...`.
pub fn builder(store: &str) -> Option<Builder> {
    let rest = store
        .strip_prefix("ssh-ng://")
        .or_else(|| store.strip_prefix("ssh://"))?;
    let (host, settings) = rest.split_once('?').unwrap_or((rest, ""));
    if host.is_empty() {
        return None;
    }
    let setting = |name: &str| {
        settings
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_owned())
    };
    Some(Builder {
        host: host.to_owned(),
        key: setting("ssh-key"),
        host_key: setting("base64-ssh-public-host-key").and_then(|key| decoded(&key)),
    })
}

/// ssh's own status when it couldn't reach the builder or lost it, which
/// no build gives.
pub const UNREACHED: i32 = 255;

/// Where a build ran and how it ended.
#[derive(Debug, PartialEq, Eq)]
pub struct Ran {
    /// The builder's host, or none when it ran here.
    pub on: Option<String>,
    /// The build's status.
    pub status: i32,
    /// What ssh sent the builder, as its own count says.
    pub sent: Option<u64>,
}

/// Builds what was asked on the builder `store` names (the store file's
/// first line), or here when there's none or ssh can't reach it. `scratch`
/// holds, for the length of the build, the builder's host key as ssh
/// reads it and ssh's own log; `ssh` and `nix` are the programs run.
/// Builds `asked` on the lent builder `store` names, or here without one.
/// A builder lost mid-build, as a switch's takeback restarts its tunnel,
/// is asked `again` for, a store's text or none, and the build is sent
/// once more: built here, a VM test held the person's machine for minutes
/// on 30 October while the builder stood again within 20 s.
pub fn run(
    asked: &Asked,
    store: Option<&str>,
    again: &dyn Fn() -> Option<String>,
    scratch: &Path,
    [ssh, nix]: [&str; 2],
) -> Ran {
    let lent = store.and_then(|store| builder(store.lines().next()?.trim()));
    let Some(mut lent) = lent else {
        return unsent(asked, nix, "no builder is lent");
    };
    let reached = match folder_of(&asked.flake) {
        None => Ok(Sending {
            command: asked.command(),
            bundle: None,
        }),
        // A `path:` flake is read as its folder holds it, git or not.
        Some(folder) if asked.flake.starts_with("path:") => packed(&folder, asked, scratch, true),
        Some(folder) => source(&folder, scratch)
            .map(|source| source.sending(asked))
            .or_else(|why| {
                eprintln!("nix-offload: {why}, so it goes as a tarball");
                packed(&folder, asked, scratch, false)
            }),
    };
    let reached = reached.and_then(|sending| {
        let ran = loop {
            let ran = on(&lent, &sending, asked.out.as_deref(), scratch, ssh);
            if !matches!(&ran, Ok(ran) if ran.status == UNREACHED) {
                break ran;
            }
            eprintln!(
                "nix-offload: ssh lost {}, so this waits for a builder to stand again",
                lent.host
            );
            match again().and_then(|store| builder(store.lines().next()?.trim())) {
                Some(stood) => lent = stood,
                None => break ran,
            }
        };
        if let Some(bundle) = &sending.bundle {
            let _ = fs::remove_file(bundle);
        }
        ran
    });
    match reached {
        Ok(ran) if ran.status != UNREACHED => ran,
        Ok(_) => unsent(asked, nix, &format!("ssh couldn't reach {}", lent.host)),
        Err(error) => unsent(asked, nix, &error),
    }
}

/// What `there` exits with when no builder took a build and it wasn't
/// asked to build here: sysexits.h's temporary failure, as `--changed`
/// gives when nowhere took its run.
pub const UNSENT: i32 = 75;

/// A build no builder took, `why`: built here only when asked to with
/// `--here`. Built here unasked after midnight on 30 October, with no
/// builder lent, every session's builds ran beside the person's work at a
/// load of 145.
fn unsent(asked: &Asked, nix: &str, why: &str) -> Ran {
    if asked.here {
        eprintln!("nix-offload: {why}, so this builds here");
        return here(asked, nix);
    }
    eprintln!(
        "nix-offload: {why}, so nothing was built; ask again once a builder stands, \
         or give --here to build on this machine"
    );
    Ran {
        on: None,
        status: UNSENT,
        sent: None,
    }
}

fn here(asked: &Asked, nix: &str) -> Ran {
    let mut command = Command::new(nix);
    command.args(asked.build());
    let status = match &asked.out {
        None => command
            .status()
            .map_or(1, |status| status.code().unwrap_or(1)),
        // Its log still shows: output() would keep standard error too.
        Some(out) => command
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .map_or(1, |built| {
                let path = String::from_utf8_lossy(&built.stdout).trim().to_owned();
                match built.status.code() {
                    Some(0) => copied(Path::new(&path), out),
                    code => code.unwrap_or(1),
                }
            }),
    };
    Ran {
        on: None,
        status,
        sent: None,
    }
}

/// Copies a build's output into `out` as plain files the account owns,
/// giving 0 once it has.
fn copied(built: &Path, out: &Path) -> i32 {
    let copied = fs::create_dir_all(out).is_ok()
        && Command::new("cp")
            .args(["-rL", "--no-preserve=mode,ownership"])
            .arg(built.join("."))
            .arg(out)
            .status()
            .is_ok_and(|status| status.success());
    if copied {
        0
    } else {
        eprintln!(
            "nix-offload: couldn't copy {} to {}",
            built.display(),
            out.display()
        );
        1
    }
}

/// The files of a folder in a git tree, NUL-separated, that `path:` reads
/// and its git doesn't ignore.
const UNIGNORED: &[&str] = &[
    "ls-files",
    "-z",
    "--cached",
    "--others",
    "--exclude-standard",
];

/// A flake that's a folder here, sent to the builder as a tarball and
/// built there as a `path:` flake, its inputs fetched there: `whole`, the
/// folder as `path:` reads it, but its git history, any result links and
/// what its git tree ignores, and otherwise the files its git tree
/// tracks, as Nix reads a git tree's flake. It costs the folder's size,
/// 4.7 MB for a checkout of this repository, so a commit GitHub has is
/// sent by name instead, and its size is said before it goes. Ignored
/// files stay here even where `path:` would read them: a Rust tree's
/// target folder held 793 MB on 30 October, which no build reads and
/// whose upload took longer than the build.
fn packed(folder: &Path, asked: &Asked, scratch: &Path, whole: bool) -> Result<Sending, String> {
    let lock = fs::read_to_string(folder.join("flake.lock")).unwrap_or_default();
    let kept = kept_here(&lock);
    if !kept.is_empty() {
        return Err(format!(
            "{} takes {} from this machine alone",
            folder.display(),
            kept.join(", ")
        ));
    }
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let tarball = mine(scratch, "folder.tgz");
    let mut tar = Command::new("tar");
    tar.arg("-czf").arg(&tarball).arg("-C").arg(folder);
    let listing: &[&str] = if whole {
        UNIGNORED
    } else {
        &["ls-files", "-z"]
    };
    let packed = match git(folder, listing, None) {
        Some(files) => {
            // A tracked file deleted and not yet committed is listed but
            // gone, which a build of the folder doesn't read either.
            tar.args(["--ignore-failed-read", "--null", "-T", "-"])
                .stdin(Stdio::piped());
            tar.spawn().and_then(|mut packing| {
                if let Some(mut input) = packing.stdin.take() {
                    std::io::Write::write_all(&mut input, files.as_bytes())?;
                }
                packing.wait()
            })
        }
        None => tar
            .args(["--exclude=./.git", "--exclude=./result*", "."])
            .status(),
    };
    if !packed.is_ok_and(|status| status.success()) {
        let _ = fs::remove_file(&tarball);
        return Err(format!("{} couldn't be packed", folder.display()));
    }
    let size = fs::metadata(&tarball).map_or(0, |file| file.len());
    eprintln!(
        "nix-offload: sending {} as a tarball of {}",
        folder.display(),
        sized(size)
    );
    // A folder named for this run, gone when its shell ends.
    let into = format!("/tmp/nix-offload-there-{}", std::process::id());
    let command = format!(
        "set -e; rm -rf {into}; mkdir -p {into}; trap 'rm -rf {into}' EXIT; \
         tar -xzf - -C {into}; {}",
        asked.command_in(&format!("path:{into}"), false)
    );
    Ok(Sending {
        command,
        bundle: Some(tarball),
    })
}

/// What the lent builder is given for a build: the command its shell
/// runs, and a file sent on its standard input, a bundle of commits it
/// lacks or a tarball of a folder.
pub struct Sending {
    pub command: String,
    pub bundle: Option<PathBuf>,
}

/// How a flake that's a git tree here reaches the builder: as a commit
/// GitHub has, or as the commits since the newest one it has, bundled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Pushed {
        /// The repository on GitHub, `owner/name`.
        github: String,
        commit: String,
        /// The flake's folder in the repository, when not its top.
        dir: Option<String>,
    },
    Bundled {
        /// The repository on GitHub, `owner/name`.
        github: String,
        base: String,
        commit: String,
        /// The flake's folder in the repository, when not its top.
        dir: Option<String>,
        bundle: PathBuf,
    },
}

impl Source {
    fn sending(self, asked: &Asked) -> Sending {
        match self {
            Source::Pushed {
                github,
                commit,
                dir,
            } => Sending {
                command: asked.command_of(&format!(
                    "github:{github}/{commit}{}",
                    dir.as_deref()
                        .map(|dir| format!("?dir={dir}"))
                        .unwrap_or_default()
                )),
                bundle: None,
            },
            Source::Bundled {
                github,
                base,
                commit,
                dir,
                bundle,
            } => {
                // A folder named for this run, gone when its shell ends.
                let into = format!("/tmp/nix-offload-there-{}", std::process::id());
                let flake = format!(
                    "git+file://{into}?rev={commit}{}",
                    dir.map(|dir| format!("&dir={dir}")).unwrap_or_default()
                );
                // A builder's start-up installs git, and one whose
                // install failed borrows it from Nix.
                let command = format!(
                    "set -e; rm -rf {into}; mkdir -p {into}; trap 'rm -rf {into}' EXIT; \
                     cd {into}; cat > there.bundle; \
                     command -v git >/dev/null || \
                     git() {{ nix shell nixpkgs#git -c git \"$@\"; }}; \
                     git init -q; git fetch -q https://github.com/{github} {base}; \
                     git fetch -q there.bundle {BUNDLED}:refs/heads/there; {}",
                    asked.command_of(&flake)
                );
                Sending {
                    command,
                    bundle: Some(bundle),
                }
            }
        }
    }
}

/// What `there --takes` says it takes: a flake that's a git tree here,
/// `--apply`, `--out`, waiting for a builder it asks for, a flake that's a
/// folder no commit names, `--changed`, changed-checks run elsewhere, and
/// `--here`, a build no builder took made here.
pub const TAKES: [&str; 7] = ["tree", "apply", "out", "wants", "folder", "changed", "here"];

/// The ref a bundle carries its commit under, here only while it's made.
pub const BUNDLED: &str = "refs/nix-offload/there";

/// The GitHub repository a remote's URL names, `owner/name`.
fn github_of(url: &str) -> Option<String> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let path = path.trim_end_matches('/').trim_end_matches(".git");
    (path.split('/').count() == 2 && !path.starts_with('/')).then(|| path.to_owned())
}

/// git's answer, trimmed, in `top`, or none when it fails.
fn git(top: &Path, arguments: &[&str], index: Option<&Path>) -> Option<String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(top).args(arguments);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let output = command.stderr(Stdio::null()).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// How the flake in `folder` reaches the builder. Its commit is the
/// tree's HEAD, or, with changes to the files its index holds, staged
/// new ones included, a commit of them made without touching the branch
/// or the index, as `nix build` of the tree here would read them. The newest commit GitHub has is where it meets
/// its upstream branch, else origin's main. A tree that isn't git, or
/// whose origin isn't on GitHub, can't reach it.
pub fn source(folder: &Path, scratch: &Path) -> Result<Source, String> {
    let folder =
        fs::canonicalize(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let not = |what: &str| format!("{} {what}", folder.display());
    let top = git(&folder, &["rev-parse", "--show-toplevel"], None)
        .map(PathBuf::from)
        .ok_or_else(|| not("isn't in a git tree"))?;
    let github = git(&top, &["remote", "get-url", "origin"], None)
        .as_deref()
        .and_then(github_of)
        .ok_or_else(|| not("has no origin on GitHub"))?;
    let dir = folder
        .strip_prefix(&top)
        .ok()
        .map(|dir| dir.to_string_lossy().into_owned())
        .filter(|dir| !dir.is_empty());
    let head = git(&top, &["rev-parse", "HEAD"], None).ok_or_else(|| not("has no commit"))?;
    let commit = if git(&top, &["diff", "--quiet", "HEAD"], None).is_some() {
        head.clone()
    } else {
        fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
        let index = mine(scratch, "index");
        // Seeded from the tree's own index, as Nix reads a tree, so a file
        // staged, or added with -N, goes too; `add -u` then takes every
        // tracked file as it stands, and anything untracked stays here.
        let made = git(&top, &["rev-parse", "--git-path", "index"], None)
            .map(|own| top.join(own))
            .filter(|own| fs::copy(own, &index).is_ok())
            .and_then(|_| git(&top, &["add", "-u"], Some(&index)))
            .and_then(|_| git(&top, &["write-tree"], Some(&index)))
            .and_then(|tree| {
                git(
                    &top,
                    &[
                        "commit-tree",
                        &tree,
                        "-p",
                        &head,
                        "-m",
                        "nix-offload there: uncommitted changes",
                    ],
                    None,
                )
            });
        let _ = fs::remove_file(&index);
        made.ok_or_else(|| not("has changes git couldn't commit"))?
    };
    let base = ["@{upstream}", "origin/main"]
        .iter()
        .find_map(|theirs| git(&top, &["merge-base", &commit, theirs], None))
        .ok_or_else(|| not("shares no commit with origin"))?;
    if base == commit {
        return Ok(Source::Pushed {
            github,
            commit,
            dir,
        });
    }
    let bundle = mine(scratch, "bundle");
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let referenced = git(&top, &["update-ref", BUNDLED, &commit], None).is_some();
    let bundled = referenced
        && git(
            &top,
            &[
                "bundle",
                "create",
                "-q",
                &bundle.to_string_lossy(),
                &format!("^{base}"),
                BUNDLED,
            ],
            None,
        )
        .is_some();
    let _ = git(&top, &["update-ref", "-d", BUNDLED], None);
    if !bundled {
        return Err(not("couldn't be bundled"));
    }
    Ok(Source::Bundled {
        github,
        base,
        commit,
        dir,
        bundle,
    })
}

/// ssh to `host`, never asking anything, logging where `said` names, whose
/// verbose log ends with what it sent, which is what a build put on the
/// connection.
pub fn reaching(ssh: &str, said: &Path) -> Command {
    let mut command = Command::new(ssh);
    command
        .args(["-o", "BatchMode=yes", "-o", "LogLevel=VERBOSE"])
        // A tunnel's connection can die with nothing on either end
        // closing it, and then ssh waits on it forever: on 30 October it
        // waited 30 minutes under a build, saying nothing. Unanswered for
        // a minute, it's lost, which a lent builder's run sends again.
        .args([
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=4",
        ])
        .arg("-E")
        .arg(said);
    command
}

/// A size in bytes as a person reads it, to a tenth of its unit.
fn sized(bytes: u64) -> String {
    let mut scale = 1;
    for unit in ["KB", "MB", "GB"] {
        scale *= 1000;
        if bytes < scale * 1000 || unit == "GB" {
            let tenths = bytes / (scale / 10);
            if bytes >= scale {
                return format!("{}.{} {unit}", tenths / 10, tenths % 10);
            }
            break;
        }
    }
    format!("{bytes} bytes")
}

/// A file of this run's own in `scratch`.
pub fn mine(scratch: &Path, name: &str) -> PathBuf {
    scratch.join(format!("{name}-{}", std::process::id()))
}

pub fn on(
    lent: &Builder,
    sending: &Sending,
    out: Option<&Path>,
    scratch: &Path,
    ssh: &str,
) -> Result<Ran, String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let known = mine(scratch, "known");
    let said = mine(scratch, "ssh.log");
    let alias = lent.host.rsplit('@').next().unwrap_or(&lent.host);
    let mut command = reaching(ssh, &said);
    if let Some(key) = &lent.key {
        command.args(["-o", "IdentitiesOnly=yes", "-i", key]);
    }
    if let Some(host_key) = &lent.host_key {
        fs::write(&known, format!("{alias} {host_key}\n"))
            .map_err(|error| format!("{}: {error}", known.display()))?;
        command
            .args(["-o", "StrictHostKeyChecking=yes", "-o"])
            .arg(format!("UserKnownHostsFile={}", known.display()))
            .args(["-o", &format!("HostKeyAlias={alias}")]);
    }
    eprintln!("nix-offload: building on the lent builder");
    command.arg(&lent.host).arg(&sending.command);
    if let Some(bundle) = &sending.bundle {
        let bundle = fs::File::open(bundle).map_err(|error| format!("the bundle: {error}"))?;
        command.stdin(bundle);
    }
    let status = match out {
        None => command.status(),
        Some(out) => unpacked_into(&mut command, out),
    };
    let sent = fs::read_to_string(&said).ok().and_then(|log| sent(&log));
    let _ = fs::remove_file(&known);
    let _ = fs::remove_file(&said);
    let status = status.map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    Ok(Ran {
        on: Some(lent.host.clone()),
        // A build killed by a signal there comes back as ssh's status.
        status: status.code().unwrap_or(UNREACHED),
        sent,
    })
}

/// Runs `command`, unpacking the tarball it writes into `out`, and gives
/// its status. A build that failed writes none, so nothing is unpacked.
fn unpacked_into(command: &mut Command, out: &Path) -> std::io::Result<std::process::ExitStatus> {
    fs::create_dir_all(out)?;
    let mut running = command.stdout(Stdio::piped()).spawn()?;
    let tarball = running
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("no output to unpack"))?;
    let unpacking = Command::new("tar")
        .arg("-xzf")
        .arg("-")
        .arg("-C")
        .arg(out)
        .stdin(tarball)
        .stderr(Stdio::null())
        .status();
    let status = running.wait()?;
    if status.success() && !unpacking.is_ok_and(|unpacked| unpacked.success()) {
        eprintln!(
            "nix-offload: couldn't unpack the output into {}",
            out.display()
        );
    }
    Ok(status)
}

/// How a build waits for a builder when none stands: how long the lender
/// has to start making one, since it reads what's wanted every 20 s, and
/// how long one may take to stand, about 70 s on 29 October.
#[derive(Clone, Copy)]
pub struct Waiting {
    pub answered: std::time::Duration,
    pub longest: std::time::Duration,
}

pub const WAITING: Waiting = Waiting {
    answered: std::time::Duration::from_secs(30),
    longest: std::time::Duration::from_secs(240),
};

/// Whether a pid runs, as signal 0 says.
fn runs(pid: i32) -> bool {
    // SAFETY: signal 0 only asks whether the process exists.
    pid > 0 && unsafe { libc::kill(pid, 0) } == 0
}

/// The pid a file holds, when it holds one.
fn pid_in(path: &Path) -> Option<i32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Waits for a builder when none stands, rather than building here: it
/// says it wants one by a file in `dir`'s `wanted` named for its pid,
/// which the lender counts as queued work, for as long as it waits. It
/// waits only while the lender whose pid `dir`'s `lender` holds runs, and
/// gives up once that lender hasn't begun making one (`lending` is empty)
/// within `answered`, or none stands within `longest`. It gives the
/// store's text once one stands.
pub fn waited(dir: &Path, waiting: Waiting) -> Option<String> {
    if !pid_in(&dir.join("lender")).is_some_and(runs) {
        return None;
    }
    let wanted = dir.join("wanted");
    let want = wanted.join(std::process::id().to_string());
    fs::create_dir_all(&wanted).ok()?;
    fs::write(&want, "").ok()?;
    eprintln!("nix-offload: no builder stands, so this asks for one and waits for it");
    let began = std::time::Instant::now();
    let read = |name: &str| {
        fs::read_to_string(dir.join(name))
            .ok()
            .filter(|text| !text.trim().is_empty())
    };
    let stood = loop {
        if let Some(store) = read("store") {
            break Some(store);
        }
        let so_far = began.elapsed();
        if so_far >= waiting.longest || (so_far >= waiting.answered && read("lending").is_none()) {
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(250).min(waiting.answered));
    };
    let _ = fs::remove_file(&want);
    stood
}

/// A builder's store once one stands again after ssh lost it, giving up
/// `waiting.longest` after `since`: it looks every few seconds, since the
/// store may still name the builder whose tunnel just went, and waits as
/// `waited` does once the store is empty. Only a lender running stands one
/// again, but a build is most often lost because its lender restarted, as
/// a switch does, so a lender that isn't running is waited for up to
/// `waiting.answered` after `since`.
pub fn stood_again(dir: &Path, waiting: Waiting, since: std::time::Instant) -> Option<String> {
    loop {
        if since.elapsed() >= waiting.longest {
            return None;
        }
        std::thread::sleep(AGAIN.min(waiting.answered));
        if pid_in(&dir.join("lender")).is_some_and(runs) {
            return fs::read_to_string(dir.join("store"))
                .ok()
                .filter(|store| !store.trim().is_empty())
                .or_else(|| waited(dir, waiting));
        }
        if since.elapsed() >= waiting.answered {
            return None;
        }
    }
}

/// How long a build lost mid-way waits before it's sent again.
const AGAIN: std::time::Duration = std::time::Duration::from_secs(5);

/// Whether a build in `dir`'s `wanted` is waiting for a builder: a file
/// there is named for a pid that still runs. One whose build ended
/// without taking it away is taken away here.
pub fn wanted(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir.join("wanted")) else {
        return false;
    };
    let mut waiting = false;
    for entry in entries.filter_map(Result::ok) {
        let pid = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok());
        if pid.is_some_and(runs) {
            waiting = true;
        } else {
            let _ = fs::remove_file(entry.path());
        }
    }
    waiting
}

/// A machine nearby that builds for systems this one can't, such as a Mac
/// at home for its own: where it answers, `host:port`, the host ssh is
/// given, the systems it takes, and the attributes it takes whatever their
/// system, as patterns where `*` stands for any run of characters. A Mac's
/// Linux builder runs under emulation, so it may take the checks that are
/// only evaluation, which it does itself, without taking every Linux build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nearby {
    pub address: String,
    pub host: String,
    pub systems: Vec<String>,
    pub attributes: Vec<String>,
}

impl Nearby {
    fn takes(&self, attribute: &str) -> bool {
        system_of(attribute).is_some_and(|system| self.systems.iter().any(|s| s == system))
            || self
                .attributes
                .iter()
                .any(|pattern| matches(pattern, attribute))
    }
}

/// Whether `text` is what `pattern` describes, where each `*` stands for
/// any run of characters, none included.
fn matches(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let mut parts: Vec<&str> = parts.collect();
    let Some(last) = parts.pop() else {
        // No star: the whole of it had to be the first part.
        return rest.is_empty();
    };
    for part in parts {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

/// The machines nearby in a file of them, a line each:
/// `ADDRESS HOST SYSTEM,SYSTEM [PATTERN,PATTERN]`. Blank lines and those
/// starting `#` say nothing, and a line short of a word names no machine.
pub fn nearby(text: &str) -> Vec<Nearby> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let (address, host, systems) = (words.next()?, words.next()?, words.next()?);
            Some(Nearby {
                address: address.to_owned(),
                host: host.to_owned(),
                systems: systems.split(',').map(str::to_owned).collect(),
                attributes: words
                    .next()
                    .map(|patterns| patterns.split(',').map(str::to_owned).collect())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// The system an attribute is built for: its second word, as flakes name
/// their outputs, `checks.aarch64-darwin.x`.
fn system_of(attribute: &str) -> Option<&str> {
    attribute.split('.').nth(1)
}

/// What was asked, shared out: each attribute a machine nearby takes, by
/// its system or by name, goes to the first that does and `answers`, and
/// the rest is left for the lent builder or here. Whether one answers is
/// asked once.
pub fn shared<'a>(
    asked: &Asked,
    nearby: &'a [Nearby],
    answers: impl Fn(&str) -> bool,
) -> (Vec<(&'a Nearby, Asked)>, Option<Asked>) {
    // An applied function or an output copied back goes the lent way,
    // which knows them.
    if asked.apply.is_some() || asked.out.is_some() {
        return (
            Vec::new(),
            Some(asked.with(&asked.flake, asked.attributes.clone())),
        );
    }
    // Only a machine that takes something asked is asked whether it
    // answers: an away machine's name can take seconds to fail to resolve.
    let answering: Vec<&Nearby> = nearby
        .iter()
        .filter(|machine| {
            asked
                .attributes
                .iter()
                .any(|attribute| machine.takes(attribute))
        })
        .filter(|machine| answers(&machine.address))
        .collect();
    let mut parts: Vec<(&Nearby, Vec<String>)> = Vec::new();
    let mut left = Vec::new();
    for attribute in &asked.attributes {
        let taker = answering.iter().find(|machine| machine.takes(attribute));
        match taker {
            Some(machine) => match parts.iter_mut().find(|(had, _)| had == machine) {
                Some((_, attributes)) => attributes.push(attribute.clone()),
                None => parts.push((machine, vec![attribute.clone()])),
            },
            None => left.push(attribute.clone()),
        }
    }
    let parts = parts
        .into_iter()
        .map(|(machine, attributes)| (machine, asked.with(&asked.flake, attributes)))
        .collect();
    let left = (!left.is_empty()).then(|| asked.with(&asked.flake, left));
    (parts, left)
}

/// The folder here a flake is, when it's one no other machine could fetch:
/// `path:DIR`, or a path itself.
fn folder_of(flake: &str) -> Option<PathBuf> {
    let path = flake.strip_prefix("path:").unwrap_or(flake);
    let path = path.split('?').next().unwrap_or(path);
    (path.starts_with('/') || path.starts_with('.')).then(|| PathBuf::from(path))
}

/// The store paths a flake's lock pins inputs to, which only a store that
/// already has them can read: a nearby machine is sent them.
fn pinned(lock: &str) -> Vec<String> {
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(lock) else {
        return Vec::new();
    };
    let mut found: Vec<String> = lock["nodes"]
        .as_object()
        .into_iter()
        .flat_map(|nodes| nodes.values())
        .filter(|node| node["locked"]["type"] == "path")
        .filter_map(|node| node["locked"]["path"].as_str())
        .filter(|path| path.starts_with("/nix/store/"))
        .map(str::to_owned)
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The inputs a flake's lock takes from this machine alone, a store path
/// or a repository at a `file:` URL, which the lent builder can't fetch:
/// such a flake is built here rather than failing there, since what they
/// hold, as a machine's secrets, isn't for sending.
fn kept_here(lock: &str) -> Vec<String> {
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(lock) else {
        return Vec::new();
    };
    let mut found: Vec<String> = lock["nodes"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, node)| {
            let locked = &node["locked"];
            locked["type"] == "path"
                || locked["url"]
                    .as_str()
                    .is_some_and(|url| url.starts_with("file:"))
        })
        .map(|(name, _)| name.clone())
        .collect();
    found.sort();
    found
}

/// Where on a nearby machine, under its account's home, a flake sent as a
/// folder is unpacked for the length of its build.
fn unpacked() -> String {
    format!(".cache/nix-offload/there/{}", std::process::id())
}

/// Builds what was asked on a machine nearby, or here when ssh can't reach
/// it, sending a flake that's a folder here with it. `tar` and `nix` are
/// the programs run here, beside `ssh`.
pub fn nearby_run(machine: &Nearby, asked: &Asked, scratch: &Path, programs: [&str; 3]) -> Ran {
    let [ssh, tar, nix] = programs;
    match sent_nearby(machine, asked, scratch, ssh, tar, nix) {
        Ok(ran) if ran.status != UNREACHED => ran,
        Ok(_) => {
            eprintln!(
                "nix-offload: ssh couldn't reach {}, so this builds here",
                machine.host
            );
            here(asked, nix)
        }
        Err(error) => {
            eprintln!("nix-offload: {error}, so this builds here");
            here(asked, nix)
        }
    }
}

fn sent_nearby(
    machine: &Nearby,
    asked: &Asked,
    scratch: &Path,
    ssh: &str,
    tar: &str,
    nix: &str,
) -> Result<Ran, String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let said = mine(scratch, "ssh.log");
    let mut sent: Option<u64> = None;
    let command = match folder_of(&asked.flake) {
        None => asked.command(),
        Some(folder) => {
            let into = unpacked();
            let listed = mine(scratch, "files");
            let status = send(&folder, &into, &machine.host, &said, &listed, ssh, tar);
            let _ = fs::remove_file(&listed);
            let status = status?;
            counted(&mut sent, &said);
            if status != 0 {
                return Ok(Ran {
                    on: Some(machine.host.clone()),
                    status,
                    sent,
                });
            }
            copy_pinned(&folder, &machine.host, nix);
            // Built where it was unpacked, which goes once the build ends,
            // its status kept.
            format!(
                "cd {into} && {}; ended=$?; cd && rm -rf {into}; exit $ended",
                asked.with("path:.", asked.attributes.clone()).command()
            )
        }
    };
    eprintln!("nix-offload: building on {}, {}", machine.host, asked.flake);
    let status = reaching(ssh, &said)
        .arg(&machine.host)
        .arg(command)
        .status()
        .map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    counted(&mut sent, &said);
    Ok(Ran {
        on: Some(machine.host.clone()),
        status: status.code().unwrap_or(UNREACHED),
        sent,
    })
}

/// Adds what ssh's log at `said` says it sent to `sent`, and drops the log.
fn counted(sent: &mut Option<u64>, said: &Path) {
    if let Some(bytes) = fs::read_to_string(said)
        .ok()
        .and_then(|log| self::sent(&log))
    {
        *sent = Some(sent.unwrap_or(0) + bytes);
    }
    let _ = fs::remove_file(said);
}

/// Sends `folder` to `into` on `host` as a compressed tarball and gives
/// ssh's status. In a git tree it holds the files git doesn't ignore,
/// listed in `listed`, as `packed` sends them: a working tree's zone held
/// 509 MB on 30 October, almost all one ignored target folder. Outside
/// one it holds the folder but its git history and the `result` links
/// builds leave there.
fn send(
    folder: &Path,
    into: &str,
    host: &str,
    said: &Path,
    listed: &Path,
    ssh: &str,
    tar: &str,
) -> Result<i32, String> {
    let mut packing = Command::new(tar);
    packing.arg("-czf").arg("-").arg("-C").arg(folder);
    match git(folder, UNIGNORED, None) {
        Some(files) => {
            fs::write(listed, files).map_err(|error| format!("{}: {error}", listed.display()))?;
            // A tracked file deleted and not yet committed is listed but
            // gone, which a build of the folder doesn't read either.
            packing
                .args(["--ignore-failed-read", "--null", "-T"])
                .arg(listed);
        }
        None => {
            packing.args(["--exclude=./.git", "--exclude=./result*", "."]);
        }
    }
    let mut packing = packing
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("couldn't run {tar}: {error}"))?;
    let packed = packing
        .stdout
        .take()
        .ok_or_else(|| format!("{tar} gave nothing to send"))?;
    let status = reaching(ssh, said)
        .arg(host)
        .arg(format!(
            "rm -rf {into} && mkdir -p {into} && tar -xzf - -C {into}"
        ))
        .stdin(packed)
        .status()
        .map_err(|error| format!("couldn't run {ssh}: {error}"))?;
    let packing = packing
        .wait()
        .map_err(|error| format!("couldn't run {tar}: {error}"))?;
    if !packing.success() {
        return Err(format!("{tar} couldn't pack {}", folder.display()));
    }
    Ok(status.code().unwrap_or(UNREACHED))
}

/// Copies to `host` the store paths `folder`'s lock pins inputs to. One
/// that can't be copied is said and left, since a build that never reads
/// that input still builds.
fn copy_pinned(folder: &Path, host: &str, nix: &str) {
    let paths = fs::read_to_string(folder.join("flake.lock"))
        .map(|lock| pinned(&lock))
        .unwrap_or_default();
    if paths.is_empty() {
        return;
    }
    // ssh's control socket goes in TMPDIR, and a long one is past what a
    // socket's path may be. This machine signs nothing it builds, so the
    // machine nearby takes the copy on its account's trust.
    let copied = Command::new(nix)
        .args([
            "copy",
            "--no-check-sigs",
            "--to",
            &format!("ssh-ng://{host}"),
        ])
        .args(&paths)
        .env("TMPDIR", "/tmp")
        .status();
    if !copied.is_ok_and(|status| status.success()) {
        eprintln!("nix-offload: couldn't copy the inputs pinned to the store to {host}");
    }
}

/// Adds a line for a build to `journal`: when it ended, how long it took,
/// what it built, where, its status and what it sent.
pub fn note(journal: &Path, at: u64, seconds: u64, asked: &Asked, ran: &Ran) {
    let line = serde_json::json!({
        "at": at,
        "seconds": seconds,
        "flake": asked.flake,
        "attributes": asked.attributes,
        "on": ran.on,
        "status": ran.status,
        "sent": ran.sent,
    });
    let written = journal
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(journal)
        })
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        eprintln!(
            "nix-offload: couldn't journal the build in {}: {error}",
            journal.display()
        );
    }
}

/// What ssh's verbose log says it sent, from its closing count:
/// `Transferred: sent 4321, received 98765 bytes, in 3.2 seconds`.
fn sent(log: &str) -> Option<u64> {
    log.lines().rev().find_map(|line| {
        line.split_once("Transferred: sent ")?
            .1
            .split_once(',')?
            .0
            .parse()
            .ok()
    })
}

/// A word the builder's shell reads as itself.
pub fn quoted(word: &str) -> String {
    if word
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"-_./:".contains(&byte))
    {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// Base64 as text, or none when it isn't both. Nix's store URIs carry a
/// host key this way, and it's the only base64 this reads, so it's
/// decoded here rather than with a crate.
fn decoded(encoded: &str) -> Option<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits: u32 = 0;
    let mut held = 0;
    let mut bytes = Vec::new();
    for byte in encoded.trim_end_matches('=').bytes() {
        let value = ALPHABET.iter().position(|letter| *letter == byte)?;
        bits = (bits << 6) | u32::try_from(value).ok()?;
        held += 6;
        if held >= 8 {
            held -= 8;
            bytes.push(u8::try_from(bits >> held).ok()?);
            bits &= (1 << held) - 1;
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_is_said_to_a_tenth_of_its_unit() {
        assert_eq!(sized(500), "500 bytes");
        assert_eq!(sized(11_964), "11.9 KB");
        assert_eq!(sized(3_913_640), "3.9 MB");
        assert_eq!(sized(793_000_000), "793.0 MB");
        assert_eq!(sized(2_500_000_000_000), "2500.0 GB");
    }

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_flake_and_its_attributes_are_asked_with_their_flags_anywhere() {
        let asked = asked(&words(
            "-L github:o/r/abc checks.x.a --keep-going checks.x.b",
        ));
        assert_eq!(
            asked,
            Some(Asked {
                flake: "github:o/r/abc".to_owned(),
                attributes: words("checks.x.a checks.x.b"),
                flags: words("-L --keep-going"),
                apply: None,
                out: None,
                here: false,
            })
        );
    }

    #[test]
    fn an_applied_function_or_an_output_copied_back_takes_one_attribute() {
        let one = asked(&words("--apply f --out /o github:o/r/abc checks.x.a")).unwrap();
        assert_eq!(one.apply.as_deref(), Some("f"));
        assert_eq!(one.out, Some(PathBuf::from("/o")));
        assert_eq!(
            asked(&words("--apply f github:o/r/abc checks.x.a checks.x.b")),
            None
        );
        assert_eq!(
            asked(&words("--out /o github:o/r/abc checks.x.a checks.x.b")),
            None
        );
        assert_eq!(asked(&words("github:o/r/abc checks.x.a --apply")), None);
    }

    #[test]
    fn an_applied_function_is_built_as_an_expression_impure_only_here() {
        let asked = asked(&[
            "--apply".to_owned(),
            "c: c.extend {}".to_owned(),
            "/t".to_owned(),
            "checks.x.a".to_owned(),
        ])
        .unwrap();
        assert_eq!(
            asked.build(),
            [
                "build",
                "--no-link",
                "--impure",
                "--expr",
                r#"(c: c.extend {}) (builtins.getFlake "/t").checks.x.a"#,
            ]
        );
        assert_eq!(
            asked.command_of("github:o/r/abc"),
            r#"nix build --no-link --expr '(c: c.extend {}) (builtins.getFlake "github:o/r/abc").checks.x.a'"#
        );
    }

    #[test]
    fn an_output_copied_back_comes_as_a_tarball_on_the_builders_output() {
        let asked = asked(&words("--out /o github:o/r/abc checks.x.a")).unwrap();
        assert_eq!(
            asked.command(),
            "out=$(nix build --no-link --print-out-paths 'github:o/r/abc#checks.x.a') \
             && tar -czf - -C \"$out\" ."
        );
    }

    const QUICK: Waiting = Waiting {
        answered: std::time::Duration::from_millis(300),
        longest: std::time::Duration::from_secs(5),
    };

    #[test]
    fn with_no_lender_running_nothing_is_waited_for() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(waited(dir.path(), QUICK), None);
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        assert_eq!(
            waited(dir.path(), QUICK),
            None,
            "one that ended lends nothing"
        );
        assert!(!dir.path().join("wanted").exists());
    }

    #[test]
    fn a_build_waits_wanting_a_builder_until_one_stands() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let lender = dir.path().to_owned();
        let making = std::thread::spawn(move || {
            while !wanted(&lender) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            fs::write(lender.join("lending"), "a-builder\n").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(500));
            fs::write(lender.join("store"), "ssh-ng://far\n").unwrap();
        });
        assert_eq!(waited(dir.path(), QUICK).as_deref(), Some("ssh-ng://far\n"));
        making.join().unwrap();
        assert!(!wanted(dir.path()), "nothing is wanted once it stands");
    }

    #[test]
    fn a_build_stops_waiting_once_the_lender_makes_none() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lender"),
            format!("{}\n", std::process::id()),
        )
        .unwrap();
        let began = std::time::Instant::now();
        assert_eq!(waited(dir.path(), QUICK), None);
        assert!(
            began.elapsed() < QUICK.longest,
            "it gave up once nothing was being made"
        );
    }

    #[test]
    fn a_build_lost_as_its_lender_restarts_waits_for_the_new_one() {
        let dir = tempfile::tempdir().unwrap();
        // The lender that just stopped, as a switch stops it.
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        let lender = dir.path().to_owned();
        let restarting = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            fs::write(lender.join("store"), "ssh-ng://far\n").unwrap();
            fs::write(lender.join("lender"), format!("{}\n", std::process::id())).unwrap();
        });
        assert_eq!(
            stood_again(dir.path(), QUICK, std::time::Instant::now()).as_deref(),
            Some("ssh-ng://far\n")
        );
        restarting.join().unwrap();
    }

    #[test]
    fn a_build_lost_with_no_lender_coming_back_stops_waiting() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("lender"), "999999999\n").unwrap();
        fs::write(dir.path().join("store"), "ssh-ng://far\n").unwrap();
        let began = std::time::Instant::now();
        assert_eq!(stood_again(dir.path(), QUICK, began), None);
        assert!(
            began.elapsed() < QUICK.longest,
            "it gave up once no lender came back"
        );
    }

    #[test]
    fn a_want_whose_build_ended_is_taken_away() {
        let dir = tempfile::tempdir().unwrap();
        let wanting = dir.path().join("wanted");
        fs::create_dir(&wanting).unwrap();
        fs::write(wanting.join("999999999"), "").unwrap();
        assert!(!wanted(dir.path()));
        assert!(!wanting.join("999999999").exists());
        fs::write(wanting.join(std::process::id().to_string()), "").unwrap();
        assert!(wanted(dir.path()));
    }

    #[test]
    fn a_github_remote_names_its_repository() {
        for url in [
            "https://github.com/o/r",
            "https://github.com/o/r.git",
            "git@github.com:o/r.git",
            "ssh://git@github.com/o/r",
        ] {
            assert_eq!(github_of(url).as_deref(), Some("o/r"), "{url}");
        }
        assert_eq!(github_of("https://delta.example/o/r"), None);
        assert_eq!(github_of("https://github.com/o"), None);
    }

    #[test]
    fn a_build_with_no_attribute_or_an_unknown_flag_is_refused() {
        assert_eq!(asked(&words("github:o/r/abc")), None);
        assert_eq!(asked(&words("-L")), None);
        assert_eq!(asked(&words("--impure github:o/r/abc checks.x.a")), None);
    }

    #[test]
    fn the_builder_is_given_one_quoted_command() {
        let asked = asked(&words("-L github:o/r/abc checks.x.it's")).unwrap();
        assert_eq!(
            asked.command(),
            r"nix build --no-link -L 'github:o/r/abc#checks.x.it'\''s'"
        );
    }

    #[test]
    fn a_store_names_its_host_its_key_and_its_host_key() {
        let store = "ssh-ng://nix-offload?compress=true&ssh-key=/k&\
                     base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==";
        assert_eq!(
            builder(store),
            Some(Builder {
                host: "nix-offload".to_owned(),
                key: Some("/k".to_owned()),
                host_key: Some("ssh-ed25519 AAAA".to_owned()),
            })
        );
        assert_eq!(
            builder("ssh://someone@far").map(|lent| lent.host),
            Some("someone@far".to_owned())
        );
        assert_eq!(builder("daemon"), None);
        assert_eq!(builder("ssh-ng://?ssh-key=/k"), None);
    }

    #[test]
    fn base64_is_decoded_whatever_its_padding() {
        assert_eq!(decoded("YQ==").as_deref(), Some("a"));
        assert_eq!(decoded("YWI=").as_deref(), Some("ab"));
        assert_eq!(decoded("YWJj").as_deref(), Some("abc"));
        assert_eq!(decoded("not base64!"), None);
    }

    fn mac() -> Nearby {
        Nearby {
            address: "mac.local:22".to_owned(),
            host: "mac".to_owned(),
            systems: words("aarch64-darwin aarch64-linux"),
            attributes: Vec::new(),
        }
    }

    #[test]
    fn nearby_machines_are_read_a_line_each() {
        let text = "# at home\n\nmac.local:22 mac aarch64-darwin,aarch64-linux\nshort line\n";
        assert_eq!(nearby(text), vec![mac()]);
        let named = "mac.local:22 mac aarch64-darwin,aarch64-linux \
                     checks.x86_64-linux.formatting,checks.x86_64-linux.*-evaluates\n";
        assert_eq!(
            nearby(named)[0].attributes,
            words("checks.x86_64-linux.formatting checks.x86_64-linux.*-evaluates")
        );
    }

    #[test]
    fn a_star_in_a_pattern_stands_for_any_run_of_characters() {
        assert!(matches("checks.x.formatting", "checks.x.formatting"));
        assert!(!matches("checks.x.formatting", "checks.x.formatting-too"));
        assert!(matches(
            "checks.x.*-evaluates",
            "checks.x.linux-dev-evaluates"
        ));
        assert!(matches("checks.x.*-evaluates", "checks.x.-evaluates"));
        assert!(!matches("checks.x.*-evaluates", "checks.y.dev-evaluates"));
        assert!(matches("checks.*.a*b", "checks.x.aXbYb"));
        assert!(!matches("checks.*.a*b", "checks.x.aXbY"));
    }

    #[test]
    fn a_nearby_machine_is_asked_whether_it_answers_only_when_it_takes_something_asked() {
        // An away machine's name can take seconds to fail to resolve, which
        // every build it would never take shouldn't wait out.
        let machines = [mac()];
        let probed = std::cell::Cell::new(0);
        let answers = |_: &str| {
            probed.set(probed.get() + 1);
            true
        };
        let linux = asked(&words("path:/z checks.x86_64-linux.a")).unwrap();
        let (near, _) = shared(&linux, &machines, answers);
        assert!(near.is_empty());
        assert_eq!(probed.get(), 0, "nothing asked is the machine's");
        let darwin = asked(&words("path:/z checks.aarch64-darwin.b")).unwrap();
        let (near, _) = shared(&darwin, &machines, answers);
        assert_eq!(near.len(), 1);
        assert_eq!(probed.get(), 1);
    }

    #[test]
    fn an_attribute_a_nearby_machine_names_goes_to_it_whatever_its_system() {
        let machines = [Nearby {
            attributes: words("checks.x86_64-linux.formatting checks.x86_64-linux.*-evaluates"),
            ..mac()
        }];
        let asked = asked(&words(
            "path:/z checks.x86_64-linux.formatting checks.x86_64-linux.dev-evaluates \
             checks.x86_64-linux.vm-boots checks.aarch64-darwin.b",
        ))
        .unwrap();
        let (near, left) = shared(&asked, &machines, |_| true);
        assert_eq!(
            near[0].1.attributes,
            words(
                "checks.x86_64-linux.formatting checks.x86_64-linux.dev-evaluates \
                 checks.aarch64-darwin.b"
            )
        );
        assert_eq!(
            left.map(|left| left.attributes),
            Some(words("checks.x86_64-linux.vm-boots"))
        );
    }

    #[test]
    fn each_attribute_goes_to_a_nearby_machine_taking_its_system_while_it_answers() {
        let machines = [mac()];
        let asked = asked(&words(
            "-L path:/z checks.x86_64-linux.a checks.aarch64-darwin.b \
             checks.aarch64-linux.c equilibrium",
        ))
        .unwrap();
        let (near, left) = shared(&asked, &machines, |_| true);
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].0, &machines[0]);
        assert_eq!(
            near[0].1.command(),
            "nix build --no-link -L 'path:/z#checks.aarch64-darwin.b' \
             'path:/z#checks.aarch64-linux.c'"
        );
        assert_eq!(
            left.map(|left| left.attributes),
            Some(words("checks.x86_64-linux.a equilibrium"))
        );

        let (near, left) = shared(&asked, &machines, |_| false);
        assert!(near.is_empty(), "one away takes nothing");
        assert_eq!(left.map(|left| left.attributes.len()), Some(4));

        let darwin = super::asked(&words("path:/z checks.aarch64-darwin.b")).unwrap();
        assert_eq!(shared(&darwin, &machines, |_| true).1, None);
    }

    #[test]
    fn only_a_flake_that_is_a_folder_here_is_sent_as_one() {
        assert_eq!(folder_of("path:/z?dir=x"), Some(PathBuf::from("/z")));
        assert_eq!(folder_of("./z"), Some(PathBuf::from("./z")));
        assert_eq!(folder_of("github:o/r/abc"), None);
        assert_eq!(folder_of("git+https://h/r"), None);
    }

    #[test]
    fn the_inputs_a_lock_pins_to_the_store_are_named() {
        let lock = r#"{"nodes": {
            "mint": {"locked": {"type": "path", "path": "/nix/store/abc-mint"}},
            "near": {"locked": {"type": "path", "path": "/home/me/near"}},
            "core": {"locked": {"type": "github", "owner": "o"}},
            "root": {"inputs": {}}
        }}"#;
        assert_eq!(pinned(lock), words("/nix/store/abc-mint"));
        assert!(pinned("not a lock").is_empty());
    }

    #[test]
    fn what_ssh_sent_is_read_from_its_last_count() {
        let log = "Authenticated to far\n\
                   Transferred: sent 4321, received 98765 bytes, in 3.2 seconds\n";
        assert_eq!(sent(log), Some(4321));
        assert_eq!(sent("Authenticated to far\n"), None);
    }
}
