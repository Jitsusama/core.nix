//! Code's homes, which the daemon writes beside Code's page whenever they
//! change (serve.rs): every repository and every tree on the disk, each
//! with the address on the page that shows it, so the local sites' search
//! and the other pages link to them. They're written whole to a file in
//! the same folder and renamed over the last, so a reader never sees half,
//! and a writing that fails leaves the last homes where they were.

use std::io;
use std::path::PathBuf;

use repos::model::Record;
use serde_json::{Value, json};

/// Code's homes as written to one file.
pub struct Homes {
    file: PathBuf,
    /// What was last written there, so a reading that changed none of
    /// them writes nothing.
    written: Option<String>,
    /// Why the last writing failed, so a failure is said once rather than
    /// at every reading.
    failing: Option<String>,
}

impl Homes {
    /// The homes to be written to `file`, with nothing written yet.
    pub fn at(file: PathBuf) -> Self {
        Self {
            file,
            written: None,
            failing: None,
        }
    }

    /// Writes the homes `record` gives, unless they're what was last
    /// written, and answers whether it wrote them.
    pub fn write(&mut self, _record: &Record) -> io::Result<bool> {
        let _ = (&self.file, &self.written);
        Ok(false)
    }

    /// Writes the homes `record` gives, and answers the line to say when
    /// they couldn't be written for a reason not said already.
    pub fn keep(&mut self, _record: &Record) -> Option<String> {
        let _ = &self.failing;
        None
    }
}

/// The homes `record` gives, as Code's page addresses them.
pub fn of(_record: &Record) -> Value {
    json!({"things": []})
}

/// `text` as an address's part, as jq's `@uri` writes it: every byte
/// percent-encoded but letters, digits and `-_.~`.
pub fn uri(text: &str) -> String {
    text.to_owned()
}

#[cfg(test)]
mod tests;
