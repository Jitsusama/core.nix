//! Finding an object by name and reading it through every delta on the way
//! to its base. The chain is walked as a loop over searches kept on the
//! heap, as git's `unpack_entry` walks its deltas on a stack of its own,
//! rather than by calling down into each base a delta names: called down
//! into, each base took more than 2 KiB of stack, so a chain a pack may
//! hold overflowed the 8 MiB a process's first thread has.

use super::{CHAIN_LIMIT, Kind, Object, Odb, Place, apply_delta, inflate_at, invalid};
use crate::hash::Oid;
use std::collections::HashMap;
use std::io;
use std::os::unix::fs::FileExt;

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
    /// The copy being read, and the deltas read down from it toward its
    /// base so far, each with where it lies.
    copy: Option<Place>,
    deltas: Vec<(Place, Vec<u8>)>,
    /// How many deltas the searches above this one read on the way to it.
    above: usize,
}

impl Search {
    fn new(oid: Oid, above: usize) -> Search {
        Search {
            oid,
            store: 0,
            passed: Vec::new(),
            loose: 0,
            refused: None,
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
}

impl Odb {
    /// The object, read through every delta on the way to its base, or
    /// None when no store here has it.
    ///
    /// What a store's indexes, packs or loose objects refuse stands only
    /// once every other place the object may lie has been read. A copy that
    /// won't read is marked bad for as long as the store is open, as git
    /// marks a copy bad and looks again, so a sound copy in another pack is
    /// read and no copy is unpacked to its failure twice. A chain that
    /// comes round to an object already being found, or runs past
    /// `CHAIN_LIMIT`, ends only where the read began, so no copy on the way
    /// is marked: the search the cycle returns to, or the first, marks its
    /// own copy and reads on.
    pub(super) fn find(&mut self, oid: &Oid) -> io::Result<Option<Object>> {
        let mut searches = vec![Search::new(*oid, 0)];
        let mut searching = HashMap::from([(*oid, 0)]);
        // The base a search above handed down, or a tree already read.
        let mut handed = None;
        loop {
            let reached = if let Some(base) = handed.take() {
                Ok(Reached::Base(base))
            } else {
                let at = searches.len() - 1;
                match self.next_copy(&mut searches[at]) {
                    Next::Packed(place) => self.read_down(&mut searches[at], place),
                    Next::Loose(object) => Ok(Reached::Base(object)),
                    // A base no store holds ends the copy of the delta that
                    // named it, below.
                    Next::Spent => {
                        let spent = searches.pop().expect("a search is under way");
                        searching.remove(&spent.oid);
                        if searches.is_empty() {
                            return spent.refused.map_or(Ok(None), Err);
                        }
                        Err(spent
                            .refused
                            .unwrap_or_else(|| invalid("delta base missing")))
                    }
                }
            };
            let at = searches.len() - 1;
            match reached {
                Err(error) => self.fail(&mut searches[at], &error),
                Ok(Reached::TooLong) => {
                    unwind(&mut searches, &mut searching, 0);
                    self.fail(&mut searches[0], &invalid("delta chain too long"));
                }
                Ok(Reached::Named(base)) => {
                    if let Some(tree) = self.tree(&base) {
                        handed = Some(tree);
                    } else if let Some(&cycle) = searching.get(&base) {
                        unwind(&mut searches, &mut searching, cycle);
                        self.fail(&mut searches[cycle], &invalid("delta base cycle"));
                    } else {
                        let above = searches[at].depth();
                        searching.insert(base, searches.len());
                        searches.push(Search::new(base, above));
                    }
                }
                Ok(Reached::Base(base)) => match self.apply(&mut searches[at], base) {
                    Err(error) => self.fail(&mut searches[at], &error),
                    Ok(object) => {
                        let found = searches.pop().expect("a search is under way");
                        searching.remove(&found.oid);
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

    /// The next copy of `search`'s object not marked bad, in the packs
    /// of each store and then in its loose objects, keeping the first
    /// refusal met on the way.
    fn next_copy(&self, search: &mut Search) -> Next {
        while let Some(store) = self.stores.get(search.store) {
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
    /// their bases in its pack, keeping each, to its base or the delta that
    /// names one.
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
            let byte = *head.first().ok_or_else(|| invalid("pack entry"))?;
            let code = (byte >> 4) & 7;
            let mut size = usize::from(byte & 15);
            let mut shift = 4;
            let mut more = byte & 0x80 != 0;
            let mut at = 1;
            while more {
                let byte = *head.get(at).ok_or_else(|| invalid("pack entry size"))?;
                at += 1;
                size |= usize::from(byte & 0x7f) << shift;
                shift += 7;
                more = byte & 0x80 != 0;
            }
            match code {
                6 => {
                    let mut byte = *head.get(at).ok_or_else(|| invalid("delta offset"))?;
                    at += 1;
                    let mut back = u64::from(byte & 0x7f);
                    while byte & 0x80 != 0 {
                        byte = *head.get(at).ok_or_else(|| invalid("delta offset"))?;
                        at += 1;
                        back = ((back + 1) << 7) | u64::from(byte & 0x7f);
                    }
                    // A base lies before its delta and past the pack's
                    // start, as git's get_delta_base holds it, so a delta
                    // can't be its own base.
                    let base_at = offset
                        .checked_sub(back)
                        .filter(|&base_at| base_at > 0 && base_at < offset)
                        .ok_or_else(|| invalid("delta base offset"))?;
                    let delta = inflate_at(file, offset + at as u64, size)?;
                    search.deltas.push(((store, pack, offset), delta));
                    offset = base_at;
                }
                7 => {
                    let base = head
                        .get(at..at + 20)
                        .and_then(Oid::from_slice)
                        .ok_or_else(|| invalid("delta base"))?;
                    let delta = inflate_at(file, offset + at as u64 + 20, size)?;
                    search.deltas.push(((store, pack, offset), delta));
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
    /// nearest it up, each result below the object asked for kept as a
    /// base for the reads to come.
    fn apply(&mut self, search: &mut Search, base: Object) -> io::Result<Object> {
        let mut object = base;
        while let Some((place, delta)) = search.deltas.pop() {
            object.data = apply_delta(&object.data, &delta)?;
            if search.depth() > 0 {
                self.keep_base(place, &object);
            }
        }
        Ok(object)
    }

    /// Ends the copy `search` was reading, which won't read for `error`:
    /// marks it bad, keeps the refusal should it be the first, and leaves
    /// the next copy to be read.
    fn fail(&mut self, search: &mut Search, error: &io::Error) {
        let said = error.to_string();
        if let Some(copy) = search.copy.take() {
            self.bad.insert(copy, (error.kind(), said.clone()));
        }
        search
            .refused
            .get_or_insert_with(|| io::Error::new(error.kind(), said));
        search.deltas.clear();
    }
}

/// Drops every search above the one at `to`, marking none of their copies:
/// they read only for the one at `to`, which ends its own.
fn unwind(searches: &mut Vec<Search>, searching: &mut HashMap<Oid, usize>, to: usize) {
    for dropped in searches.drain(to + 1..) {
        searching.remove(&dropped.oid);
    }
}
