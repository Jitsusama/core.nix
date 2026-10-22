//! The machine the guide runs on. Each program runs by its store path with
//! an empty environment but for a fixed locale and the secrets it's given,
//! so what it prints is what the parsers were written against, and nothing
//! the person's shell set can change what runs. Every call and all it
//! printed go to the log, raw, so whoever reads it later (pi, after an
//! enrolment) sees what the screen didn't show; a secret never does, since
//! none is ever an argument and the log names only where each went.

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::vault::store::Store;
use crate::vault::{Vault, VaultKey};
use crate::{Fault, Given, Journal, Machine, Program, Ran, Tool};

pub struct System {
    journal: Journal,
    log: Option<File>,
    secrets: Option<PathBuf>,
    vault: Option<Box<dyn Store>>,
    /// The vault's partition, for changing it through a handle of its own.
    vault_partition: Option<PathBuf>,
}

/// Where secrets a program reads from files are kept for as long as it
/// runs: under /run, which is memory, never a disk.
fn secret_place() -> Result<PathBuf, Fault> {
    // SAFETY: getuid can't fail and touches no memory.
    let place = if unsafe { libc::getuid() } == 0 {
        PathBuf::from("/run/guide")
    } else {
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(|runtime| Path::new(&runtime).join("guide"))
            .ok_or_else(|| Fault::Blocked {
                step: "keeping a secret",
                why: "there's no XDG_RUNTIME_DIR to keep it in memory".into(),
            })?
    };
    if !place.starts_with("/run") {
        return Err(Fault::Blocked {
            step: "keeping a secret",
            why: format!(
                "{} isn't under /run, so it may not be memory",
                place.display()
            ),
        });
    }
    Ok(place)
}

fn unkept(path: &Path) -> impl Fn(std::io::Error) -> Fault + '_ {
    move |why| Fault::Unkept {
        path: path.display().to_string(),
        why: why.to_string(),
    }
}

fn private_directory(path: &Path) -> Result<(), Fault> {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(unkept(path))
}

fn private_file(path: &Path, bytes: &[u8]) -> Result<(), Fault> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(unkept(path))?;
    file.write_all(bytes).map_err(unkept(path))
}

/// A file's inode flags, set to `set` when given, the ones before said
/// back; none where they can't be read, as on a system with no efivarfs.
fn flags(file: &File, set: Option<libc::c_long>) -> Option<libc::c_long> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let mut current: libc::c_long = 0;
        // SAFETY: the file is open and the flags are a long, as the ioctls
        // take them.
        unsafe {
            if libc::ioctl(file.as_raw_fd(), libc::FS_IOC_GETFLAGS, &mut current) != 0 {
                return None;
            }
            if let Some(set) = set {
                libc::ioctl(file.as_raw_fd(), libc::FS_IOC_SETFLAGS, &set);
            }
        }
        Some(current)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (file, set);
        None
    }
}

/// Credential files for one call, gone when it's dropped.
struct Credentials(PathBuf);

impl Drop for Credentials {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl System {
    pub fn new(journal: Journal, log: Option<File>) -> System {
        System {
            journal,
            log,
            secrets: None,
            vault: None,
            vault_partition: None,
        }
    }

    /// The stick's vault, for the steps that read or fill it.
    pub fn with_vault(mut self, vault: Box<dyn Store>) -> System {
        self.vault = Some(vault);
        self
    }

    /// The partition the stick's vault is on, so a step can change it.
    pub fn with_vault_partition(mut self, partition: impl Into<PathBuf>) -> System {
        self.vault_partition = Some(partition.into());
        self
    }

    /// Keeps the secrets programs read from files here instead, which only
    /// a test should want, since it may not be memory.
    pub fn keeping_secrets_in(mut self, place: PathBuf) -> System {
        self.secrets = Some(place);
        self
    }

