//! The git a gate's answer runs, fenced: a bundle another machine wrote
//! is read by git, and git is a large program to hand a stranger's bytes.
//!
//! Each git runs under bubblewrap with a root of its own: the Nix store
//! read-only, a `/tmp` for its home, the folders it's given, and nothing
//! else of the machine, so no key, token, socket or startup file is
//! there to read or write. Only the fetch of the forge's main shares the
//! network, and with it the files a name is looked up by; the bundle's
//! own git runs with none. Sharing the network shares the abstract
//! sockets too, and on optimus those are the terminals' remote control
//! and the X server's (`ss -xl`, 15 November), so inside bubblewrap
//! `nix-offload scoped` puts each git in a Landlock domain that can't
//! reach an abstract socket or signal a process outside it, before it
//! runs git, and runs none where Landlock can't (ISXEG8, 15 November).
//!
//! Git reads no settings but these: no system or global file, no hooks,
//! no fsmonitor, no commit graph, no credential helper or prompt, and
//! only https and local folders as transports, so a fetch carries no
//! credential and a repo that needs one is refused for it. What a bundle
//! brings is checked object by object as it's fetched (`answer.rs`).

use std::env;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

/// The settings every git of a gate runs with.
const SETTINGS: [&str; 7] = [
    "core.hooksPath=/dev/null",
    "core.fsmonitor=false",
    "core.commitGraph=false",
    "credential.helper=",
    "protocol.allow=never",
    "protocol.https.allow=always",
    "protocol.file.allow=always",
];

/// The files a name is looked up by, for the one git that shares the
/// network.
const LOOKUP: [&str; 3] = ["/etc/resolv.conf", "/etc/hosts", "/etc/nsswitch.conf"];

/// How a gate's git is run.
#[derive(Debug, Clone)]
pub enum Fence {
    /// Under bubblewrap, scoped by this nix-offload, as `answer` runs it.
    Bwrap {
        /// bubblewrap.
        bwrap: PathBuf,
        /// This nix-offload, in the store, which scopes each git.
        scoped: PathBuf,
        /// git, in the store.
        git: PathBuf,
    },
    /// As it is, with the same settings, as a test runs it where no
    /// namespace can be made. Only tests build it, so nothing shipped can
    /// run a gate's git outside the fence.
    #[cfg(test)]
    Bare {
        /// git.
        git: PathBuf,
    },
}

/// Whether git may reach the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Net {
    /// It fetches the forge's main.
    Shared,
    /// It reads only what's here.
    None,
}

/// A program's path in the Nix store, through every link, or nothing.
fn in_store(path: &Path) -> Option<PathBuf> {
    let own = path.canonicalize().ok()?;
    own.starts_with("/nix/store/").then_some(own)
}

/// A program found on `PATH`, by its name.
pub fn on_path(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

/// The CA bundle git's https checks the forge against, in the store.
fn certificates() -> Option<&'static str> {
    option_env!("NIX_OFFLOAD_CACERT")
}

impl Fence {
    /// The fence `answer` runs git in: bubblewrap and git as nix-offload
    /// was built with them, each in the store, or why there is none.
    ///
    /// # Errors
    ///
    /// Off Linux, or when bubblewrap, git or this program isn't in the
    /// store.
    pub fn found() -> Result<Self, String> {
        if !cfg!(target_os = "linux") {
            return Err(
                "a gate answers only on Linux, where bubblewrap and Landlock are".to_owned(),
            );
        }
        let built = |name: Option<&str>, fallback: &str| {
            name.map(PathBuf::from)
                .or_else(|| on_path(fallback))
                .and_then(|path| in_store(&path))
        };
        let bwrap = built(option_env!("NIX_OFFLOAD_BWRAP"), "bwrap")
            .ok_or("bubblewrap isn't in the store")?;
        let git = built(option_env!("NIX_OFFLOAD_GIT"), "git").ok_or("git isn't in the store")?;
        let scoped = env::current_exe()
            .ok()
            .and_then(|path| in_store(&path))
            .ok_or("this nix-offload isn't in the store, where bubblewrap would see it")?;
        Ok(Self::Bwrap { bwrap, scoped, git })
    }

