//! The object store's tests of deltas read through the bases they name:
//! how often a base is read, how far a chain may run and what ends one that
//! never would, over packs built byte by byte as git writes them.

use super::tests::{Removed, objects_over, read_from};
use super::*;
use std::sync::mpsc;
use std::time::Duration;

/// `count` names, sorted, none of them `sorted()`'s.
pub(super) fn names(count: usize) -> Vec<[u8; 20]> {
    (0..count)
        .map(|at| {
            let mut name = [0xab; 20];
            let at = u32::try_from(at).expect("a test's few names");
            name[..4].copy_from_slice(&(at + 1).to_be_bytes());
            name
        })
        .collect()
}

/// A blob of fewer than 16 bytes, as a pack's entry.
pub(super) fn blob(data: &[u8]) -> Vec<u8> {
    let mut entry = vec![0x30 | u8::try_from(data.len()).expect("a short blob")];
    entry.extend(miniz_oxide::deflate::compress_to_vec_zlib(data, 1));
    entry
}

/// A delta naming its base, `base`, as a pack's entry: type 7, of 4
/// bytes, over a base of one byte to a result of one, inserting "x".
fn over(base: [u8; 20]) -> Vec<u8> {
    delta_over(base, 1)
}

/// The same delta, saying its base is of `base_size` bytes.
fn delta_over(base: [u8; 20], base_size: u8) -> Vec<u8> {
    let mut entry = vec![0x70 | 4];
    entry.extend(base);
    entry.extend(miniz_oxide::deflate::compress_to_vec_zlib(
        &[base_size, 1, 1, b'x'],
        1,
    ));
    entry
}

/// A delta's data over a base of `base_size` bytes, copying it whole and
/// adding `c`.
pub(super) fn appending(base_size: u8, c: u8) -> [u8; 6] {
    [base_size, base_size + 1, 0x90, base_size, 1, c]
}

/// A delta naming its base, `base`, as a pack's entry over `delta`.
pub(super) fn named(base: [u8; 20], delta: &[u8]) -> Vec<u8> {
    let mut entry = vec![0x70 | u8::try_from(delta.len()).expect("a short delta")];
    entry.extend(base);
    entry.extend(miniz_oxide::deflate::compress_to_vec_zlib(delta, 1));
    entry
}

/// A pack's entries in order, knowing how far past the first each starts,
/// so a delta may place its base by offset as git writes one.
pub(super) struct Entries {
    pub(super) entries: Vec<Vec<u8>>,
    pub(super) starts: Vec<u64>,
}

impl Entries {
    pub(super) fn new() -> Entries {
        Entries {
            entries: Vec::new(),
            starts: vec![0],
        }
    }

    pub(super) fn push(&mut self, entry: Vec<u8>) {
        let end = self.starts[self.entries.len()] + entry.len() as u64;
        self.starts.push(end);
        self.entries.push(entry);
    }

    /// Adds a delta over `delta` placing its base, the entry at `base`, by
    /// how far back it lies, in git's own sum of seven-bit groups.
    pub(super) fn place_over(&mut self, base: usize, delta: &[u8]) {
        let mut back = self.starts[self.entries.len()] - self.starts[base];
        let mut distance = vec![low_seven(back)];
        back >>= 7;
        while back > 0 {
            back -= 1;
            distance.push(0x80 | low_seven(back));
            back >>= 7;
        }
        distance.reverse();
        let mut entry = vec![0x60 | u8::try_from(delta.len()).expect("a short delta")];
        entry.extend(distance);
        entry.extend(miniz_oxide::deflate::compress_to_vec_zlib(delta, 1));
        self.push(entry);
    }
}

pub(super) fn low_seven(value: u64) -> u8 {
    u8::try_from(value & 0x7f).expect("seven bits fit a byte")
}

/// An objects folder of packs, each holding its `entries` in order, the
/// one at each place named by `names` at that place, through an index of
/// its own.
pub(super) fn packed(names: &[[u8; 20]], packs: &[Vec<Vec<u8>>]) -> Removed {
    let listed: Vec<_> = packs
        .iter()
        .map(|entries| (names[..entries.len()].to_vec(), entries.clone()))
        .collect();
    packed_as(&listed)
}