    fn write_log(&mut self, program: &str, args: &[&str], given: &Given, ran: &Ran) {
        // A log that can't be written mustn't stop a repair, so a failed
        // write is dropped here and the log is simply shorter.
        if let Some(log) = &mut self.log {
            let _ = writeln!(log, "$ {program} {}", args.join(" "));
            // SAFETY: geteuid can't fail and touches no memory.
            if given.as_root && unsafe { libc::geteuid() } != 0 {
                let _ = writeln!(log, "-- as root, through run0");
            } else if given.as_root {
                let _ = writeln!(log, "-- as root, which it already is");
            }
            for (name, value) in &given.plain {
                let _ = writeln!(log, "-- with {name}={value}");
            }
            let places: Vec<&str> = given
                .env
                .iter()
                .chain(&given.credentials)
                .map(|(place, _)| *place)
                .chain(given.stdin.map(|_| "stdin"))
                .collect();
            if !places.is_empty() {
                let _ = writeln!(log, "-- given {}", places.join(", "));
            }
            if given.prints_secret {
                let _ = writeln!(log, "-- what it printed is secret, so it isn't kept");
            } else {
                let _ = log.write_all(&ran.stdout);
            }
            if !ran.stderr.is_empty() {
                let _ = writeln!(log, "-- it said:");
                let _ = log.write_all(&ran.stderr);
            }
            let _ = writeln!(log, "-- status {}\n", ran.status);
            let _ = log.flush();
        }
    }
}

impl Machine for System {
    fn run_given(&mut self, program: Program, args: &[&str], given: Given) -> Result<Ran, Fault> {
        let path = match program {
            Program::Tool(tool) => tool.program().ok_or(Fault::Unbuilt { tool: tool.name() })?,
            Program::Carried(path) if path.starts_with("/nix/store/") && !path.contains("/../") => {
                path
            }
            Program::Carried(path) => {
                return Err(Fault::Unstarted {
                    program: path.into(),
                    why: "a program the system carries has to be in the Nix store".into(),
                });
            }
        };
        // Root already, it runs as it is: run0 would only start a service
        // to be what this process already is.
        // SAFETY: geteuid can't fail and touches no memory.
        let through_run0 = given.as_root && unsafe { libc::geteuid() } != 0;
        if through_run0 && !given.env.is_empty() {
            return Err(Fault::Unstarted {
                program: path.into(),
                why: "a secret in its environment would be on run0's command line".into(),
            });
        }
        let credentials = if given.credentials.is_empty() {
            None
        } else {
            static CALLS: AtomicU32 = AtomicU32::new(0);
            let place = match &self.secrets {
                Some(place) => place.clone(),
                None => secret_place()?,
            }
            .join(format!(
                "credentials-{}-{}",
                std::process::id(),
                CALLS.fetch_add(1, Ordering::Relaxed)
            ));
            private_directory(&place)?;
            let credentials = Credentials(place);
            for (name, secret) in &given.credentials {
                private_file(&credentials.0.join(name), secret.expose().as_bytes())?;
            }
            Some(credentials)
        };
        let mut command = if through_run0 {
            // run0 starts it as a service of its own, which loads each
            // credential from where it's held here and is given only
            // these variables.
            let run0 = Tool::Run0.program().ok_or(Fault::Unbuilt {
                tool: Tool::Run0.name(),
            })?;
            let mut command = Command::new(run0);
            command.arg("--pipe").arg("--setenv=LC_ALL=C.UTF-8");
            for (name, value) in &given.plain {
                command.arg(format!("--setenv={name}={value}"));
            }
            if let Some(held) = &credentials {
                for (name, _) in &given.credentials {
                    command.arg(format!(
                        "--property=LoadCredential={name}:{}",
                        held.0.join(name).display()
                    ));
                }
            }
            command.arg("--").arg(path).args(args).env_clear();
            command
        } else {
            let mut command = Command::new(path);
            command.args(args).env_clear().env("LC_ALL", "C.UTF-8");
            for (name, value) in &given.plain {
                command.env(name, value);
            }
            for (name, secret) in &given.env {
                command.env(name, secret.expose());
            }
            if let Some(held) = &credentials {
                command.env("CREDENTIALS_DIRECTORY", &held.0);
            }
            command
        };
        command
            .stdin(if given.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let unstarted = |why: std::io::Error| Fault::Unstarted {
            program: path.into(),
            why: why.to_string(),
        };
        let mut child = command.spawn().map_err(unstarted)?;
        if let Some(secret) = given.stdin {
            let mut stdin = child.stdin.take().expect("a piped stdin");
            // A program that stops reading early still gets to say why.
            let _ = stdin.write_all(secret.expose().as_bytes());
        }
        let output = child.wait_with_output().map_err(unstarted)?;
        let ran = Ran {
            program: program.name().into(),
            // A program killed by a signal has no status; -1 says so.
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        };
        self.write_log(path, args, &given, &ran);
        Ok(ran)
    }

    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(why) => Err(Fault::unreadable(path, why)),
        }
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), Fault> {
        let path = Path::new(path);
        if let Some(directory) = path.parent() {
            private_directory(directory)?;
        }
        private_file(path, bytes)
    }

    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<(), Fault> {
        use std::os::unix::fs::PermissionsExt;
        let path = Path::new(path);
        if let Some(directory) = path.parent() {
            DirBuilder::new()
                .recursive(true)
                .mode(0o755)
                .create(directory)
                .map_err(unkept(directory))?;
        }
        fs::write(path, bytes).map_err(unkept(path))?;
        // A file there before keeps its mode through a write, so it's set.
        fs::set_permissions(path, fs::Permissions::from_mode(0o644)).map_err(unkept(path))
    }

