//! What stops a step, said so a person knows what happened and where to
//! look. Every tool's own words are in the log; a fault carries the last of
//! them so the screen can show why without the log.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// A tool ran and said no.
    Refused {
        program: String,
        status: i32,
        said: String,
    },
    /// A tool couldn't be started at all.
    Unstarted { program: String, why: String },
    /// This build of the guide has no path for a tool it needs.
    Unbuilt { tool: &'static str },
    /// What a tool or a file said isn't what the guide reads.
    Unreadable { what: String, why: String },
    /// A file the guide keeps couldn't be read or written.
    Unkept { path: String, why: String },
    /// Something sealed wouldn't open: the wrong key, or it was changed.
    Unopened { what: String, why: String },
    /// A step ran, and observing it again says it isn't done.
    Unproven {
        step: &'static str,
        standing: String,
    },
    /// A step can't run here and now.
    Blocked { step: &'static str, why: String },
    /// A step was asked to run without everything it asks for.
    Unanswered {
        step: &'static str,
        ask: &'static str,
    },
    /// A stand-in machine was asked for something its test never told it.
    Unscripted { call: String },
    /// What's next needs the stick, and it's out.
    StickOut,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Refused {
                program,
                status,
                said,
            } => write!(f, "{program} stopped with status {status}: {said}"),
            Fault::Unstarted { program, why } => write!(f, "{program} couldn't start: {why}"),
            Fault::Unbuilt { tool } => write!(f, "this guide was built without {tool}"),
            Fault::Unreadable { what, why } => write!(f, "couldn't read {what}: {why}"),
            Fault::Unkept { path, why } => write!(f, "couldn't keep {path}: {why}"),
            Fault::Unopened { what, why } => write!(f, "{what} wouldn't open: {why}"),
            Fault::Unproven { step, standing } => {
                write!(
                    f,
                    "{step} ran, but the machine says it isn't done: {standing}"
                )
            }
            Fault::Blocked { step, why } => write!(f, "{step} can't run: {why}"),
            Fault::Unanswered { step, ask } => write!(f, "{step} needs {ask} before it runs"),
            Fault::Unscripted { call } => write!(f, "the test never said what {call} answers"),
            Fault::StickOut => write!(f, "the stick isn't plugged in"),
        }
    }
}

impl std::error::Error for Fault {}

impl Fault {
    /// What happened, said under the step's own title, so it doesn't name
    /// the step again by its id, which the log and the journal use.
    pub fn said(&self) -> String {
        match self {
            Fault::Unproven { .. } => {
                "it finished, but looking at the machine again shows it isn't done".into()
            }
            Fault::Blocked { why, .. } => why.clone(),
            Fault::Unanswered { .. } => "it needs an answer it wasn't given".into(),
            Fault::StickOut => "this needs what's on the stick, and the stick is out".into(),
            fault => fault.to_string(),
        }
    }

    /// Something a tool or a file said that the guide can't read.
    pub fn unreadable(what: impl Into<String>, why: impl fmt::Display) -> Fault {
        Fault::Unreadable {
            what: what.into(),
            why: why.to_string(),
        }
    }
}