/// A pack's entries, and the name of each.
type Pack = (Vec<[u8; 20]>, Vec<Vec<u8>>);

/// What reading an object answers: its data, or what refused it.
pub(super) type Read = Result<Option<Vec<u8>>, String>;

/// The same, each pack's entries named by the names beside them.
fn packed_as(packs: &[Pack]) -> Removed {
    let mut indexed = Vec::new();
    let mut bytes = Vec::new();
    for (at, (names, entries)) in packs.iter().enumerate() {
        let mut pack = b"PACK".to_vec();
        pack.extend(2u32.to_be_bytes());
        let count = u32::try_from(entries.len()).expect("a test's few entries");
        pack.extend(count.to_be_bytes());
        let mut offsets = Vec::new();
        for entry in entries {
            offsets.push(pack.len() as u64);
            pack.extend(entry);
        }
        pack.extend([0; 20]);
        let name = format!("pack-{at:02}");
        indexed.push((
            name.clone(),
            crate::pack_index::tests::version_2_at(names, &offsets),
        ));
        bytes.push((name, pack));
    }
    let listed = indexed
        .iter()
        .map(|(name, index)| (name.as_str(), index.clone()))
        .collect();
    let folders = Removed(vec![objects_over(Vec::new(), listed)]);
    for (name, pack) in bytes {
        std::fs::write(folders.0[0].join(format!("pack/{name}.pack")), pack)
            .expect("the temp folder takes a file");
    }
    folders
}

/// What `read` answers, on a thread with as much stack as a process's
/// first, where the engine reads, or whether it panicked or took past
/// `seconds`. A panic's own message prints with the test's output, as the
/// thread's does. A read that takes too long is left to run: a thread
/// can't be stopped from outside, and the test's process ends it.
pub(super) fn within<T: Send + 'static>(
    seconds: u64,
    read: impl FnOnce() -> T + Send + 'static,
) -> Result<T, &'static str> {
    let (send, answer) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || send.send(read()))
        .expect("the test starts a thread");
    answer
        .recv_timeout(Duration::from_secs(seconds))
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => "took too long",
            mpsc::RecvTimeoutError::Disconnected => "panicked",
        })
}

#[test]
fn a_delta_whose_base_is_at_last_itself_is_refused() {
    let names = names(2);
    // names[0] a delta over itself, and over names[1] whose base is
    // names[0]: a cycle of one, and of two, which only a base remembered
    // past the one just read finds.
    for (entries, what) in [
        (vec![over(names[0])], "itself"),
        (
            vec![over(names[1]), over(names[0])],
            "another delta over it",
        ),
    ] {
        let folders = packed(&names, &[entries]);
        assert_eq!(
            read_from(&folders.0[0], names[0]),
            Err("delta base cycle".into()),
            "a delta whose base is, at last, {what} wasn't refused"
        );
    }
}

#[test]
fn a_delta_over_itself_leaves_a_sound_copy_in_another_pack_read() {
    let names = names(2);
    // names[0] over itself in the first pack, and sound in the second, of
    // one byte as the delta's base must be, so the delta applied over the
    // sound copy would read as "x". Then the same of names[0] over
    // names[1], over names[0]: the cycle is names[0]'s to end, not the
    // copy of names[1] on the way.
    // Read after it, names[1] stands as nothing in the first, and as the
    // delta over the sound copy in the second, its copy left unmarked.
    for (first, then, what) in [
        (vec![over(names[0])], None, "itself"),
        (
            vec![over(names[1]), over(names[0])],
            Some(b"x".to_vec()),
            "another delta over it",
        ),
    ] {
        let folders = packed(&names, &[first, vec![blob(b"a")]]);
        let mut odb = Odb::open(&folders.0[0]);
        let mut read = |name: [u8; 20]| {
            odb.read(&Oid(name))
                .map(|object| object.map(|object| object.data))
                .map_err(|error| error.to_string())
        };
        let (top, next) = (read(names[0]), read(names[1]));
        assert_eq!(
            (top, next),
            (Ok(Some(b"a".to_vec())), Ok(then)),
            "a delta over {what} hid a sound copy, was read over it, \
             or marked a copy on the way"
        );
    }
}

