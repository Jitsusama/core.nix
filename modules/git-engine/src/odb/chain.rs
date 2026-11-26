//! Finding an object by name and reading it through every delta on the way
//! to its base. The chain is walked as a loop over searches kept on the
//! heap, as git's `unpack_entry` walks its deltas on a stack of its own,
//! rather than by calling down into each base a delta names: called down
//! into, each base took more than 2 KiB of stack, so a chain a pack may
//! hold overflowed the 8 MiB a process's first thread has.

use super::{
    BASE_BUDGET, CHAIN_LIMIT, Kind, Mark, Object, Odb, Place, Stored, apply_delta, delta_base_size,
    inflate_at, invalid,
};
use crate::hash::Oid;
use std::collections::{HashMap, HashSet};
use std::io;
use std::os::unix::fs::FileExt;

/// The copies passed over in a find, each with what refused it and what
/// passed it over, which passes it over again only while it holds.
type Skipped = HashMap<Place, (io::ErrorKind, String, Hold)>;

/// The bases a find spent every copy of, each with what refused it and,
/// where a copy was passed over, what passed it over. Copies stay marked
/// bad for the rest of the find, so a base spent with none passed over
/// would be spent again just as it was, and one spent with a copy passed
/// over would be again while what passed it over holds: a copy naming it
/// then ends as the first did, without its copies looked up again.
type Spent = HashMap<Oid, (io::ErrorKind, String, Option<Hold>)>;

/// What passed a copy over, or a search's copies: a chain under it that
/// ran past `CHAIN_LIMIT` from the depth it was entered at, and the names
/// a chain under it came round to while each was being found. Reached no
/// nearer the top, a chain runs as far again; and while a name is being
/// found, a chain reaching it comes round again. Either way it reads no
/// better, so it's passed over again. Reached nearer the top, or once a
/// name it came round to is found or spent, it may read, so it's read.
#[derive(Clone, Default)]
struct Hold {
    /// The depth the copy was entered at, where a chain under it ran too
    /// long.
    from: Option<usize>,
    /// The names a chain under it came round to.
    round: Vec<Oid>,
}

impl Hold {
    /// Whether a copy entered at `depth` would be passed over as before.
    fn holds(&self, depth: usize, searching: &HashSet<Oid>) -> bool {
        self.from.is_none_or(|from| depth >= from)
            && self.round.iter().all(|name| searching.contains(name))
    }

    /// The same, said of a copy entered at `depth`, over whose chain it
    /// held: what ran too long under a base the copy names runs too long
    /// under the copy wherever it's entered no nearer the top.
    fn under(&self, depth: usize) -> Hold {
        Hold {
            from: self.from.map(|_| depth),
            round: self.round.clone(),
        }
    }

    /// Adds what passed over another copy of the same search, so the
    /// search's copies are passed over again only where every one's would
    /// be: from the deepest of their depths, with all their names.
    fn join(&mut self, other: &Hold) {
        self.from = self.from.max(other.from);
        for name in &other.round {
            if !self.round.contains(name) {
                self.round.push(*name);
            }
        }
    }
}

/// The bases a find found by name, kept for the rest of it, so a base
/// named again by the next copy of a delta over it is handed down rather
/// than found again, as git's delta base cache keeps one. They're kept up
/// to `BASE_BUDGET` bytes a find, whatever their kind, and dropped as the
/// find ends.
#[derive(Default)]
struct Resolved {
    objects: HashMap<Oid, Object>,
    bytes: usize,
}

impl Resolved {
    fn get(&self, oid: &Oid) -> Option<Object> {
        self.objects.get(oid).map(|object| Object {
            kind: object.kind,
            data: object.data.clone(),
        })
    }

    /// Keeps `object`, found as `oid`, should there be room for it.
    fn keep(&mut self, oid: Oid, object: &Object) {
        if self.bytes + object.data.len() > BASE_BUDGET {
            return;
        }
        self.bytes += object.data.len();
        self.objects.insert(
            oid,
            Object {
                kind: object.kind,
                data: object.data.clone(),
            },
        );
    }
}

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
    /// coming round or running too long, rather than refused, and what
    /// passed over each.
    skipped: bool,
    hold: Hold,
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
            hold: Hold::default(),
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
    /// says nothing of the copy whose delta named it, and what passed
    /// them over.
    Skipped(io::Error, Hold),
    /// A named base whose copies were spent, each refused, or which no
    /// store holds: the copy whose delta named it reads no better this
    /// answer, though the base may be written, or read once memory is
    /// there, by the next.
    Unbased(io::Error),
}

