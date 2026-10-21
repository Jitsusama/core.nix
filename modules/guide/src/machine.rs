//! The machine a step observes and acts on: the programs it runs, the
//! files it reads, and the journal of what can't be observed. The real one
//! is `system::System`; tests give steps `fake::Fake`.

use crate::{Fault, Journal, Tool};

pub trait Machine {
    /// Runs a tool with these arguments and says what it answered, whatever
    /// its status. `Ran::ok` turns a refusal into a fault.
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Ran, Fault>;

    /// A file's bytes, or nothing when there's no such file.
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, Fault>;

    /// What earlier steps recorded that observing can't show.
    fn journal(&mut self) -> &mut Journal;
}

/// What a tool answered.
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
            Err(Fault::Refused {
                program: self.program,
                status: self.status,
                said: last_line(&self.stderr),
            })
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
}