#[test]
fn a_base_found_or_missed_for_one_copy_answers_the_same_for_the_next() {
    let names = names(2);
    // names[0] twice a delta over names[1]: first where nothing holds
    // names[1], then where the first copy says its base is of 2 bytes,
    // so it won't apply over the sound names[1] beside it.
    for (packs, found, what) in [
        (
            vec![vec![over(names[1])], vec![over(names[1])]],
            Err("delta base missing".to_string()),
            "missed",
        ),
        (
            vec![
                vec![delta_over(names[1], 2), blob(b"a")],
                vec![over(names[1])],
            ],
            Ok(Some(b"x".to_vec())),
            "found",
        ),
    ] {
        let folders = packed(&names, &packs);
        // A miss reads the same however often it's looked for, so the
        // lookups say whether the next copy looked for it again.
        super::chain::LOOKUPS.with(|lookups| lookups.set(0));
        let read = read_from(&folders.0[0], names[0]);
        let lookups = super::chain::LOOKUPS.with(std::cell::Cell::get);
        assert_eq!(
            (read, lookups),
            (found, 4),
            "a base {what} for one copy didn't answer the same for the next"
        );
    }
}

#[test]
fn a_delta_over_a_base_named_is_read_as_often_as_asked() {
    let names = names(2);
    let folders = packed(&names, &[vec![over(names[1]), blob(b"a")]]);
    let mut odb = Odb::open(&folders.0[0]);
    let mut read = || {
        odb.read(&Oid(names[0]))
            .map(|object| object.map(|object| object.data))
            .map_err(|error| error.to_string())
    };
    let (first, second) = (read(), read());
    assert_eq!(
        (first, second),
        (Ok(Some(b"x".to_vec())), Ok(Some(b"x".to_vec()))),
        "a delta read again wasn't read as the first time was, its base kept or marked wrongly"
    );
}

#[test]
fn a_chain_of_named_bases_is_read_to_its_limit_and_refused_past_it() {
    // names[0] the top of a chain of CHAIN_LIMIT + 1 deltas, each over
    // the name after it, down to the blob names[CHAIN_LIMIT + 1].
    // Then the same beside a second pack holding names[0] sound.
    let names = names(CHAIN_LIMIT + 2);
    let mut entries: Vec<Vec<u8>> = names[1..].iter().map(|&base| over(base)).collect();
    entries.push(blob(b"a"));
    for (packs, top) in [
        (
            vec![entries.clone()],
            Err("delta chain too long".to_string()),
        ),
        (
            vec![entries.clone(), vec![blob(b"b")]],
            Ok(Some(b"b".to_vec())),
        ),
    ] {
        let folders = packed(&names, &packs);
        let objects = folders.0[0].clone();
        let names = names.clone();
        let read = within(60, move || {
            let mut odb = Odb::open(&objects);
            let mut read = |name: [u8; 20]| {
                odb.read(&Oid(name))
                    .map(|object| object.map(|object| object.data))
                    .map_err(|error| error.to_string())
            };
            // The longer first, so its refusal can't be one the shorter
            // left.
            let past = read(names[0]);
            let to = read(names[1]);
            (past, to)
        });
        assert_eq!(
            read,
            Ok((top.clone(), Ok(Some(b"x".to_vec())))),
            "a chain of named bases past its limit wasn't refused, or hid a sound copy, \
             or a chain to it wasn't read (the top: {top:?})"
        );
    }
}

