//! A machine for tests: every program's answer and every file is what the
//! test says, and anything it didn't say is a fault rather than a guess, so
//! a step can never reach the real machine from a test. It keeps what it
//! was asked and the secrets each program was given, for a test to check a
//! step ran what it should, handed each secret where it belongs, and did
//! nothing else.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use crate::vault::VaultKey;
use crate::vault::store::Store;
use crate::tool::Path;
use crate::{Fault, Given, Journal, Machine, Program, Ran, Tool};

/// A call as the `StandIn` keys what it answers: the program, then its
/// arguments.
type Call = (String, Vec<String>);

#[derive(Default)]
pub struct StandIn {
    answers: BTreeMap<Call, Vec<Ran>>,
    /// The files a program leaves behind when run with these arguments.
    left: BTreeMap<Call, Vec<(String, Vec<u8>)>>,
    files: BTreeMap<String, Vec<u8>>,
    absent: Vec<String>,
    links: BTreeMap<String, Option<String>>,
    pointers: BTreeMap<String, String>,
    vault: Option<Box<dyn Store>>,
    disks: BTreeMap<String, std::path::PathBuf>,
    mounts: BTreeMap<String, std::path::PathBuf>,
    journal: Journal,
    meter: crate::progress::Meter,
    /// Whether the stick is out, its vault out of reach.
    stick_out: bool,
    /// Every call made, in order, as `program arg arg`, followed by the
    /// names of the plain variables it was given in braces, the names of
    /// the secrets in brackets, and `(as root)` for one run through run0.
    pub calls: Vec<String>,
    /// Each secret given, as where it went (`PIN`, `cryptenroll.fido2-pin`,
    /// `stdin`) and what it was, in order.
    pub secrets: Vec<(String, String)>,
    /// What each program run with `run_from` read, as its call and the
    /// bytes, in order.
    pub fed: Vec<(String, Vec<u8>)>,
}

impl StandIn {
    pub fn new() -> StandIn {
        StandIn::default()
    }

    /// What the tool prints, finishing well, when run with these
    /// arguments. Said more than once, the answers are given in turn and
    /// the last one stays.
    pub fn prints(&mut self, tool: Tool, args: &[&str], stdout: &str) -> &mut StandIn {
        self.answers(Program::Tool(tool), args, 0, stdout, "")
    }

    /// What the tool says when it refuses, with these arguments.
    pub fn refuses(
        &mut self,
        tool: Tool,
        args: &[&str],
        status: i32,
        stderr: &str,
    ) -> &mut StandIn {
        self.answers(Program::Tool(tool), args, status, "", stderr)
    }

    /// Everything a program answers with these arguments: its status and
    /// all it printed, for one that prints what it can and still refuses.
    pub fn answers(
        &mut self,
        program: Program,
        args: &[&str],
        status: i32,
        stdout: &str,
        stderr: &str,
    ) -> &mut StandIn {
        self.answers
            .entry(key(program, args))
            .or_default()
            .push(Ran {
                program: program.name().into(),
                status,
                stdout: stdout.into(),
                stderr: stderr.into(),
            });
        self
    }

    /// A file the tool leaves behind when run with these arguments, as
    /// ssh-keygen leaves the keys it downloads.
    pub fn leaves(&mut self, tool: Tool, args: &[&str], path: &str, bytes: &[u8]) -> &mut StandIn {
        self.left
            .entry(key(Program::Tool(tool), args))
            .or_default()
            .push((path.into(), bytes.into()));
        self
    }

    /// A file and what it holds.
    pub fn file(&mut self, path: &str, bytes: &[u8]) -> &mut StandIn {
        self.absent.retain(|absent| absent != path);
        self.files.insert(path.into(), bytes.into());
        self
    }

    /// A file there's none of.
    pub fn no_file(&mut self, path: &str) -> &mut StandIn {
        self.files.remove(path);
        self.absent.push(path.into());
        self
    }

    /// Takes the stick out, or puts it back, its vault with it.
    pub fn stick_out(&mut self, out: bool) -> &mut StandIn {
        self.stick_out = out;
        self
    }

