//! Finding an object by name and reading it through every delta on the way
//! to its base. The chain is walked as a loop over searches kept on the
//! heap, as git's `unpack_entry` walks its deltas on a stack of its own,
//! rather than by calling down into each base a delta names: called down
//! into, each base took more than 2 KiB of stack, so a chain a pack may
//! hold overflowed the 8 MiB a process's first thread has.

use super::{CHAIN_LIMIT, Kind, Object, Odb, Place, apply_delta, inflate_at, invalid};
use crate::hash::Oid;
use std::collections::{HashMap, HashSet};
use std::io;
use std::os::unix::fs::FileExt;

/// The copies passed over for the rest of a find, and why each was.
type Skipped = HashMap<Place, (io::ErrorKind, String)>;

/// The bases a find spent every copy of, each with whether one was passed
/// over and what refused it. Copies stay marked bad, and passed over, for
/// the rest of the find, so a base spent once would be spent again just
/// as it was: a copy naming it again ends as the first did, without its
/// copies looked up again.
type Spent = HashMap<Oid, (bool, io::ErrorKind, String)>;

#[cfg(test)]
thread_local! {
    /// How many times this thread's finds asked a store for a copy, which
    /// the tests count to hold a find's work to the copies there are.
    pub(super) static LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// An object being found by name, the one asked for or a base a delta
/// above it names, and how far the finding has got.
struct Search {
    oid: Oid,
    /// The store whose packs are asked, and those of its packs already
    /// passed over; once every store's packs are, the store whose loose
    /// objects are asked next.
    store: usize,
    passed: Vec<usize>,
    loose: usize,
    /// The first refusal met, which stands should no copy read.
    refused: Option<io::Error>,
    /// Whether a copy was passed over for this find alone, its chain
    /// coming round or running too long, rather than refused.
    skipped: bool,
    /// The copy being read, and the deltas read down from it toward its
    /// base so far, nearest the copy first.
    copy: Option<Place>,
    deltas: Vec<Hop>,
    /// How many deltas the searches above this one read on the way to it.
    above: usize,
}

/// A delta read down through: where it lies, and where its data begins
/// and how large it inflates. Its data is inflated only as it's applied,
/// as git's `unpack_entry` stacks only where each delta lies, so a long
/// chain holds one delta's data at a time rather than all of them.
struct Hop {
    place: Place,
    data: u64,
    size: usize,
}

impl Search {
    fn new(oid: Oid, above: usize) -> Search {
        Search {
            oid,
            store: 0,
            passed: Vec::new(),
            loose: 0,
            refused: None,
            skipped: false,
            copy: None,
            deltas: Vec::new(),
            above,
        }
    }

    /// How many deltas lie above the entry read next.
    fn depth(&self) -> usize {
        self.above + self.deltas.len()
    }
}

/// The next place a search may find its object.
enum Next {
    Packed(Place),
    Loose(Object),
    Spent,
}

/// Where reading down a copy's deltas ended.
enum Reached {
    /// Its base, over which every delta read is applied.
    Base(Object),
    /// A delta naming its base, which is found by name.
    Named(Oid),
    /// Past `CHAIN_LIMIT` deltas from the object asked for.
    TooLong,
    /// A named base whose copies were spent with one passed over, which
    /// says nothing of the copy whose delta named it.
    Skipped(io::Error),
}

impl Odb {
    /// The object, read through every delta on the way to its base, or
    /// None when no store here has it.
    ///
    /// What a store's indexes, packs or loose objects refuse stands only
    /// once every other place the object may lie has been read. A copy that
    /// won't read is marked bad for as long as the store is open, as git
    /// marks a copy bad and looks again, so a sound copy in another pack is
    /// read and no copy is unpacked to its failure twice. A copy refused
    /// memory is marked too, with that refusal, since only a stream truly
    /// inflating that far is refused it, and unpacking it on every find
    /// would cost the same each time; a change to the packs clears every
    /// mark, as the store is opened again. A copy whose
    /// chain comes round to an object already being found, or runs past
    /// `CHAIN_LIMIT`, may be sound under another copy of a base on the
    /// way, so it's only passed over, for this find alone, and the next
    /// copy read; a base spent with a copy passed over passes over the
    /// copy that named it in turn. Each copy is passed over at most once a
    /// find, so a find ends after reading each copy at most once, and a
    /// base spent ends each copy naming it after as it ended the first, so
    /// its copies are looked up once a find, however many copies name it.
    pub(super) fn find(&mut self, oid: &Oid) -> io::Result<Option<Object>> {
        let mut searches = vec![Search::new(*oid, 0)];
        let mut searching = HashSet::from([*oid]);
        let mut skipped = Skipped::new();
        let mut spent = Spent::new();
        // The base a search above handed down, or a tree already read.
        let mut handed = None;
        loop {
            let at = searches.len() - 1;
            let reached = if let Some(base) = handed.take() {
                Ok(Reached::Base(base))
            } else {
                match self.next_copy(&mut searches[at], &skipped) {
                    Next::Packed(place) => self.read_down(&mut searches[at], place),
                    Next::Loose(object) => Ok(Reached::Base(object)),
                    // A base no store holds ends the copy of the delta that
                    // named it, below.
                    Next::Spent => {
                        let ended = end(&mut searches, &mut searching);
                        if searches.is_empty() {
                            return ended.refused.map_or(Ok(None), Err);
                        }
                        let refused = ended
                            .refused
                            .unwrap_or_else(|| invalid("delta base missing"));
                        spent.insert(
                            ended.oid,
                            (ended.skipped, refused.kind(), refused.to_string()),
                        );
                        if ended.skipped {
                            Ok(Reached::Skipped(refused))
                        } else {
                            Err(refused)
                        }
                    }
                }
            };
            let at = searches.len() - 1;
            match reached {
                Err(error) => self.fail(&mut searches[at], &error),
                Ok(Reached::TooLong) => {
                    skip(
                        &mut searches[at],
                        &mut skipped,
                        &invalid("delta chain too long"),
                    );
                }
                Ok(Reached::Skipped(error)) => skip(&mut searches[at], &mut skipped, &error),
                Ok(Reached::Named(base)) => {
                    if let Some(tree) = self.tree(&base) {
                        handed = Some(tree);
                    } else if let Some((passed_over, kind, said)) = spent.get(&base) {
                        let error = io::Error::new(*kind, said.clone());
                        if *passed_over {
                            skip(&mut searches[at], &mut skipped, &error);
                        } else {
                            self.fail(&mut searches[at], &error);
                        }
                    } else if searching.insert(base) {
                        let above = searches[at].depth();
                        searches.push(Search::new(base, above));
                    } else {
                        skip(
                            &mut searches[at],
                            &mut skipped,
                            &invalid("delta base cycle"),
                        );
                    }
                }
                Ok(Reached::Base(base)) => match self.apply(&mut searches[at], base) {
                    Err(error) => self.fail(&mut searches[at], &error),
                    Ok(object) => {
                        let found = end(&mut searches, &mut searching);
                        self.keep_tree(&found.oid, &object);
                        if searches.is_empty() {
                            return Ok(Some(object));
                        }
                        handed = Some(object);
                    }
                },
            }
        }
    }

    /// The next copy of `search`'s object neither marked bad nor passed
    /// over in this find, in the packs of each store and then in its loose
    /// objects, keeping the first refusal met on the way. A copy passed
    /// over before passes over this search too, should it find nothing,
    /// since its base may be sound under another copy of a base on the way.
    fn next_copy(&self, search: &mut Search, skipped: &Skipped) -> Next {
        while let Some(store) = self.stores.get(search.store) {
            #[cfg(test)]
            LOOKUPS.with(|lookups| lookups.set(lookups.get() + 1));
            let Some((pack, offset)) =
                store.locate(&search.oid, &search.passed, &mut search.refused)
            else {
                search.store += 1;
                search.passed.clear();
                continue;
            };
            search.passed.push(pack);
            let place = (search.store, pack, offset);
            if let Some((kind, said)) = self.bad.get(&place) {
                search
                    .refused
                    .get_or_insert_with(|| io::Error::new(*kind, said.clone()));
            } else if let Some((kind, said)) = skipped.get(&place) {
                search.skipped = true;
                search
                    .refused
                    .get_or_insert_with(|| io::Error::new(*kind, said.clone()));
            } else {
                search.copy = Some(place);
                return Next::Packed(place);
            }
        }
        while let Some(store) = self.stores.get(search.loose) {
            search.loose += 1;
            match store.loose(&search.oid) {
                Ok(Some(object)) => return Next::Loose(object),
                Ok(None) => {}
                Err(error) => {
                    search.refused.get_or_insert(error);
                }
            }
        }
        Next::Spent
    }

    /// Reads `search`'s copy at `place` down through the deltas that place
    /// their bases in its pack, noting where each lies, to its base or the
    /// delta that names one.
    fn read_down(&mut self, search: &mut Search, place: Place) -> io::Result<Reached> {
        let (store, pack, mut offset) = place;
        loop {
            if search.depth() > CHAIN_LIMIT {
                return Ok(Reached::TooLong);
            }
            if let Some(base) = self.bases.get(&(store, pack, offset)) {
                return Ok(Reached::Base(Object {
                    kind: base.0,
                    data: base.1.clone(),
                }));
            }
            let file = &self.stores[store].packs[pack].data;
            let head = {
                let mut buf = [0; 32];
                let got = file.read_at(&mut buf, offset)?;
                buf[..got].to_vec()
            };
            let (code, size, mut at) = entry_header(&head)?;
            match code {
                6 => {
                    let (back, read) = base_distance(&head[at..])?;
                    at += read;
                    // A base lies before its delta and past the pack's
                    // start, as git's get_delta_base holds it, so a delta
                    // can't be its own base.
                    let base_at = offset
                        .checked_sub(back)
                        .filter(|&base_at| base_at > 0 && base_at < offset)
                        .ok_or_else(|| invalid("delta base offset"))?;
                    search.deltas.push(Hop {
                        place: (store, pack, offset),
                        data: offset + at as u64,
                        size,
                    });
                    offset = base_at;
                }
                7 => {
                    let base = head
                        .get(at..at + 20)
                        .and_then(Oid::from_slice)
                        .ok_or_else(|| invalid("delta base"))?;
                    search.deltas.push(Hop {
                        place: (store, pack, offset),
                        data: offset + at as u64 + 20,
                        size,
                    });
                    return Ok(Reached::Named(base));
                }
                code => {
                    let kind = Kind::from_pack(code).ok_or_else(|| invalid("pack entry kind"))?;
                    let object = Object {
                        kind,
                        data: inflate_at(file, offset + at as u64, size)?,
                    };
                    if search.depth() > 0 {
                        self.keep_base((store, pack, offset), &object);
                    }
                    return Ok(Reached::Base(object));
                }
            }
        }
    }

    /// `base` with every delta `search` read applied over it, from the one
    /// nearest it up, each inflated as it's applied and each result below
    /// the object asked for kept as a base for the reads to come.
    fn apply(&mut self, search: &mut Search, base: Object) -> io::Result<Object> {
        let mut object = base;
        while let Some(hop) = search.deltas.pop() {
            let (store, pack, _) = hop.place;
            let delta = inflate_at(&self.stores[store].packs[pack].data, hop.data, hop.size)?;
            object.data = apply_delta(&object.data, &delta)?;
            if search.depth() > 0 {
                self.keep_base(hop.place, &object);
            }
        }
        Ok(object)
    }

    /// Ends the copy `search` was reading, which won't read for `error`:
    /// marks it bad, keeps the refusal should it be the first, and leaves
    /// the next copy to be read. The mark keeps the error's kind, so a copy
    /// refused memory is never taken for a corrupt one.
    fn fail(&mut self, search: &mut Search, error: &io::Error) {
        if let Some(copy) = search.copy.take() {
            self.bad.insert(copy, (error.kind(), error.to_string()));
        }
        refuse(search, error);
    }
}

/// A pack entry's kind, the size of what it inflates to, and how many of
/// `head`'s bytes its header takes, refused as git's
/// `unpack_object_header_buffer` refuses one whose size runs past a word:
/// once a byte's seven bits would shift past it, before any are dropped.
fn entry_header(head: &[u8]) -> io::Result<(u8, usize, usize)> {
    let byte = *head.first().ok_or_else(|| invalid("pack entry"))?;
    let code = (byte >> 4) & 7;
    let mut size = usize::from(byte & 15);
    let mut shift = 4;
    let mut more = byte & 0x80 != 0;
    let mut at = 1;
    while more {
        let byte = *head.get(at).ok_or_else(|| invalid("pack entry size"))?;
        if shift > usize::BITS - 7 {
            return Err(invalid("pack entry size"));
        }
        at += 1;
        size |= usize::from(byte & 0x7f) << shift;
        shift += 7;
        more = byte & 0x80 != 0;
    }
    Ok((code, size, at))
}

/// How far back an offset delta places its base, and how many of `head`'s
/// bytes say so, refused as git's `get_delta_base` refuses a distance
/// past 64 bits.
fn base_distance(head: &[u8]) -> io::Result<(u64, usize)> {
    let mut byte = *head.first().ok_or_else(|| invalid("delta offset"))?;
    let mut back = u64::from(byte & 0x7f);
    let mut at = 1;
    while byte & 0x80 != 0 {
        byte = *head.get(at).ok_or_else(|| invalid("delta offset"))?;
        at += 1;
        let next = back
            .checked_add(1)
            .filter(|next| next >> (u64::BITS - 7) == 0)
            .ok_or_else(|| invalid("delta offset"))?;
        back = (next << 7) | u64::from(byte & 0x7f);
    }
    Ok((back, at))
}

/// Passes over the copy `search` was reading for the rest of this find,
/// marking nothing: its chain came round to an object being found, or ran
/// too long, which may not be the copy's own doing.
fn skip(search: &mut Search, skipped: &mut Skipped, error: &io::Error) {
    if let Some(copy) = search.copy.take() {
        skipped.insert(copy, (error.kind(), error.to_string()));
    }
    search.skipped = true;
    refuse(search, error);
}

/// Keeps `error` as `search`'s refusal should it be the first, and drops
/// the deltas read for the copy it ends.
fn refuse(search: &mut Search, error: &io::Error) {
    search
        .refused
        .get_or_insert_with(|| io::Error::new(error.kind(), error.to_string()));
    search.deltas.clear();
}

/// Takes the search on top off, no longer being found.
fn end(searches: &mut Vec<Search>, searching: &mut HashSet<Oid>) -> Search {
    let ended = searches.pop().expect("a search is under way");
    searching.remove(&ended.oid);
    ended
}
