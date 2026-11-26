//! What kind an object is, read from headers as git's `oid_object_info`
//! reads it: a loose object's own, or a pack entry's and those of the
//! deltas under it down to their base, so that learning a tag points at a
//! blob costs the blob's headers rather than its bytes.

use super::chain::{base_distance, entry_header};
use super::{CHAIN_LIMIT, Kind, Odb};
use crate::hash::Oid;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;

/// What a copy's headers say of it.
enum Headed {
    Kind(Kind),
    /// A delta whose base is named, which says the kind.
    Named(Oid),
}

impl Odb {
    /// The kind of `oid`, or None where no store has it. Where its headers
    /// won't say, it's read whole, as `read` reads it, so anything refusing
    /// that refuses this, and only a copy's headers are trusted further
    /// than `read` trusts them.
    pub fn kind(&mut self, oid: &Oid) -> io::Result<Option<Kind>> {
        if let Some(kind) = self.kind_by_headers(oid) {
            return Ok(Some(kind));
        }
        Ok(self.read(oid)?.map(|object| object.kind))
    }

    /// The kind the headers of `oid`'s first copy say, through the bases
    /// its deltas name, or None where any won't say. Every delta on the
    /// way, placed or named, counts against one limit, as `read` counts a
    /// chain's depth, and a chain past it, a cycle of names among them, is
    /// left for `read` to refuse: no pack costs more headers than `read`
    /// would read deltas.
    fn kind_by_headers(&self, oid: &Oid) -> Option<Kind> {
        let mut deltas = 0;
        let mut oid = *oid;
        loop {
            if self.trees.contains_key(&oid) {
                return Some(Kind::Tree);
            }
            match self.headed(&oid, &mut deltas)? {
                Headed::Kind(kind) => return Some(kind),
                Headed::Named(base) => oid = base,
            }
        }
    }

    /// What the headers of the copy `read` would read first say: the first
    /// in the packs of any store, then the first loose. A copy marked bad,
    /// or an index or file that won't read, says nothing, as does a delta
    /// past `CHAIN_LIMIT` of them counted in `deltas`.
    fn headed(&self, oid: &Oid, deltas: &mut usize) -> Option<Headed> {
        for (at, store) in self.stores.iter().enumerate() {
            let mut refused = None;
            if let Some((pack, offset)) = store.locate(oid, &[], &mut refused) {
                if self.bad.contains_key(&(at, pack, offset)) {
                    return None;
                }
                return packed(&store.packs[pack].data, offset, deltas);
            }
            if refused.is_some() {
                return None;
            }
        }
        for store in &self.stores {
            match store.loose_kind(oid) {
                Ok(Some(kind)) => return Some(Headed::Kind(kind)),
                Ok(None) => {}
                Err(_) => return None,
            }
        }
        None
    }
}

