//! A repository's tags as the engine keeps them between answers: each by
//! name with the commit it peels to, read from a files repository's packed
//! and loose refs or given from a reftable's, and settled while the files
//! they came from hold.

use crate::hash::Oid;
use crate::odb::{Kind, Odb};
use crate::stamp::{Stamp, Stamps};
use std::collections::HashMap;
use std::path::Path;

/// Every tag by name with the commit it comes to, and the stamps of what
/// they were read from, which settle them while they hold.
pub(crate) struct Tags {
    pub(crate) stamps: Stamps,
    /// Sorted by name, each with the commit it peels to, or where peeling
    /// it stopped.
    named: Vec<(String, Oid)>,
    /// Where in `named` a tag isn't peeled yet, peeled as an answer is made;
    /// one whose object wouldn't read is peeled again each answer, alone,
    /// as the store marks what it holds: a packed copy marked bad answers
    /// from its mark, and an I/O error, a base's fault or a loose copy is
    /// read afresh, which may find it written since.
    unpeeled: Vec<usize>,
}

/// How far an annotated tag peels.
enum Peeled {
    /// To what isn't a tag object: the commit, most often.
    To(Oid),
    /// To a tag object no store holds.
    Missing(Oid),
    /// To a tag object that wouldn't read.
    Unread(Oid),
}

impl Tags {
    /// The tags `named`, read with `stamps`, a packed annotated tag taking
    /// what `peeled` says it peels to; the rest are peeled as they're asked.
    pub(crate) fn new(
        stamps: Stamps,
        mut named: Vec<(String, Oid)>,
        peeled: &HashMap<String, Oid>,
    ) -> Tags {
        named.sort();
        let mut unpeeled = Vec::new();
        for (at, (name, oid)) in named.iter_mut().enumerate() {
            match peeled.get(name) {
                Some(target) => *oid = *target,
                None => unpeeled.push(at),
            }
        }
        Tags {
            stamps,
            named,
            unpeeled,
        }
    }

    /// The first tag by name pointing at `commit`, once the tags not yet
    /// peeled are peeled from where they stopped.
    pub(crate) fn naming(&mut self, commit: Oid, odb: &mut Odb) -> String {
        self.peel(odb);
        self.named
            .iter()
            .find(|(_, peeled)| *peeled == commit)
            .map(|(name, _)| name.clone())
            .unwrap_or_default()
    }

    /// Peels each tag not yet peeled: one whose object is missing settles
    /// with the stamps of where it would come from, rather than be looked
    /// for every answer, and one whose object wouldn't read is left for the
    /// next.
    fn peel(&mut self, odb: &mut Odb) {
        let mut missing = false;
        for at in std::mem::take(&mut self.unpeeled) {
            self.named[at].1 = match peel(odb, self.named[at].1) {
                Peeled::To(oid) => oid,
                Peeled::Missing(oid) => {
                    // Tags in one fan-out folder share its stamp.
                    for (path, stamp) in odb.folders_of(&oid) {
                        if !self.stamps.iter().any(|(kept, _)| *kept == path) {
                            self.stamps.push((path, stamp));
                        }
                    }
                    missing = true;
                    oid
                }
                Peeled::Unread(oid) => {
                    self.unpeeled.push(at);
                    oid
                }
            };
        }
        if missing {
            self.stamps.extend(odb.pack_stamps());
        }
    }
}

/// How far the tag naming `oid` peels: an annotated tag points at a tag
/// object, which points on, eight deep at most.
fn peel(odb: &mut Odb, mut oid: Oid) -> Peeled {
    for _ in 0..8 {
        let object = match odb.read(&oid) {
            Ok(Some(object)) => object,
            Ok(None) => return Peeled::Missing(oid),
            Err(_) => return Peeled::Unread(oid),
        };
        if object.kind != Kind::Tag {
            break;
        }
        let target = object
            .data
            .strip_prefix(b"object ")
            .and_then(|rest| Oid::from_hex(rest.get(..40)?));
        let Some(target) = target else { break };
        oid = target;
    }
    Peeled::To(oid)
}

/// The tags of a files repository, packed and loose, by name, filling
/// `peeled` with what each packed annotated tag peels to and `stamps` with
/// what they were read from.
pub(crate) fn files(
    shared: &Path,
    stamps: &mut Stamps,
    peeled: &mut HashMap<String, Oid>,
) -> Vec<(String, Oid)> {
    let mut found = std::collections::BTreeMap::new();
    let packed = shared.join("packed-refs");
    stamps.push((packed.clone(), Stamp::of(&packed)));
    if let Ok(text) = std::fs::read_to_string(&packed) {
        let mut last: Option<String> = None;
        for line in text.lines() {
            // A `^` line gives the commit the tag above it peels to.
            if let Some(hex) = line.strip_prefix('^')
                && let Some(oid) = Oid::from_hex(hex.as_bytes())
                && let Some(tag) = last.take()
            {
                peeled.insert(tag, oid);
            } else if let Some((hex, name)) = line.split_once(' ')
                && let Some(tag) = name.strip_prefix("refs/tags/")
                && let Some(oid) = Oid::from_hex(hex.as_bytes())
            {
                found.insert(tag.to_string(), oid);
                last = Some(tag.to_string());
            } else {
                last = None;
            }
        }
    }
    let mut stack = vec![shared.join("refs/tags")];
    while let Some(dir) = stack.pop() {
        // A tag is written beside the others and renamed into place, so its
        // folder's stamp moves with every tag made or moved.
        stamps.push((dir.clone(), Stamp::of(&dir)));
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(shared.join("refs/tags"))
                && let Ok(text) = std::fs::read(&path)
                && let Some(oid) = Oid::from_hex(text.trim_ascii())
            {
                // A loose tag is newer than its packed line, and so is what
                // it peels to.
                let name = rel.to_string_lossy().into_owned();
                peeled.remove(&name);
                found.insert(name, oid);
            }
        }
    }
    found.into_iter().collect()
}
