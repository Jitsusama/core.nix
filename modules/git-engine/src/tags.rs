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
                    self.keep(odb.folders_of(&oid));
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
            self.keep(odb.pack_stamps());
        }
    }

    /// Settles the tags with `stamps` too, each path kept once, as a tag
    /// left unread may yet be missing on a later answer.
    fn keep(&mut self, stamps: Stamps) {
        for (path, stamp) in stamps {
            if !self.stamps.iter().any(|(kept, _)| *kept == path) {
                self.stamps.push((path, stamp));
            }
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
    // Read as bytes, since a ref's name may be any, and one line not UTF-8
    // mustn't cost every packed tag.
    if let Ok(bytes) = std::fs::read(&packed) {
        let text = String::from_utf8_lossy(&bytes);
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
                // it peels to, unless it names what its packed line does.
                let name = rel.to_string_lossy().into_owned();
                if found.get(&name) != Some(&oid) {
                    peeled.remove(&name);
                }
                found.insert(name, oid);
            }
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{Odb, Oid, Tags, files};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    /// A files repository's shared folder holding `packed` as its
    /// packed-refs, and v1 loose naming `loose`, if given.
    fn shared(name: &str, packed: &[u8], loose: Option<Oid>) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("git-engine-tags-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("refs/tags")).expect("the tags folder is made");
        std::fs::write(dir.join("packed-refs"), packed).expect("packed-refs is written");
        if let Some(oid) = loose {
            std::fs::write(dir.join("refs/tags/v1"), format!("{}\n", oid.hex()))
                .expect("v1 is written loose");
        }
        dir
    }

    /// The tags read from `dir`, and what the packed ones peel to.
    fn read(dir: &Path) -> (Vec<(String, Oid)>, HashMap<String, Oid>) {
        let mut peeled = HashMap::new();
        let found = files(dir, &mut Vec::new(), &mut peeled);
        let _ = std::fs::remove_dir_all(dir);
        (found, peeled)
    }

    #[test]
    fn a_loose_tag_takes_its_packed_peel_only_while_it_names_the_same_object() {
        let (tag, commit, moved) = (Oid([4; 20]), Oid([5; 20]), Oid([6; 20]));
        let packed = format!("{} refs/tags/v1\n^{}\n", tag.hex(), commit.hex());
        let same = read(&shared("same", packed.as_bytes(), Some(tag)));
        let newer = read(&shared("newer", packed.as_bytes(), Some(moved)));
        assert_eq!(
            (same.1.get("v1"), newer.0, newer.1.get("v1")),
            (Some(&commit), vec![("v1".to_string(), moved)], None),
            "a loose tag naming its packed line's object lost its peel, or one naming \
             another kept the stale peel or the stale object"
        );
    }

    #[test]
    fn a_packed_line_not_utf8_costs_no_other_packed_tag() {
        let (tag, commit) = (Oid([7; 20]), Oid([8; 20]));
        let packed = [
            format!("{} refs/tags/caf", Oid([9; 20]).hex()).as_bytes(),
            b"\xe9\n",
            format!("{} refs/tags/v1\n^{}\n", tag.hex(), commit.hex()).as_bytes(),
        ]
        .concat();
        let (found, peeled) = read(&shared("latin", &packed, None));
        assert_eq!(
            (found.len(), found.last(), peeled.get("v1")),
            (2, Some(&("v1".to_string(), tag)), Some(&commit)),
            "a packed-refs line not UTF-8 cost the tags packed beside it"
        );
    }

    #[test]
    fn a_tag_whose_object_is_missing_is_peeled_once_and_no_stamp_kept_twice() {
        let objects = std::env::temp_dir().join(format!("git-engine-tags-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&objects);
        std::fs::create_dir_all(objects.join("pack")).expect("the objects folder is made");
        let (gone, unread) = (Oid([1; 20]), Oid([2; 20]));
        // v2's object is there but isn't zlib, so it's left unread till
        // it's taken away, and then missing too.
        let loose = objects.join(&unread.hex()[..2]);
        std::fs::create_dir_all(&loose).expect("v2's folder is made");
        let file = loose.join(&unread.hex()[2..]);
        std::fs::write(&file, b"not zlib").expect("v2's object is written");
        let named = vec![("v1".to_string(), gone), ("v2".to_string(), unread)];
        let mut tags = Tags::new(Vec::new(), named, &HashMap::new());
        let mut odb = Odb::open(&objects);
        tags.naming(Oid([3; 20]), &mut odb);
        let first = tags.unpeeled.clone();
        std::fs::remove_file(&file).expect("v2's object is taken away");
        tags.naming(Oid([3; 20]), &mut odb);
        let mut paths: Vec<_> = tags.stamps.iter().map(|(path, _)| path).collect();
        let kept = paths.len();
        paths.sort();
        paths.dedup();
        assert_eq!(
            (first, tags.unpeeled.len(), kept),
            (vec![1], 0, paths.len()),
            "a missing tag was left to peel again, an unread one wasn't, or a stamp \
             was kept twice"
        );
        let _ = std::fs::remove_dir_all(&objects);
    }
}