    /// What a file holds now, as the test said or a step wrote it.
    pub fn holds(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// Where a link leads, or that nothing's there.
    pub fn link(&mut self, path: &str, to: Option<&str>) -> &mut StandIn {
        self.links.insert(path.into(), to.map(Into::into));
        self
    }

    /// A link and what it holds, for following one step at a time.
    pub fn pointer(&mut self, path: &str, to: &str) -> &mut StandIn {
        self.pointers.insert(path.into(), to.into());
        self
    }

    /// A disk a step may open for writing, standing in for a device by a
    /// file the test made.
    pub fn disk(&mut self, device: &str, file: &std::path::Path) -> &mut StandIn {
        self.disks.insert(device.into(), file.into());
        self
    }

    /// The stick's vault, as a test made it.
    /// A partition a step may mount for reading, standing in for it by
    /// a directory.
    pub fn mount(&mut self, device: &str, directory: &std::path::Path) -> &mut StandIn {
        self.mounts.insert(device.into(), directory.into());
        self
    }

    pub fn with_vault(&mut self, vault: Box<dyn Store>) -> &mut StandIn {
        self.vault = Some(vault);
        self
    }
}

/// A file being written to the stand-in, kept once it's dropped.
struct Writing<'f> {
    stand_in: &'f mut StandIn,
    path: String,
    bytes: Vec<u8>,
}

impl Write for Writing<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for Writing<'_> {
    fn drop(&mut self) {
        let bytes = std::mem::take(&mut self.bytes);
        self.stand_in.file(&self.path, &bytes);
    }
}

fn key(program: Program, args: &[&str]) -> (String, Vec<String>) {
    let name = match program {
        Program::Tool(tool) => tool.name().to_string(),
        Program::Carried(path) => path.to_string(),
    };
    (name, args.iter().map(ToString::to_string).collect())
}

impl Machine for StandIn {
    fn run_given(&mut self, program: Program, args: &[&str], given: Given) -> Result<Ran, Fault> {
        let (name, _) = key(program, &[]);
        let mut call = std::iter::once(name.as_str())
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        if !given.plain.is_empty() {
            let names: Vec<&str> = given.plain.iter().map(|(name, _)| *name).collect();
            call = format!("{call} {{{}}}", names.join(" "));
        }
        // A cryptsetup without systemd's plugins passes over every token
        // and answers as if it had tried, so a step that leaves them out
        // would pass here and do nothing on a machine.
        if program == Program::Tool(Tool::Cryptsetup)
            && args.contains(&"--token-only")
            && !given
                .plain
                .iter()
                .any(|&(name, value)| name == "LD_LIBRARY_PATH" && Path::Tokens.value() == Some(value))
        {
            return Err(Fault::Unscripted {
                call: format!("{call} without systemd's token plugins"),
            });
        }
        let mut handed = vec![];
        for (place, secret) in given.env.iter().chain(&given.credentials) {
            handed.push(place.to_string());
            self.secrets
                .push((place.to_string(), secret.expose().into()));
        }
        if let Some(secret) = given.stdin {
            handed.push("stdin".into());
            self.secrets.push(("stdin".into(), secret.expose().into()));
        }
        if !handed.is_empty() {
            call = format!("{call} [{}]", handed.join(" "));
        }
        if given.as_root {
            call = format!("{call} (as root)");
        }
        self.calls.push(call.clone());
        let queued = self
            .answers
            .get_mut(&key(program, args))
            .ok_or(Fault::Unscripted { call })?;
        let ran = if queued.len() > 1 {
            queued.remove(0)
        } else {
            queued[0].clone()
        };
        for (path, bytes) in self
            .left
            .get(&key(program, args))
            .cloned()
            .unwrap_or_default()
        {
            self.file(&path, &bytes);
        }
        Ok(ran)
    }

    /// As `run_given` given nothing, what it printed sealed for real into
    /// the file, so a test opens it as the guide would.
    fn run_sealed(
        &mut self,
        tool: Tool,
        args: &[&str],
        key: &VaultKey,
        path: &str,
    ) -> Result<Ran, Fault> {
        let mut ran = self.run_given(Program::Tool(tool), args, Given::default())?;
        let printed = std::mem::take(&mut ran.stdout);
        let mut sealing = key.sealing(vec![])?;
        let sealed = sealing
            .write_all(&printed)
            .and_then(|()| sealing.finish())
            .map_err(|why| Fault::Unkept {
                path: path.into(),
                why: why.to_string(),
            })?;
        self.calls.push(format!("writing {path}"));
        self.file(path, &sealed);
        Ok(ran)
    }

