//! What a daemon decides of its own, whichever releases through the
//! engine: what it may release, how a value is read and held, and what
//! command it runs in.

use std::path::PathBuf;
use std::process::Command;

use crate::ask::Ask;

/// A value read to be released, its bytes zeroed once it's let go.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Secret {
        Secret(value)
    }

    /// The value, for the command it's released to alone.
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: zeroes are valid UTF-8, so the string stays a string.
        unsafe { self.0.as_bytes_mut() }.fill(0);
    }
}

/// What a daemon decides of its own: what it may release, whether the
/// session is locked, how a value is read, and what command it runs in.
pub trait Releaser: Send + Sync {
    /// Why an ask may not be served at all, where it may not; it's then
    /// answered refused, unseen.
    fn refused(&self, ask: &Ask) -> Option<String>;
    /// Whether the session in front is locked, when no yes is taken.
    fn locked(&self) -> bool;
    /// The value an ask names, read once Joel has said yes.
    ///
    /// # Errors
    ///
    /// Where it can't be read, saying why.
    fn read_value(&self, ask: &Ask) -> Result<Secret, String>;
    /// The command an ask runs, given the value; its standard input,
    /// output and error are the engine's to set.
    ///
    /// # Errors
    ///
    /// Where it can't be made, saying why.
    fn command(&self, ask: &Ask, value: &Secret) -> Result<Command, String>;
    /// The program an ask's command comes to, as this releaser resolves
    /// it, which a standing yes is to as much as the command's words;
    /// none by default, so a releaser that resolves none makes no grant.
    fn program(&self, ask: &Ask) -> Option<PathBuf> {
        let _ = ask;
        None
    }
    /// What to tell the agent of what the releaser did once a release was
    /// answered, its command run or not, a sentence each; nothing by
    /// default.
    fn after(&self, ask: &Ask) -> Vec<String> {
        let _ = ask;
        Vec::new()
    }
}
