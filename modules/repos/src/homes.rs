//! Code's homes, which the daemon writes beside Code's page whenever they
//! change (serve.rs): every repository and every tree on the disk, each
//! with the address on the page that shows it, so the local sites' search
//! and the other pages link to them. They're written whole to a file in
//! the same folder and renamed over the last, so a reader never sees half,
//! and a writing that fails leaves the last homes where they were.

use std::fmt::Write as _;
use std::fs::{self, File, Permissions};
use std::io::{self, Write as _};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

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
    pub fn write(&mut self, record: &Record) -> io::Result<bool> {
        let said = of(record).to_string();
        if self.written.as_ref() == Some(&said) {
            return Ok(false);
        }
        // Until this writing ends well the file may hold anything, so the
        // next one writes whatever it's given.
        self.written = None;
        let folder = match self.file.parent() {
            Some(folder) if !folder.as_os_str().is_empty() => folder,
            _ => Path::new("."),
        };
        fs::create_dir_all(folder)?;
        let name = self.file.file_name().unwrap_or_default().to_string_lossy();
        let temp = folder.join(format!(".{name}.{}", std::process::id()));
        let wrote = write_whole(&temp, &said).and_then(|()| fs::rename(&temp, &self.file));
        if let Err(why) = wrote {
            let _ = fs::remove_file(&temp);
            return Err(why);
        }
        self.written = Some(said);
        Ok(true)
    }

    /// Writes the homes `record` gives, and answers the line to say when
    /// they couldn't be written for a reason not said already.
    pub fn keep(&mut self, record: &Record) -> Option<String> {
        match self.write(record) {
            Ok(_) => {
                self.failing = None;
                None
            }
            Err(why) => {
                let why = why.to_string();
                if self.failing.as_ref() == Some(&why) {
                    return None;
                }
                let file = self.file.display();
                let said =
                    format!("repos: couldn't write Code's homes to {file} ({why}); the last stay");
                self.failing = Some(why);
                Some(said)
            }
        }
    }
}

/// `said` in a file of its own at `temp`, readable by every reader, since
/// the local sites' server reads it as another account.
fn write_whole(temp: &Path, said: &str) -> io::Result<()> {
    let mut file = File::create(temp)?;
    file.write_all(said.as_bytes())?;
    file.write_all(b"\n")?;
    file.set_permissions(Permissions::from_mode(0o644))?;
    file.sync_all()
}

/// The homes `record` gives, as Code's page addresses them.
pub fn of(record: &Record) -> Value {
    let repositories = record.repositories.iter().map(|repository| {
        let named = match &repository.owner {
            Some(owner) => format!("{owner}/{}", repository.name),
            None => repository.name.clone(),
        };
        json!({
            "kind": "repository",
            "key": repository.key,
            "name": repository.name,
            "names": [repository.name],
            "address": format!("code/#repo={}", uri(&named)),
        })
    });
    let trees = record
        .repositories
        .iter()
        .flat_map(|repository| &repository.checkouts)
        .flat_map(|checkout| &checkout.trees)
        .filter(|tree| tree.present)
        .map(|tree| {
            json!({
                "kind": "tree",
                "key": tree.path,
                "name": tree.path,
                "names": [tree.path],
                "address": format!("code/#tree={}", uri(&tree.path)),
            })
        });
    json!({"things": repositories.chain(trees).collect::<Vec<_>>()})
}

/// `text` as an address's part, as jq's `@uri` writes it: every byte
/// percent-encoded but letters, digits and `-_.~`.
pub fn uri(text: &str) -> String {
    let mut said = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            said.push(char::from(byte));
        } else {
            let _ = write!(said, "%{byte:02X}");
        }
    }
    said
}

#[cfg(test)]
mod tests;
