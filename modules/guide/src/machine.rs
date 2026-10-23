//! The machine a step observes and acts on: the programs it runs, the
//! files it reads and writes, and the journal of what can't be observed.
//! The real one is `system::System`; tests give steps `fake::Fake`.

use std::io::Write;

use crate::vault::store::Store;
use crate::vault::{Vault, VaultKey};
use crate::{Fault, Journal, Secret, Tool};

/// What a step runs: one of the guide's own tools, or a program the
/// system being installed carries, by its store path, such as that
/// machine's disko script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program<'a> {
    Tool(Tool),
    Carried(&'a str),
}

impl Program<'_> {
    /// How the log and a fault name it.
    pub fn name(&self) -> &str {
        match self {
            Program::Tool(tool) => tool.name(),
            Program::Carried(path) => path.rsplit('/').next().unwrap_or(path),
        }
    }
}

/// Secrets a program is given, each the way that program takes it, never
/// as an argument, so no process list or log shows one.
#[derive(Default)]
pub struct Given<'a> {
    /// Environment variables that aren't secret, logged as they are: the
    /// disk tools a disk script finds on `PATH`, cryptsetup's plugins.
    pub plain: Vec<(&'static str, &'a str)>,
    /// Environment variables, as systemd's tools read `PIN`, `NEWPIN` and
    /// `PASSWORD`.
    pub env: Vec<(&'static str, &'a Secret)>,
    /// Files in `$CREDENTIALS_DIRECTORY`, as systemd-cryptenroll reads a
    /// FIDO2 enrolment PIN from `cryptenroll.fido2-pin`. They're kept in
    /// memory and gone once the program is.
    pub credentials: Vec<(&'static str, &'a Secret)>,
    /// What it reads on its standard input.
    pub stdin: Option<&'a Secret>,
    /// What it prints is itself a secret, such as a recovery key, so the
    /// log leaves it out.
    pub prints_secret: bool,
    /// It runs as root, through run0, which asks polkit, for what a person
    /// on an installed machine can't do as themself, such as enrolling the
    /// TPM. Its secrets go only as credentials and standard input, since
    /// an environment variable would be on run0's command line.
    pub as_root: bool,
}

pub trait Machine {
    /// Runs a program with these arguments and the secrets it's given, and
    /// says what it answered, whatever its status. `Ran::ok` turns a
    /// refusal into a fault.
    fn run_given(&mut self, program: Program, args: &[&str], given: Given) -> Result<Ran, Fault>;

    /// Runs a tool with these arguments and nothing else.
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Ran, Fault> {
        self.run_given(Program::Tool(tool), args, Given::default())
    }

    /// A file's bytes, or nothing when there's no such file.
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, Fault>;

