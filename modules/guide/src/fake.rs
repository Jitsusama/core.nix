//! A machine for tests: every tool's answer and every file is what the
//! test says, and anything it didn't say is a fault rather than a guess, so
//! a step can never reach the real machine from a test. It keeps what it
//! was asked, for a test to check a step ran what it should and nothing
//! else.

use std::collections::BTreeMap;

use crate::{Fault, Journal, Machine, Ran, Tool};

#[derive(Default)]
pub struct Fake {
    answers: BTreeMap<(Tool, Vec<String>), Vec<Ran>>,
    files: BTreeMap<String, Vec<u8>>,
    absent: Vec<String>,
    journal: Journal,
    /// Every call made, in order, as `tool arg arg`.
    pub calls: Vec<String>,
}

impl Fake {
    pub fn new() -> Fake {
        Fake::default()
    }

    /// What the tool prints, finishing well, when run with these
    /// arguments. Said more than once, the answers are given in turn and
    /// the last one stays.
    pub fn prints(&mut self, tool: Tool, args: &[&str], stdout: &str) -> &mut Fake {
        self.answers(tool, args, 0, stdout, "")
    }

    /// What the tool says when it refuses, with these arguments.
    pub fn refuses(&mut self, tool: Tool, args: &[&str], status: i32, stderr: &str) -> &mut Fake {
        self.answers(tool, args, status, "", stderr)
    }

    /// Everything a tool answers with these arguments: its status and all
    /// it printed, for a tool that prints what it can and still refuses.
    pub fn answers(
        &mut self,
        tool: Tool,
        args: &[&str],
        status: i32,
        stdout: &str,
        stderr: &str,
    ) -> &mut Fake {
        self.answers
            .entry((tool, args.iter().map(|arg| arg.to_string()).collect()))
            .or_default()
            .push(Ran {
                program: tool.name().into(),
                status,
                stdout: stdout.into(),
                stderr: stderr.into(),
            });
        self
    }

    /// A file and what it holds.
    pub fn file(&mut self, path: &str, bytes: &[u8]) -> &mut Fake {
        self.files.insert(path.into(), bytes.into());
        self
    }

    /// A file there's none of.
    pub fn no_file(&mut self, path: &str) -> &mut Fake {
        self.absent.push(path.into());
        self
    }
}

impl Machine for Fake {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Ran, Fault> {
        let call = std::iter::once(tool.name())
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        self.calls.push(call.clone());
        let key = (tool, args.iter().map(|arg| arg.to_string()).collect());
        let queued = self
            .answers
            .get_mut(&key)
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

    fn journal(&mut self) -> &mut Journal {
        &mut self.journal
    }
}