#[test]
fn a_delta_placed_at_itself_or_the_packs_start_is_refused() {
    let names = names(1);
    // A delta placing its base, type 6, of 4 bytes, 0 bytes back, at
    // itself, then 12 bytes back, at the pack's header.
    for back in [0, 12] {
        let mut entry = vec![0x60 | 4, back];
        entry.extend(miniz_oxide::deflate::compress_to_vec_zlib(
            &[1, 1, 1, b'x'],
            1,
        ));
        let folders = packed(&names, &[vec![entry]]);
        assert_eq!(
            read_from(&folders.0[0], names[0]),
            Err("delta base offset".into()),
            "a delta {back} bytes back from itself was read"
        );
    }
}

#[test]
fn a_copy_that_wont_unpack_is_unpacked_once_however_many_deltas_lie_over_it() {
    // Three packs, each holding a chain of 16 deltas over one entry of a
    // kind git has none of. Asking every copy at every level again for
    // each copy above it would unpack 3 to the 17th copies.
    let names = names(17);
    let mut entries: Vec<Vec<u8>> = names[1..].iter().map(|&base| over(base)).collect();
    entries.push(vec![0x50 | 1, 0]);
    let folders = packed(&names, &[entries.clone(), entries.clone(), entries]);
    let objects = folders.0[0].clone();
    let read = within(20, move || {
        let mut odb = Odb::open(&objects);
        let read = odb
            .read(&Oid(names[0]))
            .map(|object| object.map(|object| object.data))
            .map_err(|error| error.to_string());
        (read, odb.bad.len())
    });
    assert_eq!(
        read,
        Ok((Err("pack entry kind".into()), 3 * 17)),
        "a chain of copies that won't unpack didn't end in its first refusal, each copy once"
    );
}

#[test]
fn a_copy_marked_bad_stays_bad_while_the_store_is_open() {
    let names = names(2);
    // names[0] a delta over names[1], which nothing holds yet.
    let folders = packed(&names[..1], &[vec![over(names[1])]]);
    let objects = &folders.0[0];
    let mut odb = Odb::open(objects);
    let before = odb.read(&Oid(names[0])).map_err(|error| error.to_string());
    let hex = Oid(names[1]).hex();
    std::fs::create_dir_all(objects.join(&hex[..2])).expect("the temp folder takes a folder");
    std::fs::write(
        objects.join(&hex[..2]).join(&hex[2..]),
        miniz_oxide::deflate::compress_to_vec_zlib(b"blob 1\0a", 1),
    )
    .expect("the temp folder takes a file");
    let after = odb.read(&Oid(names[0])).map_err(|error| error.to_string());
    let reopened = read_from(objects, names[0]);
    assert_eq!(
        (before.err(), after.err(), reopened),
        (
            Some("delta base missing".into()),
            Some("delta base missing".into()),
            Ok(Some(b"x".to_vec()))
        ),
        "a copy marked bad was unpacked again before its store was opened again, \
         or stayed bad past it"
    );
}

#[test]
fn a_copy_refused_memory_is_left_unmarked_as_is_a_delta_naming_it() {
    let names = names(2);
    // names[0] a delta over names[1] in each pack, names[1]'s copy in the
    // first spoilt and in the second sound, so the second copy of the
    // delta meets a base already spent. The spoilt one's size takes two
    // bytes, so its stream starts past where the sound one's does.
    let mut spoilt = [vec![0xb1, 0x00], blob(b"a")[1..].to_vec()].concat();
    *spoilt.last_mut().expect("a stream ends in its checksum") ^= 1;
    // The pack's 12 bytes of header, the delta, then the blob's one.
    let sound = 12 + over(names[1]).len() as u64 + 1;
    let folders = packed_as(&[
        (names.clone(), vec![over(names[1]), spoilt]),
        (names.clone(), vec![over(names[1]), blob(b"a")]),
    ]);
    let objects = folders.0[0].clone();
    let read = within(20, move || {
        let mut odb = Odb::open(&objects);
        let mut read = || {
            let read = odb
                .read(&Oid(names[0]))
                .map(|object| object.map(|object| object.data))
                .map_err(|error| error.to_string());
            (read, odb.bad.len())
        };
        STARVED.with(|starved| starved.set(Some(sound)));
        let starved = read();
        STARVED.with(|starved| starved.set(None));
        (starved, read())
    });
    assert_eq!(
        read,
        Ok((
            (Err("pack object's stream".into()), 1),
            (Ok(Some(b"x".to_vec())), 1)
        )),
        "a copy refused memory, or a delta over its object, was marked bad and not read again"
    );
}

