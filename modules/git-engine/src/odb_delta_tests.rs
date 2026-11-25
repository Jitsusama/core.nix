//! The object store's tests of deltas read through the bases they name:
//! how often a base is read, how far a chain may run and what ends one that
//! never would, over packs built byte by byte as git writes them.

use super::tests::{Removed, objects_over, read_from};
use super::*;
use std::sync::mpsc;
use std::time::Duration;

/// `count` names, sorted, none of them `sorted()`'s.
fn names(count: usize) -> Vec<[u8; 20]> {
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
fn blob(data: &[u8]) -> Vec<u8> {
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

/// An objects folder of packs, each holding its `entries` in order, the
/// one at each place named by `names` at that place, through an index of
/// its own.
fn packed(names: &[[u8; 20]], packs: &[Vec<Vec<u8>>]) -> Removed {
    let mut indexed = Vec::new();
    let mut bytes = Vec::new();
    for (at, entries) in packs.iter().enumerate() {
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
            crate::pack_index::tests::version_2_at(&names[..entries.len()], &offsets),
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
/// first, where the engine reads, or None should it take past `seconds`.
fn within<T: Send + 'static>(seconds: u64, read: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (send, answer) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || send.send(read()))
        .expect("the test starts a thread");
    answer.recv_timeout(Duration::from_secs(seconds)).ok()
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
fn a_base_found_or_missed_for_one_copy_is_found_again_for_the_next() {
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
        assert_eq!(
            read_from(&folders.0[0], names[0]),
            found,
            "a base {what} for one copy wasn't searched for again for the next"
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
        "a delta read again took its base, read before, for one being read"
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
            Some((top.clone(), Ok(Some(b"x".to_vec())))),
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
        Some((Err("pack entry kind".into()), 3 * 17)),
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