    /// As `run_given` given nothing, the file opened for real and what it
    /// held kept in `fed`.
    fn run_unsealed(
        &mut self,
        tool: Tool,
        args: &[&str],
        key: &VaultKey,
        path: &str,
    ) -> Result<Ran, Fault> {
        let sealed = self
            .read(path)?
            .ok_or_else(|| Fault::unreadable(path, "it isn't there"))?;
        let mut ran = self.run_given(Program::Tool(tool), args, Given::default())?;
        let mut held = vec![];
        key.unsealing(sealed.as_slice())?
            .read_to_end(&mut held)
            .map_err(|why| Fault::unreadable(path, why))?;
        let call = self.calls.last().cloned().unwrap_or_default();
        self.fed.push((call, held));
        ran.stdout.clear();
        Ok(ran)
    }

    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        if let Some(bytes) = self.files.get(path) {
            Ok(Some(bytes.clone()))
        } else if self.absent.iter().any(|absent| absent == path) {
            Ok(None)
        } else {
            Err(Fault::Unscripted {
                call: format!("reading {path}"),
            })
        }
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), Fault> {
        self.calls.push(format!("writing {path}"));
        self.file(path, bytes);
        Ok(())
    }

    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<(), Fault> {
        self.calls.push(format!("publishing {path}"));
        self.file(path, bytes);
        Ok(())
    }

    fn create(&mut self, path: &str) -> Result<Box<dyn Write + '_>, Fault> {
        self.calls.push(format!("writing {path}"));
        Ok(Box::new(Writing {
            stand_in: self,
            path: path.into(),
            bytes: vec![],
        }))
    }

    fn set_efi_variable(&mut self, name: &str, value: &[u8]) -> Result<(), Fault> {
        self.calls.push(format!("setting the firmware's {name}"));
        let mut stored = vec![0x07, 0, 0, 0];
        stored.extend_from_slice(value);
        self.file(&format!("/sys/firmware/efi/efivars/{name}"), &stored);
        Ok(())
    }

    fn remove(&mut self, path: &str) -> Result<(), Fault> {
        self.calls.push(format!("removing {path}"));
        self.no_file(path);
        Ok(())
    }

    fn remove_tree(&mut self, path: &str) -> Result<(), Fault> {
        self.calls.push(format!("removing {path} and all it holds"));
        let inside = format!("{path}/");
        self.files.retain(|file, _| !file.starts_with(&inside));
        Ok(())
    }

    fn resolve(&mut self, path: &str) -> Result<Option<String>, Fault> {
        self.links
            .get(path)
            .cloned()
            .ok_or_else(|| Fault::Unscripted {
                call: format!("resolving {path}"),
            })
    }

    fn read_link(&mut self, path: &str) -> Result<Option<String>, Fault> {
        Ok(self.pointers.get(path).cloned())
    }

    fn meter(&mut self) -> crate::progress::Meter {
        self.meter.clone()
    }

    fn vault(&mut self) -> Result<&mut dyn Store, Fault> {
        if self.stick_out {
            return Err(Fault::StickOut);
        }
        match &mut self.vault {
            Some(vault) => Ok(vault.as_mut()),
            None => Err(Fault::Unscripted {
                call: "opening the vault".into(),
            }),
        }
    }

    fn change_vault(
        &mut self,
        change: &mut dyn FnMut(&mut dyn Store) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        self.calls.push("changing the vault".into());
        change(self.vault()?)
    }

    fn read_partition(
        &mut self,
        device: &str,
        read: &mut dyn FnMut(&std::path::Path),
    ) -> Result<(), Fault> {
        self.calls.push(format!("mounting {device} for reading"));
        let directory = self.mounts.get(device).ok_or_else(|| Fault::Unscripted {
            call: format!("mounting {device}"),
        })?;
        read(directory);
        self.calls.push(format!("unmounting {device}"));
        Ok(())
    }

    fn open_partition(&mut self, device: &str) -> Result<std::fs::File, Fault> {
        self.calls
            .push(format!("opening {device} to change in place"));
        let file = self.disks.get(device).ok_or_else(|| Fault::Unscripted {
            call: format!("opening {device}"),
        })?;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(file)
            .map_err(|why| Fault::unreadable(device, why))
    }

    fn open_disk(&mut self, device: &str, mounted: &[String]) -> Result<std::fs::File, Fault> {
        self.calls.push(format!(
            "opening {device} for writing, unmounting [{}]",
            mounted.join(" ")
        ));
        let file = self.disks.get(device).ok_or_else(|| Fault::Unscripted {
            call: format!("opening {device}"),
        })?;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(file)
            .map_err(|why| Fault::unreadable(device, why))
    }

    fn journal(&mut self) -> &mut Journal {
        &mut self.journal
    }
}