    /// Keeps a file only its owner can read, its directories made only
    /// they can enter. Anything secret goes only where it's held in memory.
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), Fault>;

    /// Keeps a file anyone can read, for what's sealed or public, such as
    /// a rescue bundle, which the person reads without asking for root.
    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<(), Fault>;

    /// A file to write a lot to, such as a cache unpacked from the vault,
    /// kept as `write` keeps one.
    fn create(&mut self, path: &str) -> Result<Box<dyn Write + '_>, Fault>;

    /// Sets a firmware variable through efivarfs, its value after the
    /// attributes systemd gives the boot loader's own: non-volatile, and
    /// read at boot and at run time.
    fn set_efi_variable(&mut self, name: &str, value: &[u8]) -> Result<(), Fault>;

    /// Takes a file away; one already gone is no fault.
    fn remove(&mut self, path: &str) -> Result<(), Fault>;

    /// Takes a directory away with all it holds, such as a cache unpacked
    /// for an install; one already gone is no fault.
    fn remove_tree(&mut self, path: &str) -> Result<(), Fault>;

    /// Where a link leads in the end, or nothing when there's nothing
    /// there.
    fn resolve(&mut self, path: &str) -> Result<Option<String>, Fault>;

    /// What a link holds, one step, or nothing when it isn't a link.
    fn read_link(&mut self, path: &str) -> Result<Option<String>, Fault>;

    /// Where a link leads in the end inside a system mounted at `root`,
    /// whose absolute links mean that system's own paths, not this one's,
    /// as that system names it. Whether anything's there isn't asked.
    fn resolve_inside(&mut self, root: &str, path: &str) -> Result<Option<String>, Fault> {
        let mut at = path.to_string();
        for _ in 0..40 {
            let Some(target) = self.read_link(&at)? else {
                return Ok(Some(at.strip_prefix(root).unwrap_or(&at).to_string()));
            };
            at = if target.starts_with('/') {
                format!("{root}{target}")
            } else {
                let directory = at.rsplit_once('/').map_or("", |(directory, _)| directory);
                format!("{directory}/{target}")
            };
        }
        Err(Fault::unreadable(path, "its links go round in a loop"))
    }

    /// The stick's vault, its files sealed.
    fn vault(&mut self) -> Result<&mut dyn Store, Fault>;

    /// Copies the first `bytes` of a file or a device into a file kept as
    /// `write` keeps one, counting them on the meter as they go.
    fn copy(&mut self, from: &str, to: &str, bytes: u64) -> Result<(), Fault> {
        let mut read = self
            .read(from)?
            .ok_or_else(|| Fault::unreadable(from, "it isn't there"))?;
        if (read.len() as u64) < bytes {
            return Err(Fault::unreadable(from, "it's shorter than what's copied"));
        }
        read.truncate(usize::try_from(bytes).expect("no more than was read"));
        self.write(to, &read)?;
        self.meter().advance(bytes);
        Ok(())
    }

    /// The meter long work feeds as it goes, which the screen reads while
    /// a step runs; by default one nothing feeds, so the screen says only
    /// how long the step has been running.
    fn meter(&mut self) -> crate::progress::Meter {
        crate::progress::Meter::new()
    }

    /// A stick's whole disk, for writing, once what was mounted from it
    /// (its partitions' devices) is unmounted.
    fn open_disk(&mut self, device: &str, mounted: &[String]) -> Result<std::fs::File, Fault>;

    /// Changes the stick's vault, all of it written back to the stick
    /// before this returns, since the machine may be shut down or the stick
    /// pulled straight after.
    fn change_vault(
        &mut self,
        change: &mut dyn FnMut(&mut dyn Store) -> Result<(), Fault>,
    ) -> Result<(), Fault>;

    /// A stick's partition, mounted by the person only for reading while
    /// `read` looks inside it and unmounted after, such as to see what a
    /// stick carries before it's written over.
    fn read_partition(
        &mut self,
        device: &str,
        read: &mut dyn FnMut(&std::path::Path),
    ) -> Result<(), Fault>;

    /// A partition nothing has mounted, opened as root for changing in
    /// place, such as the stick's own EFI system partition.
    fn open_partition(&mut self, device: &str) -> Result<std::fs::File, Fault>;

    /// Unpacks every file the vault holds under `from` into the directory
    /// `into`, each checked as it's extracted, and says how many bytes.
    /// One call, since a cache is gigabytes and goes straight to the disk.
    fn unpack(&mut self, key: VaultKey, from: &str, into: &str) -> Result<u64, Fault> {
        let mut held = vec![];
        {
            let mut vault = Vault::open(self.vault()?, key)?;
            let paths: Vec<String> = vault
                .files()
                .iter()
                .filter(|file| file.path.starts_with(from))
                .map(|file| file.path.clone())
                .collect();
            for path in paths {
                let mut bytes = vec![];
                vault.extract(&path, &mut bytes)?;
                held.push((path, bytes));
            }
        }
        let mut total = 0;
        for (path, bytes) in held {
            let to = format!("{into}/{}", path[from.len()..].trim_start_matches('/'));
            self.write(&to, &bytes)?;
            total += bytes.len() as u64;
        }
        Ok(total)
    }

    /// What earlier steps recorded that observing can't show.
    fn journal(&mut self) -> &mut Journal;
}

/// What a program answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    pub program: String,
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Ran {
    /// What it printed, when it finished well; otherwise why it didn't.
    pub fn ok(self) -> Result<String, Fault> {
        if self.status == 0 {
            String::from_utf8(self.stdout).map_err(|_| {
                Fault::unreadable(format!("what {} printed", self.program), "not UTF-8")
            })
        } else {
            Err(self.refusal())
        }
    }

    /// Why it stopped, from the last it said.
    pub fn refusal(&self) -> Fault {
        Fault::Refused {
            program: self.program.clone(),
            status: self.status,
            said: last_line(&self.stderr),
        }
    }
}

/// The last line a tool wrote, which is where most say why they stopped.
fn last_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("it said nothing")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ran(status: i32, stdout: &str, stderr: &str) -> Ran {
        Ran {
            program: "sbctl".into(),
            status,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    #[test]
    fn a_tool_that_finished_well_gives_what_it_printed() {
        assert_eq!(ran(0, "fine\n", "").ok(), Ok("fine\n".into()));
    }

    #[test]
    fn a_refusal_carries_the_last_line_the_tool_wrote() {
        assert_eq!(
            ran(1, "", "reading keys\nno keys found\n\n").ok(),
            Err(Fault::Refused {
                program: "sbctl".into(),
                status: 1,
                said: "no keys found".into(),
            })
        );
    }

    #[test]
    fn a_silent_refusal_says_so() {
        let Err(Fault::Refused { said, .. }) = ran(2, "", "").ok() else {
            panic!("a refusal")
        };
        assert_eq!(said, "it said nothing");
    }

    #[test]
    fn a_profile_is_followed_inside_the_system_it_belongs_to() {
        let mut fake = crate::fake::Fake::new();
        // An absolute link to another link, which only the installed
        // system's root holds.
        fake.pointer("/mnt/nix/var/nix/profiles/system", "system-1-link")
            .pointer(
                "/mnt/nix/var/nix/profiles/system-1-link",
                "/nix/var/nix/gcroots/booted",
            )
            .pointer(
                "/mnt/nix/var/nix/gcroots/booted",
                "/nix/store/abc-nixos-system-machine",
            );
        assert_eq!(
            fake.resolve_inside("/mnt", "/mnt/nix/var/nix/profiles/system")
                .unwrap()
                .as_deref(),
            Some("/nix/store/abc-nixos-system-machine")
        );
    }

    #[test]
    fn a_carried_program_is_named_by_its_file() {
        assert_eq!(
            Program::Carried("/nix/store/abc-disko/bin/disko-format").name(),
            "disko-format"
        );
    }
}