/// What the entry at `offset` in a pack's `file` says, through the deltas
/// that place their bases in the same pack, each counted in `deltas`, or
/// None once they pass `CHAIN_LIMIT`.
fn packed(file: &File, mut offset: u64, deltas: &mut usize) -> Option<Headed> {
    loop {
        let mut head = [0; 32];
        let got = file.read_at(&mut head, offset).ok()?;
        let head = &head[..got];
        let (code, _, at) = entry_header(head).ok()?;
        if matches!(code, 6 | 7) {
            *deltas += 1;
            if *deltas > CHAIN_LIMIT {
                return None;
            }
        }
        match code {
            6 => {
                let (back, _) = base_distance(&head[at..]).ok()?;
                // As `read_down` holds it: before the delta, past the start.
                offset = offset
                    .checked_sub(back)
                    .filter(|&base_at| base_at > 0 && base_at < offset)?;
            }
            7 => return Some(Headed::Named(Oid::from_slice(head.get(at..at + 20)?)?)),
            code => return Kind::from_pack(code).map(Headed::Kind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::delta_tests::{Entries, appending, blob, named, names, packed, within};
    use super::super::{CHAIN_LIMIT, Kind, Odb, starve};
    use crate::hash::Oid;

    #[test]
    fn a_kind_is_read_from_headers_without_inflating_the_object() {
        let names = names(5);
        // names[0] a blob, names[1] a delta placing it by offset, names[2]
        // one naming names[1], names[3] one naming what no store holds, and
        // names[4] held nowhere.
        let mut entries = Entries::new();
        entries.push(blob(b"a"));
        entries.place_over(0, &appending(1, b'b'));
        entries.push(named(names[1], &appending(2, b'c')));
        entries.push(named([0xee; 20], &appending(1, b'd')));
        let folders = packed(&names, &[entries.entries]);
        let mut odb = Odb::open(&folders.0[0]);
        // No object's bytes, nor any delta's, fit in a byte.
        starve(Some(1));
        let kinds: Vec<_> = names[..3]
            .iter()
            .map(|name| odb.kind(&Oid(*name)).map_err(|error| error.to_string()))
            .collect();
        starve(None);
        let mut missing = |name| odb.kind(&Oid(name)).map_err(|error| error.kind());
        let (unbased, gone) = (missing(names[3]), missing(names[4]));
        assert_eq!(
            (kinds, unbased, gone),
            (
                vec![Ok(Some(Kind::Blob)); 3],
                Err(std::io::ErrorKind::NotFound),
                Ok(None)
            ),
            "a kind wasn't read from headers alone, a delta with no base wasn't refused as \
             one whose base may yet come, or a missing object wasn't None"
        );
    }

    #[test]
    fn a_kind_is_refused_past_the_limit_its_deltas_reach_placed_and_named() {
        // The blob names[0], then CHAIN_LIMIT + 1 deltas, each over the
        // entry before it, every other one naming its base: neither the
        // placed deltas nor the named ones reach the limit alone.
        let names = names(CHAIN_LIMIT + 2);
        let mut entries = Entries::new();
        entries.push(blob(b"a"));
        for at in 1..names.len() {
            let delta = [1, 1, 1, b'x'];
            if at % 2 == 0 {
                entries.push(named(names[at - 1], &delta));
            } else {
                entries.place_over(at - 1, &delta);
            }
        }
        let folders = packed(&names, &[entries.entries]);
        let objects = folders.0[0].clone();
        let (top, at_limit) = (Oid(names[CHAIN_LIMIT + 1]), Oid(names[CHAIN_LIMIT]));
        let kinds = within(60, move || {
            let mut odb = Odb::open(&objects);
            // At the limit its headers say, with no object's bytes read.
            starve(Some(1));
            let at_limit = odb.kind(&at_limit).map_err(|error| error.to_string());
            starve(None);
            let top = odb.kind(&top).map_err(|error| error.to_string());
            (at_limit, top)
        });
        assert_eq!(
            kinds,
            Ok((Ok(Some(Kind::Blob)), Err("delta chain too long".to_owned()))),
            "a chain at its limit wasn't kinded by its headers, or one past it, counted \
             placed and named together, was kinded where read refuses it"
        );
    }

    #[test]
    fn a_copy_marked_bad_says_nothing_of_its_kind() {
        let names = names(1);
        // In the first pack a copy whose header says a commit of one byte
        // and whose body won't inflate, in the second a sound blob.
        let unsound = vec![0x11, 0xff, 0xff, 0xff, 0xff];
        let folders = packed(&names, &[vec![unsound], vec![blob(b"a")]]);
        let mut odb = Odb::open(&folders.0[0]);
        let oid = Oid(names[0]);
        let read = odb
            .read(&oid)
            .map(|object| object.map(|object| (object.kind, object.data)))
            .map_err(|error| error.to_string());
        let kind = odb.kind(&oid).map_err(|error| error.to_string());
        assert_eq!(
            (read, kind),
            (Ok(Some((Kind::Blob, b"a".to_vec()))), Ok(Some(Kind::Blob))),
            "a copy that wouldn't unpack wasn't passed over for the sound one, or its \
             header was believed once it was marked bad"
        );
    }

    #[test]
    fn a_loose_kind_is_read_from_its_header() {
        let objects = crate::pack_index::tests::unique("kind-loose");
        let oid = Oid([0x0b; 20]);
        let folder = objects.join(&oid.hex()[..2]);
        std::fs::create_dir_all(&folder).expect("the temp folder takes a folder");
        let object = [b"commit 4096\0".as_slice(), &[b'x'; 4096]].concat();
        let stream = miniz_oxide::deflate::compress_to_vec_zlib(&object, 1);
        std::fs::write(folder.join(&oid.hex()[2..]), stream).expect("the temp folder takes a file");
        starve(Some(1));
        let kind = Odb::open(&objects)
            .kind(&oid)
            .map_err(|error| error.to_string());
        starve(None);
        std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
        assert_eq!(
            kind,
            Ok(Some(Kind::Commit)),
            "a loose object's kind took more than its header"
        );
    }
}
