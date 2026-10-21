//! A machine for tests: every program's answer and every file is what the
//! test says, and anything it didn't say is a fault rather than a guess, so
//! a step can never reach the real machine from a test. It keeps what it
//! was asked and the secrets each program was given, for a test to check a
//! step ran what it should, handed each secret where it belongs, and did
//! nothing else.

use std::collections::BTreeMap;
use std::io::Write;

use crate::vault::store::Store;
use crate::{Fault, Given, Journal, Machine, Program, Ran, Tool};

#[derive(Default)]
pub struct Fake {
    answers: BTreeMap<(String, Vec<String>), Vec<Ran>>,
    files: BTreeMap<String, Vec<u8>>,
    absent: Vec<String>,
    links: BTreeMap<String, Option<String>>,
    pointers: BTreeMap<String, String>,
    vault: Option<Box<dyn Store>>,
    disks: BTreeMap<String, std::path::PathBuf>,
    journal: Journal,
    /// Every call made, in order, as `program arg arg`, followed by the
    /// names of the secrets it was given in brackets, and `(as root)` for
    /// one run through run0.
    pub calls: Vec<String>,
    /// Each secret given, as where it went (`PIN`, `cryptenroll.fido2-pin`,
    /// `stdin`) and what it was, in order.
    pub secrets: Vec<(String, String)>,
}

impl Fake {
    pub fn new() -> Fake {
        Fake::default()
    }

    /// What the tool prints, finishing well, when run with these
    /// arguments. Said more than once, the answers are given in turn and
    /// the last one stays.
    pub fn prints(&mut self, tool: Tool, args: &[&str], stdout: &str) -> &mut Fake {
        self.answers(Program::Tool(tool), args, 0, stdout, "")
    }

    /// What the tool says when it refuses, with these arguments.
    pub fn refuses(&mut self, tool: Tool, args: &[&str], status: i32, stderr: &str) -> &mut Fake {
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
    ) -> &mut Fake {
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

    /// A file and what it holds.
    pub fn file(&mut self, path: &str, bytes: &[u8]) -> &mut Fake {
        self.absent.retain(|absent| absent != path);
        self.files.insert(path.into(), bytes.into());
        self
    }

    /// A file there's none of.
    pub fn no_file(&mut self, path: &str) -> &mut Fake {
        self.files.remove(path);
        self.absent.push(path.into());
        self
    }

    /// What a file holds now, as the test said or a step wrote it.
    pub fn holds(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// Where a link leads, or that nothing's there.
    pub fn link(&mut self, path: &str, to: Option<&str>) -> &mut Fake {
        self.links.insert(path.into(), to.map(Into::into));
        self
    }

    /// A link and what it holds, for following one step at a time.
    pub fn pointer(&mut self, path: &str, to: &str) -> &mut Fake {
        self.pointers.insert(path.into(), to.into());
        self
    }

    /// A disk a step may open for writing, standing in for a device by a
    /// file the test made.
    pub fn disk(&mut self, device: &str, file: &std::path::Path) -> &mut Fake {
        self.disks.insert(device.into(), file.into());
        self
    }

    /// The stick's vault, as a test made it.
    pub fn with_vault(&mut self, vault: Box<dyn Store>) -> &mut Fake {
        self.vault = Some(vault);
        self
    }
}

/// A file being written to the fake, kept once it's dropped.
struct Writing<'f> {
    fake: &'f mut Fake,
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
        self.fake.file(&self.path, &bytes);
    }
}

fn key(program: Program, args: &[&str]) -> (String, Vec<String>) {
    let name = match program {
        Program::Tool(tool) => tool.name().to_string(),
        Program::Carried(path) => path.to_string(),
    };
    (name, args.iter().map(|arg| arg.to_string()).collect())
}

impl Machine for Fake {
    fn run_given(&mut self, program: Program, args: &[&str], given: Given) -> Result<Ran, Fault> {
        let (name, _) = key(program, &[]);
        let mut call = std::iter::once(name.as_str())
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
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
        Ok(if queued.len() > 1 {
            queued.remove(0)
        } else {
            queued[0].clone()
        })
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

    fn create(&mut self, path: &str) -> Result<Box<dyn Write + '_>, Fault> {
        self.calls.push(format!("writing {path}"));
        Ok(Box::new(Writing {
            fake: self,
            path: path.into(),
            bytes: vec![],
        }))
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

    fn vault(&mut self) -> Result<&mut dyn Store, Fault> {
        match &mut self.vault {
            Some(vault) => Ok(vault.as_mut()),
            None => Err(Fault::Unscripted {
                call: "opening the vault".into(),
            }),
        }
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