    fn create(&mut self, path: &str) -> Result<Box<dyn Write + '_>, Fault> {
        let path = Path::new(path);
        if let Some(directory) = path.parent() {
            private_directory(directory)?;
        }
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(unkept(path))?;
        Ok(Box::new(std::io::BufWriter::with_capacity(1 << 20, file)))
    }

    /// As systemd's efi_set_variable does (src/basic/efivars.c): efivarfs
    /// marks most variables immutable, so the flag is dropped first and put
    /// back after, and the attributes and value go in one write to a file
    /// opened without truncating, which efivarfs doesn't do.
    fn set_efi_variable(&mut self, name: &str, value: &[u8]) -> Result<(), Fault> {
        let path = Path::new("/sys/firmware/efi/efivars").join(name);
        let refused = |why: std::io::Error| Fault::Unkept {
            path: path.display().to_string(),
            why: why.to_string(),
        };
        const IMMUTABLE: libc::c_long = 0x10;
        let held = File::open(&path).ok();
        let before = held.as_ref().and_then(|file| flags(file, None));
        if let (Some(file), Some(before)) = (&held, before) {
            flags(file, Some(before & !IMMUTABLE));
        }
        let mut buffer = 0x07u32.to_le_bytes().to_vec();
        buffer.extend_from_slice(value);
        let written = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o644)
            .open(&path)
            .and_then(|mut file| file.write_all(&buffer));
        if let (Some(file), Some(before)) = (&held, before) {
            flags(file, Some(before));
        }
        written.map_err(refused)
    }

    fn remove(&mut self, path: &str) -> Result<(), Fault> {
        match fs::remove_file(path) {
            Err(why) if why.kind() != std::io::ErrorKind::NotFound => {
                Err(unkept(Path::new(path))(why))
            }
            _ => Ok(()),
        }
    }

    fn remove_tree(&mut self, path: &str) -> Result<(), Fault> {
        match fs::remove_dir_all(path) {
            Err(why) if why.kind() != std::io::ErrorKind::NotFound => {
                Err(unkept(Path::new(path))(why))
            }
            _ => Ok(()),
        }
    }

    fn resolve(&mut self, path: &str) -> Result<Option<String>, Fault> {
        match fs::canonicalize(path) {
            Ok(found) => Ok(Some(found.to_string_lossy().into_owned())),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(why) => Err(Fault::unreadable(path, why)),
        }
    }

    fn read_link(&mut self, path: &str) -> Result<Option<String>, Fault> {
        match fs::read_link(path) {
            Ok(target) => Ok(Some(target.to_string_lossy().into_owned())),
            Err(why)
                if matches!(
                    why.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput
                ) =>
            {
                Ok(None)
            }
            Err(why) => Err(Fault::unreadable(path, why)),
        }
    }

    fn open_disk(&mut self, device: &str, mounted: &[String]) -> Result<File, Fault> {
        crate::stick::device::open(device, mounted)
    }

    /// Through its own handle, unmounted and synced before it returns. The
    /// vault's other handle only reads, so it never writes anything back
    /// over what this one wrote.
    fn change_vault(
        &mut self,
        change: &mut dyn FnMut(&mut dyn Store) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        let device = self.vault_partition.clone().ok_or(Fault::Blocked {
            step: "changing the vault",
            why: "the guide found no vault here".into(),
        })?;
        let unkept = |why: std::io::Error| Fault::Unkept {
            path: device.display().to_string(),
            why: why.to_string(),
        };
        let partition = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&device)
            .map_err(unkept)?;
        let synced = partition.try_clone().map_err(unkept)?;
        let mut fat = crate::vault::store::Fat::open(partition)?;
        change(&mut fat)?;
        fat.close()?;
        synced.sync_all().map_err(unkept)
    }

    fn read_partition(&mut self, device: &str, read: &mut dyn FnMut(&Path)) -> Result<(), Fault> {
        crate::stick::device::read_partition(device, read)
    }

    /// Exclusive, which Linux refuses for a block device something has
    /// mounted, so nothing reads it while it's changed.
    fn open_partition(&mut self, device: &str) -> Result<File, Fault> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_EXCL)
            .open(device)
            .map_err(|why| Fault::unreadable(device, why))
    }

    fn vault(&mut self) -> Result<&mut dyn Store, Fault> {
        match &mut self.vault {
            Some(vault) => Ok(vault.as_mut()),
            None => Err(Fault::Blocked {
                step: "opening the vault",
                why: "the guide found no vault here".into(),
            }),
        }
    }

    /// Streamed: each file goes from the vault to the disk a block at a
    /// time, never whole in memory.
    fn unpack(&mut self, key: VaultKey, from: &str, into: &str) -> Result<u64, Fault> {
        let mut vault = Vault::open(self.vault()?, key)?;
        let files: Vec<(String, u64)> = vault
            .files()
            .iter()
            .filter(|file| file.path.starts_with(from))
            .map(|file| (file.path.clone(), file.bytes))
            .collect();
        let mut total = 0;
        for (path, bytes) in files {
            let to = Path::new(into).join(path[from.len()..].trim_start_matches('/'));
            if let Some(directory) = to.parent() {
                private_directory(directory)?;
            }
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&to)
                .map_err(unkept(&to))?;
            let mut out = std::io::BufWriter::with_capacity(1 << 20, file);
            vault.extract(&path, &mut out)?;
            out.flush().map_err(unkept(&to))?;
            total += bytes;
        }
        Ok(total)
    }

    fn journal(&mut self) -> &mut Journal {
        &mut self.journal
    }
}