/// What reading each of `names` from a store opened once answers, and how
/// many copies it marked bad by the end.
pub(super) fn read_each(objects: &std::path::Path, names: &[[u8; 20]]) -> (Vec<Read>, usize) {
    let mut odb = Odb::open(objects);
    let read = names
        .iter()
        .map(|&name| {
            odb.read(&Oid(name))
                .map(|object| object.map(|object| object.data))
                .map_err(|error| error.to_string())
        })
        .collect();
    (read, odb.bad.len())
}

#[test]
fn a_copy_whose_chain_comes_round_leaves_a_base_read_from_its_other_copy() {
    let names = names(2);
    // names[0]'s only copy a delta over names[1], whose copy beside it is a
    // delta over names[0], and whose copy in the second pack is sound.
    let folders = packed_as(&[
        (names.clone(), vec![over(names[1]), over(names[0])]),
        (vec![names[1]], vec![blob(b"a")]),
    ]);
    let objects = folders.0[0].clone();
    assert_eq!(
        within(20, move || read_each(&objects, &names)),
        Ok((vec![Ok(Some(b"x".to_vec())), Ok(Some(b"a".to_vec()))], 0)),
        "a chain coming round refused an object a sound copy of its base reads, \
         or marked a copy on the way"
    );
}

#[test]
fn a_base_whose_copies_were_passed_over_passes_over_the_next_copy_naming_it() {
    let names = names(2);
    // names[0] a delta over names[1] in each pack, and names[1]'s only copy
    // a delta over names[0]: the second copy of names[0] finds names[1]
    // again once its one copy was passed over, which marks nothing.
    let folders = packed_as(&[
        (names.clone(), vec![over(names[1]), over(names[0])]),
        (vec![names[0]], vec![over(names[1])]),
    ]);
    let objects = folders.0[0].clone();
    let (read, bad) = within(20, move || read_each(&objects, &names)).expect("the reads ended");
    assert_eq!(
        (read.clone(), bad),
        (vec![Err("delta base cycle".into()); 2], 0),
        "a chain coming round wasn't refused for coming round, or marked a copy whose \
         base was passed over: {read:?}"
    );
}

#[test]
fn a_copy_passed_over_is_read_once_a_find_however_many_copies_name_its_object() {
    // Each name a delta over the next in both packs, the last over the
    // first in one: read again for each copy naming it, the chain's copies
    // would be read 2^40 times over.
    let names = names(40);
    let over_next: Vec<Vec<u8>> = (0..names.len())
        .map(|at| over(names[(at + 1) % names.len()]))
        .collect();
    let folders = packed_as(&[
        (names.clone(), over_next.clone()),
        (names[..39].to_vec(), over_next[..39].to_vec()),
    ]);
    let objects = folders.0[0].clone();
    let (read, bad) = within(20, move || read_each(&objects, &names[..1])).expect("the read ended");
    assert_eq!(
        (read.clone(), bad),
        (vec![Err("delta base cycle".into())], 0),
        "a chain coming round wasn't refused for coming round, or marked a copy: {read:?}"
    );
}

