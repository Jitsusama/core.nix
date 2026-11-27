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
    /// one whose object wouldn't read for a while, as for memory or an I/O
    /// error, is peeled again each answer, alone, which may find it read.
    unpeeled: Vec<usize>,
}

/// How far an annotated tag peels.
enum Peeled {
    /// To what isn't a tag object: the commit, most often.
    To(Oid),
    /// To a tag object no store holds.
    Missing(Oid),
    /// To a tag object whose bytes are wrong: they stay so till its file
    /// is written again, as one missing stays missing.
    Corrupt(Oid),
    /// To a tag object that wouldn't read this time, a delta's base no
    /// store holds yet among the reasons.
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

    /// Peels each tag not yet peeled: one whose object is missing or wrong
    /// settles with the stamps of where it would come from, rather than be
    /// read again every answer, and one whose object wouldn't read this time
    /// is left for the next.
    fn peel(&mut self, odb: &mut Odb) {
        let mut missing = false;
        for at in std::mem::take(&mut self.unpeeled) {
            self.named[at].1 = match peel(odb, self.named[at].1) {
                Peeled::To(oid) => oid,
                Peeled::Missing(oid) | Peeled::Corrupt(oid) => {
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
/// object, which points on, eight deep at most. Only a tag object is read
/// whole: what it points at is asked only its kind, as git's `peel_object`
/// asks, so a tag on a large blob costs the blob's headers.
fn peel(odb: &mut Odb, mut oid: Oid) -> Peeled {
    // Only data that won't read settles; anything else is tried again.
    let stopped = |oid, error: std::io::Error| {
        if error.kind() == std::io::ErrorKind::InvalidData {
            Peeled::Corrupt(oid)
        } else {
            Peeled::Unread(oid)
        }
    };
    for _ in 0..8 {
        match odb.kind(&oid) {
            Ok(Some(Kind::Tag)) => {}
            Ok(Some(_)) => break,
            Ok(None) => return Peeled::Missing(oid),
            Err(error) => return stopped(oid, error),
        }
        let object = match odb.read(&oid) {
            Ok(Some(object)) => object,
            Ok(None) => return Peeled::Missing(oid),
            Err(error) => return stopped(oid, error),
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
    if let Ok(Some(bytes)) = crate::regular_file::read(&packed) {
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
                && let Ok(Some(text)) = crate::regular_file::read(&path)
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
    fn a_tag_whose_object_is_missing_or_wrong_is_peeled_once_and_no_stamp_kept_twice() {
        let objects = crate::pack_index::tests::unique("tags-wrong");
        std::fs::create_dir_all(objects.join("pack")).expect("the objects folder is made");
        let (gone, wrong) = (Oid([1; 20]), Oid([2; 20]));
        // v2's object is there but isn't zlib, which it stays till its
        // folder takes another file.
        let loose = objects.join(&wrong.hex()[..2]);
        std::fs::create_dir_all(&loose).expect("v2's folder is made");
        std::fs::write(loose.join(&wrong.hex()[2..]), b"not zlib").expect("v2 is written");
        // v3 is missing too, in v1's fan-out folder, whose stamp is kept once.
        let mut also_gone = [1; 20];
        also_gone[19] = 9;
        let named = vec![
            ("v1".to_string(), gone),
            ("v2".to_string(), wrong),
            ("v3".to_string(), Oid(also_gone)),
        ];
        let mut tags = Tags::new(Vec::new(), named, &HashMap::new());
        tags.naming(Oid([3; 20]), &mut Odb::open(&objects));
        let mut paths: Vec<_> = tags.stamps.iter().map(|(path, _)| path.clone()).collect();
        let kept = paths.len();
        paths.sort();
        paths.dedup();
        std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
        assert_eq!(
            (tags.unpeeled.len(), kept, paths.contains(&loose)),
            (0, paths.len(), true),
            "a missing or wrong tag was left to peel again, a stamp was kept twice, or \
             the wrong one's folder wasn't kept"
        );
    }

    /// Writes `object`, whole with its header, loose as `oid` in `objects`.
    fn loose(objects: &Path, oid: Oid, object: &[u8]) {
        let folder = objects.join(&oid.hex()[..2]);
        std::fs::create_dir_all(&folder).expect("the object's folder is made");
        std::fs::write(
            folder.join(&oid.hex()[2..]),
            miniz_oxide::deflate::compress_to_vec_zlib(object, 1),
        )
        .expect("the object is written");
    }

    #[test]
    fn a_tag_is_peeled_past_its_target_s_kind_without_reading_the_target() {
        let objects = crate::pack_index::tests::unique("tags-blob");
        let (tag, blob) = (Oid([6; 20]), Oid([7; 20]));
        let raw = format!("object {}\ntype blob\n", blob.hex());
        loose(
            &objects,
            tag,
            format!("tag {}\0{raw}", raw.len()).as_bytes(),
        );
        loose(
            &objects,
            blob,
            &[b"blob 4096\0".as_slice(), &[b'x'; 4096]].concat(),
        );
        let mut tags = Tags::new(Vec::new(), vec![("v1".to_string(), tag)], &HashMap::new());
        // Room for the tag's bytes, not the blob's.
        crate::odb::starve(Some(1024));
        let named = tags.naming(blob, &mut Odb::open(&objects));
        crate::odb::starve(None);
        std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
        assert_eq!(
            (named.as_str(), tags.unpeeled.len()),
            ("v1", 0),
            "a tag on a blob read the blob whole to learn its kind"
        );
    }

    #[test]
    fn a_tag_whose_object_wouldn_t_read_this_time_is_peeled_again_the_next() {
        let objects = crate::pack_index::tests::unique("tags-starved");
        let (tag, commit) = (Oid([4; 20]), Oid([5; 20]));
        let raw = format!("object {}\ntype commit\n", commit.hex());
        loose(
            &objects,
            tag,
            format!("tag {}\0{raw}", raw.len()).as_bytes(),
        );
        let mut tags = Tags::new(Vec::new(), vec![("v1".to_string(), tag)], &HashMap::new());
        let mut odb = Odb::open(&objects);
        crate::odb::starve(Some(16));
        let starved = tags.naming(commit, &mut odb);
        crate::odb::starve(None);
        let fed = tags.naming(commit, &mut odb);
        std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
        assert_eq!(
            (starved.as_str(), fed.as_str(), tags.unpeeled.len()),
            ("", "v1", 0),
            "a tag whose object had no memory to read was settled unpeeled, or peeled \
             through nothing"
        );
    }
}
