//! Code's homes, which the daemon writes beside Code's page whenever they
//! change (serve.rs): every repository and every tree on the disk, each
//! with the address on the page that shows it, so the local sites' search
//! and the other pages link to them. They're written whole to a file in
//! the same folder and renamed over the last, so a reader never sees half,
//! and a writing that fails leaves the last homes where they were. Each
//! reading compares them with the file itself, so homes another writer
//! changed or removed are put right at the next.

use std::fmt::Write as _;
use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write as _};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use repos::model::Record;
use serde_json::{Value, json};

/// Code's homes as written to one file.
pub struct Homes {
    file: PathBuf,
    /// Why the last writing failed, so a failure is said once rather than
    /// at every reading.
    failing: Option<String>,
}

impl Homes {
    /// The homes to be written to `file`, with nothing written yet.
    pub fn at(file: PathBuf) -> Self {
        Self {
            file,
            failing: None,
        }
    }

    /// Writes the homes `record` gives, unless the file holds them
    /// already, and answers whether it wrote them.
    pub fn write(&mut self, record: &Record) -> io::Result<bool> {
        let said = format!("{}\n", of(record));
        if fs::read_to_string(&self.file).is_ok_and(|held| held == said) {
            return Ok(false);
        }
        let folder = match self.file.parent() {
            Some(folder) if !folder.as_os_str().is_empty() => folder,
            _ => Path::new("."),
        };
        fs::create_dir_all(folder)?;
        let name = self.file.file_name().unwrap_or_default().to_string_lossy();
        // Hidden, so the server never answers with it, and the same name
        // each time, so a writing a killed daemon left half-done goes at
        // the next. One daemon an account writes it.
        let temp = folder.join(format!(".{name}.writing"));
        let wrote = write_whole(&temp, &said).and_then(|()| fs::rename(&temp, &self.file));
        if let Err(why) = wrote {
            let _ = fs::remove_file(&temp);
            return Err(why);
        }
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

/// `said` in a file of its own at `temp`: made anew, so whatever was left
/// there, a link included, is never written through, and private until
/// it's whole. Then it's open to every reader, as the documents' other
/// files are, whatever umask the daemon started with.
fn write_whole(temp: &Path, said: &str) -> io::Result<()> {
    match fs::remove_file(temp) {
        Err(why) if why.kind() != io::ErrorKind::NotFound => return Err(why),
        _ => {}
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(temp)?;
    file.write_all(said.as_bytes())?;
    file.set_permissions(Permissions::from_mode(0o644))?;
    file.sync_all()
}

/// The homes `record` gives, as Code's page addresses them.
pub fn of(record: &Record) -> Value {
    // By their keys, not the reading's newest first, so a commit that only
    // reorders the record changes no homes.
    let mut repositories: Vec<_> = record.repositories.iter().collect();
    repositories.sort_by(|a, b| a.key.cmp(&b.key));
    let repositories = repositories.into_iter().map(|repository| {
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
    let mut trees: Vec<_> = record
        .repositories
        .iter()
        .flat_map(|repository| &repository.checkouts)
        .flat_map(|checkout| &checkout.trees)
        .filter(|tree| tree.present)
        .collect();
    trees.sort_by(|a, b| a.path.cmp(&b.path));
    let trees = trees.into_iter().map(|tree| {
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
