//! The machine the guide runs on. Each tool runs by its store path with an
//! empty environment but for a fixed locale, so what it prints is what the
//! parsers were written against, and nothing the person's shell set can
//! change what runs. Every call and all it printed go to the log, raw, so
//! whoever reads it later (pi, after an enrolment) sees what the screen
//! didn't show.

use std::fs::{self, File};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::{Fault, Journal, Machine, Ran, Tool};

pub struct System {
    journal: Journal,
    log: Option<File>,
}

impl System {
    pub fn new(journal: Journal, log: Option<File>) -> System {
        System { journal, log }
    }

    fn write_log(&mut self, program: &str, args: &[&str], ran: &Ran) {
        // A log that can't be written mustn't stop a repair, so a failed
        // write is dropped here and the log is simply shorter.
        if let Some(log) = &mut self.log {
            let _ = writeln!(log, "$ {program} {}", args.join(" "));
            let _ = log.write_all(&ran.stdout);
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
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Ran, Fault> {
        let program = tool.program().ok_or(Fault::Unbuilt { tool: tool.name() })?;
        let output = Command::new(program)
            .args(args)
            .env_clear()
            .env("LC_ALL", "C.UTF-8")
            .stdin(Stdio::null())
            .output()
            .map_err(|why| Fault::Unstarted {
                program: program.into(),
                why: why.to_string(),
            })?;
        let ran = Ran {
            program: tool.name().into(),
            // A tool killed by a signal has no status; -1 says so.
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        };
        self.write_log(program, args, &ran);
        Ok(ran)
    }

    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(why) => Err(Fault::unreadable(path, why)),
        }
    }

    fn journal(&mut self) -> &mut Journal {
        &mut self.journal
    }
}
