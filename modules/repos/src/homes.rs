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
use std::io::{self, Read as _, Write as _};
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
        if holds(&self.file, &said) {
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
        // Named, since the hidden file is one a reader of the folder won't see.
        let wrote = write_whole(&temp, &said)
            .map_err(|why| io::Error::new(why.kind(), format!("{why}, at {}", temp.display())))
            .and_then(|()| fs::rename(&temp, &self.file));
        if let Err(why) = wrote {
            let _ = fs::remove_file(&temp);
            return Err(why);
        }
        Ok(true)
    }

    /// Writes the homes `record` gives, and answers the line to say when
    /// they couldn't be written for a reason not said already, or are right
    /// again after failing, written or put right by another writer, so a
    /// failure's line is never left reading as though it still held.
    pub fn keep(&mut self, record: &Record) -> Option<String> {
        match self.write(record) {
            Ok(_) => {
                let file = self.file.display();
                self.failing
                    .take()
                    .map(|_| format!("repos: Code's homes at {file} are right again"))
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

/// Whether `file` holds `said` already: only a plain file of its length is
/// read, so a pipe or a link to a device there never holds the reading
/// loop, and is put right like any other homes gone wrong. It's opened
/// once, following no link and waiting on no pipe, and what was opened is
/// what's judged, so nothing swapped in after a look can hold it either,
/// and a terminal there never becomes the daemon's.
fn holds(file: &Path, said: &str) -> bool {
    let Ok(held) = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(file)
    else {
        return false;
    };
    let length = said.len() as u64;
    let plain = held
        .metadata()
        .is_ok_and(|kind| kind.is_file() && kind.len() == length);
    // A byte past the homes at most, should another writer be growing it.
    let mut text = String::new();
    plain && held.take(length + 1).read_to_string(&mut text).is_ok() && text == said
}

/// `said` in a file of its own at `temp`: made anew, so whatever was left
/// there, a link included, is never written through, and private until
/// it's whole. Then it's open to every reader, as the documents' other
/// files are, whatever umask the daemon started with. It isn't flushed to
/// the disk: a start compares the homes with the file and writes them
/// again, so a machine that lost power serves its homes again from the
/// first reading, and a flush would hold the reading loop behind
/// everyone else's writes.
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
    file.set_permissions(Permissions::from_mode(0o644))
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