    /// git run with `arguments` in `folder`, writing only `writable` and
    /// reading `readable` beside the store, its output said back.
    ///
    /// # Errors
    ///
    /// When git can't start or fails, with what it said.
    pub fn git(
        &self,
        net: Net,
        folder: &Path,
        writable: &[&Path],
        readable: &[&Path],
        arguments: &[&str],
    ) -> Result<String, String> {
        let settings = SETTINGS.iter().flat_map(|setting| ["-c", setting]);
        let mut command = match self {
            #[cfg(test)]
            Self::Bare { git } => {
                let mut command = Command::new(git);
                command.current_dir(folder);
                command
            }
            Self::Bwrap { bwrap, scoped, git } => {
                let mut command = Command::new(bwrap);
                command.args(["--unshare-all", "--die-with-parent", "--new-session"]);
                if net == Net::Shared {
                    command.arg("--share-net");
                    for file in LOOKUP {
                        command.args(["--ro-bind-try", file, file]);
                    }
                }
                command.args(["--ro-bind", "/nix/store", "/nix/store"]);
                command.args(["--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp"]);
                for path in readable {
                    command.arg("--ro-bind").arg(path).arg(path);
                }
                for path in writable {
                    command.arg("--bind").arg(path).arg(path);
                }
                command.arg("--chdir").arg(folder);
                command.arg(scoped).args(["scoped", "--"]).arg(git);
                command
            }
        };
        command.env_clear().envs([
            ("HOME", "/tmp"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_TERMINAL_PROMPT", "0"),
            ("LC_ALL", "C"),
        ]);
        if let Some(certificates) = certificates() {
            command
                .env("GIT_SSL_CAINFO", certificates)
                .env("SSL_CERT_FILE", certificates);
        }
        let output = command
            .args(settings)
            .args(arguments)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("git couldn't start: {error}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let said = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "git {} failed: {}",
                arguments.join(" "),
                said.trim()
            ))
        }
    }
}

/// `nix-offload scoped -- PROGRAM WORD...`: runs the program in a
/// Landlock domain that reaches no abstract socket and signals nothing
/// outside it, and with no new privileges, or runs nothing. It's how a
/// gate's git runs inside bubblewrap, which can't make mounts once
/// Landlock holds it.
pub fn scoped(arguments: &[String]) -> ExitCode {
    let [dashes, program, words @ ..] = arguments else {
        eprintln!("nix-offload: scoped needs -- PROGRAM WORD...");
        return ExitCode::from(64);
    };
    if dashes != "--" {
        eprintln!("nix-offload: scoped needs -- PROGRAM WORD...");
        return ExitCode::from(64);
    }
    if let Err(error) = scope() {
        eprintln!(
            "nix-offload: scoped can't keep {program} from abstract sockets: {error}. Landlock's \
             scopes need Linux 6.12 or later; nothing runs without them."
        );
        return ExitCode::from(69);
    }
    let error = Command::new(program).args(words).exec();
    eprintln!("nix-offload: scoped couldn't start {program}: {error}");
    ExitCode::from(70)
}

/// Landlock's ruleset as the kernel reads it, at ABI 6.
#[cfg(target_os = "linux")]
#[repr(C)]
struct Ruleset {
    handled_access_fs: u64,
    handled_access_net: u64,
    scoped: u64,
}

/// Landlock's scope of abstract Unix sockets.
#[cfg(target_os = "linux")]
const SCOPE_ABSTRACT_UNIX_SOCKET: u64 = 1;
/// Landlock's scope of signals.
#[cfg(target_os = "linux")]
const SCOPE_SIGNAL: u64 = 2;

/// Puts this process, and what it runs, in a Landlock domain of its own
/// that reaches no abstract socket and signals nothing outside it.
#[cfg(target_os = "linux")]
fn scope() -> std::io::Result<()> {
    use std::io::Error;
    let ruleset = Ruleset {
        handled_access_fs: 0,
        handled_access_net: 0,
        scoped: SCOPE_ABSTRACT_UNIX_SOCKET | SCOPE_SIGNAL,
    };
    // SAFETY: the kernel reads the ruleset, of the size given, and
    // returns a descriptor or -1.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            &raw const ruleset,
            size_of::<Ruleset>(),
            0u32,
        )
    };
    if fd < 0 {
        return Err(Error::last_os_error());
    }
    let fd = libc::c_int::try_from(fd).map_err(|_| Error::other("a descriptor out of range"))?;
    // SAFETY: plain calls on this process and a descriptor it owns.
    let restricted = unsafe {
        libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == 0
            && libc::syscall(libc::SYS_landlock_restrict_self, fd, 0u32) == 0
    };
    let error = Error::last_os_error();
    // SAFETY: closes the descriptor this made.
    unsafe { libc::close(fd) };
    if restricted { Ok(()) } else { Err(error) }
}

/// Off Linux there's no Landlock, so nothing is run.
#[cfg(not(target_os = "linux"))]
fn scope() -> std::io::Result<()> {
    Err(std::io::Error::other("only Linux has Landlock"))
}