impl Odb {
    /// The object, read through every delta on the way to its base, or
    /// None when no store here has it.
    ///
    /// What a store's indexes, packs or loose objects refuse stands only
    /// once every other place the object may lie has been read. A copy that
    /// won't read is marked bad, as git marks a copy bad and looks again,
    /// so a sound copy in another pack is read and no copy is unpacked to
    /// its failure twice an answer. A copy refused memory is marked too,
    /// since only a stream truly inflating that far is refused it, and
    /// unpacking it on every find would cost the same each time. How long
    /// a mark lasts is `Lasts`'s to say. A copy whose chain comes round to
    /// an object already being found, or runs past `CHAIN_LIMIT`, may be
    /// sound under another copy of a base on the way, so it's only passed
    /// over, for this find alone, and the next copy read; a base spent with
    /// a copy passed over passes over the copy that named it in turn. A
    /// copy passed over is passed over again only while what passed it
    /// over holds, as `Hold` says, and read again where it no longer does,
    /// since reached nearer the top, or with the name it came round to
    /// found, it may read; so which copy is read first changes what's
    /// found only by what it costs. A base found by name is kept for the
    /// rest of the find, up to `BASE_BUDGET` bytes, and a base spent is
    /// kept while what spent it holds, so a copy naming either after the
    /// first ends as the first did, without its copies looked up again.
    pub(super) fn find(&mut self, oid: &Oid) -> io::Result<Option<Object>> {
        let mut searches = vec![Search::new(*oid, 0)];
        let mut searching = HashSet::from([*oid]);
        let mut skipped = Skipped::new();
        let mut spent = Spent::new();
        let mut resolved = Resolved::default();
        // The base a search above handed down, or a tree already read.
        let mut handed = None;
        loop {
            let at = searches.len() - 1;
            // Whether the base this copy reaches is one, found by name.
            let named = handed.is_some();
            let reached = if let Some(base) = handed.take() {
                Ok(Reached::Base(base))
            } else {
                match self.next_copy(&mut searches[at], &skipped, &searching) {
                    Next::Packed(place) => self.read_down(&mut searches[at], place),
                    Next::Loose(object) => Ok(Reached::Base(object)),
                    // A base no store holds ends the copy of the delta that
                    // named it, below.
                    Next::Spent => {
                        let ended = end(&mut searches, &mut searching);
                        if searches.is_empty() {
                            return ended.refused.map_or(Ok(None), Err);
                        }
                        // Not found rather than wrong, as the next answer
                        // may find the base written loose or in a new pack.
                        let refused = ended.refused.unwrap_or_else(|| {
                            io::Error::new(io::ErrorKind::NotFound, "delta base missing")
                        });
                        let hold = ended.skipped.then_some(ended.hold);
                        spent.insert(
                            ended.oid,
                            (refused.kind(), refused.to_string(), hold.clone()),
                        );
                        if let Some(hold) = hold {
                            Ok(Reached::Skipped(refused, hold))
                        } else {
                            Ok(Reached::Unbased(refused))
                        }
                    }
                }
            };
            let at = searches.len() - 1;
            match reached {
                Err(error) => self.fail(&mut searches[at], &error, true),
                Ok(Reached::Unbased(error)) => self.fail(&mut searches[at], &error, false),
                Ok(Reached::TooLong) => {
                    let hold = Hold {
                        from: Some(searches[at].above),
                        round: Vec::new(),
                    };
                    let error = invalid("delta chain too long");
                    skip(&mut searches[at], &mut skipped, &error, hold);
                }
                Ok(Reached::Skipped(error, hold)) => {
                    let hold = hold.under(searches[at].above);
                    skip(&mut searches[at], &mut skipped, &error, hold);
                }
                Ok(Reached::Named(base)) => {
                    let depth = searches[at].depth();
                    // A base spent with a copy passed over, where what
                    // passed it over no longer holds, is found again.
                    let stale = matches!(
                        spent.get(&base),
                        Some((.., Some(hold))) if !hold.holds(depth, &searching)
                    );
                    if stale {
                        spent.remove(&base);
                    }
                    if let Some(tree) = self.tree(&base) {
                        handed = Some(tree);
                    } else if let Some(found) = resolved.get(&base) {
                        handed = Some(found);
                    } else if let Some((kind, said, hold)) = spent.get(&base) {
                        let error = io::Error::new(*kind, said.clone());
                        if let Some(hold) = hold {
                            let hold = hold.under(searches[at].above);
                            skip(&mut searches[at], &mut skipped, &error, hold);
                        } else {
                            self.fail(&mut searches[at], &error, false);
                        }
                    } else if searching.insert(base) {
                        searches.push(Search::new(base, depth));
                    } else {
                        let hold = Hold {
                            from: None,
                            round: vec![base],
                        };
                        let error = invalid("delta base cycle");
                        skip(&mut searches[at], &mut skipped, &error, hold);
                    }
                }
                Ok(Reached::Base(base)) => match self.apply(&mut searches[at], base, named) {
                    Err((error, own)) => self.fail(&mut searches[at], &error, own),
                    Ok(object) => {
                        let found = end(&mut searches, &mut searching);
                        self.keep_tree(&found.oid, &object);
                        if searches.is_empty() {
                            return Ok(Some(object));
                        }
                        resolved.keep(found.oid, &object);
                        handed = Some(object);
                    }
                },
            }
        }
    }