#[test]
fn a_base_spent_is_looked_up_once_a_find_however_many_copies_name_it() {
    // Each name a delta over the next in every pack, the last over the
    // first in one: each name's copies past its first name a base spent.
    let (rungs, packs) = (20, 6);
    let names = names(rungs);
    let over_next: Vec<Vec<u8>> = (0..rungs).map(|at| over(names[(at + 1) % rungs])).collect();
    let mut listed = vec![(names.clone(), over_next.clone())];
    for _ in 1..packs {
        listed.push((names[..rungs - 1].to_vec(), over_next[..rungs - 1].to_vec()));
    }
    let folders = packed_as(&listed);
    let objects = folders.0[0].clone();
    let (read, lookups) = within(20, move || {
        super::chain::LOOKUPS.with(|lookups| lookups.set(0));
        let read = read_from(&objects, names[0]);
        (read, super::chain::LOOKUPS.with(std::cell::Cell::get))
    })
    .expect("the read ended");
    // Once a find, each name's copies take one lookup each and one more
    // to find there are no more; looked up again for each copy naming it,
    // they'd take as many again for each of the packs. Fewer is no fault.
    assert_eq!(
        read,
        Err("delta base cycle".into()),
        "a chain coming round wasn't refused for coming round"
    );
    // Each name is looked up at least once, so fewer than that is a
    // count that stopped counting.
    assert!(
        (rungs..=(rungs - 1) * (packs + 1) + 2).contains(&lookups),
        "a base spent was looked up again for a copy naming it: {lookups} lookups"
    );
}

#[test]
fn a_chain_too_long_leaves_a_shorter_copy_of_a_base_on_the_way_read() {
    // names[0] the top of a chain of CHAIN_LIMIT + 1 deltas, each over the
    // name after it, with names[1] sound in a second pack.
    let names = names(CHAIN_LIMIT + 2);
    let mut entries: Vec<Vec<u8>> = names[1..].iter().map(|&base| over(base)).collect();
    entries.push(blob(b"a"));
    let folders = packed_as(&[(names.clone(), entries), (vec![names[1]], vec![blob(b"b")])]);
    let objects = folders.0[0].clone();
    let read = within(60, move || read_each(&objects, &names[..1]));
    assert_eq!(
        read,
        Ok((vec![Ok(Some(b"x".to_vec()))], 0)),
        "a chain too long refused an object a shorter copy of its base reads, \
         or marked a copy on the way"
    );
}

#[test]
fn deltas_placed_by_offset_are_applied_in_order_alone_and_among_named_ones() {
    let names = names(5);
    // "a", then each a delta adding a letter over the one before it: by
    // offset, by offset, by name and by offset again.
    let mut entries = Entries::new();
    entries.push(blob(b"a"));
    entries.place_over(0, &appending(1, b'b'));
    entries.place_over(1, &appending(2, b'c'));
    entries.push(named(names[2], &appending(3, b'd')));
    entries.place_over(3, &appending(4, b'e'));
    let folders = packed(&names, &[entries.entries]);
    let (read, _) = read_each(&folders.0[0], &names);
    assert_eq!(
        read,
        ["a", "ab", "abc", "abcd", "abcde"].map(|data| Ok(Some(data.as_bytes().to_vec()))),
        "deltas placed by offset, or among named ones, weren't applied nearest the base first"
    );
}

#[test]
fn a_chain_placed_or_mixed_is_read_to_its_limit_and_refused_past_it() {
    // The blob names[0], then CHAIN_LIMIT + 1 deltas, each over the entry
    // before it, placed by offset, then naming every other base.
    let names = names(CHAIN_LIMIT + 2);
    for mixed in [false, true] {
        let mut entries = Entries::new();
        entries.push(blob(b"a"));
        for at in 1..names.len() {
            let delta = [1, 1, 1, b'x'];
            if mixed && at % 2 == 0 {
                entries.push(named(names[at - 1], &delta));
            } else {
                entries.place_over(at - 1, &delta);
            }
        }
        let folders = packed(&names, &[entries.entries]);
        let objects = folders.0[0].clone();
        let tops = [names[CHAIN_LIMIT + 1], names[CHAIN_LIMIT]];
        let read = within(60, move || read_each(&objects, &tops));
        assert_eq!(
            read,
            Ok((
                vec![
                    Err("delta chain too long".to_string()),
                    Ok(Some(b"x".to_vec()))
                ],
                0
            )),
            "a chain past its limit wasn't refused, marked a copy, or one at it wasn't read \
             (named bases among placed ones: {mixed})"
        );
    }
}