    /// The next copy of `search`'s object neither marked bad nor passed
    /// over in this find, in the packs of each store and then in its loose
    /// objects, keeping the first refusal met on the way. A copy passed
    /// over before, where what passed it over still holds, passes over
    /// this search too, should it find nothing, since its base may be
    /// sound under another copy of a base on the way.
    fn next_copy(&self, search: &mut Search, skipped: &Skipped, searching: &HashSet<Oid>) -> Next {
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
            if let Some(mark) = self.bad.get(&place) {
                search
                    .refused
                    .get_or_insert_with(|| io::Error::new(mark.kind, mark.said.clone()));
            } else if let Some((kind, said, hold)) = skipped
                .get(&place)
                .filter(|(.., hold)| hold.holds(search.above, searching))
            {
                search.skipped = true;
                search.hold.join(hold);
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
                        data: inflate_at(file, offset + at as u64, size, Stored::Packed)?,
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
    /// the object asked for kept as a base for the reads to come; or what
    /// refused it, and whether that's the copy's own fault. Applied over a
    /// base found by `name` elsewhere, the first delta fails for the base's
    /// fault only where the base isn't the size the delta says, which a
    /// loose base no stamp watches can mend; anything else it refuses, a
    /// delta's own bytes refuse over any base of that size.
    fn apply(
        &mut self,
        search: &mut Search,
        base: Object,
        named: bool,
    ) -> Result<Object, (io::Error, bool)> {
        let mut object = base;
        let mut over_named = named;
        while let Some(hop) = search.deltas.pop() {
            let (store, pack, _) = hop.place;
            let file = &self.stores[store].packs[pack].data;
            let delta = inflate_at(file, hop.data, hop.size, Stored::Packed)
                .map_err(|error| (error, true))?;
            let base_blamed =
                over_named && delta_base_size(&delta).is_some_and(|size| size != object.data.len());
            object.data =
                apply_delta(&object.data, &delta).map_err(|error| (error, !base_blamed))?;
            over_named = false;
            if search.depth() > 0 {
                self.keep_base(hop.place, &object);
            }
        }
        Ok(object)
    }

    /// Ends the copy `search` was reading, which won't read for `error`:
    /// marks it bad, keeps the refusal should it be the first, and leaves
    /// the next copy to be read. The mark keeps the error's kind, so a copy
    /// refused memory is never taken for a corrupt one, and lasts as
    /// `Mark::of` says for an error that's the copy's `own` fault or not.
    fn fail(&mut self, search: &mut Search, error: &io::Error, own: bool) {
        if let Some(copy) = search.copy.take() {
            self.bad.insert(copy, Mark::of(error, own));
        }
        refuse(search, error);
    }
}

/// A pack entry's kind, the size of what it inflates to, and how many of
/// `head`'s bytes its header takes, refused as git's
/// `unpack_object_header_buffer` refuses one whose size runs past a word:
/// once a byte's seven bits would shift past it, before any are dropped.
pub(super) fn entry_header(head: &[u8]) -> io::Result<(u8, usize, usize)> {
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
pub(super) fn base_distance(head: &[u8]) -> io::Result<(u64, usize)> {
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

/// Passes over the copy `search` was reading while what passed it over,
/// `hold`, holds, marking nothing: its chain came round to an object being
/// found, or ran too long, which may not be the copy's own doing.
fn skip(search: &mut Search, skipped: &mut Skipped, error: &io::Error, hold: Hold) {
    search.hold.join(&hold);
    if let Some(copy) = search.copy.take() {
        skipped.insert(copy, (error.kind(), error.to_string(), hold));
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
