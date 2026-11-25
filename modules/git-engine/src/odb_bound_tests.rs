//! The object store's tests of what bounds a read: sizes said in several
//! bytes, sizes past a word or no stream fills, deltas writing past what
//! they say, and the bases and trees kept between reads.

use super::delta_tests::{
    Entries, appending, blob, low_seven, named, names, packed, read_each, within,
};
use super::tests::{Removed, objects_over, read_from};
use super::*;

/// A blob of any size, as a pack's entry, its size in as many bytes as
/// it takes.
fn big_blob(data: &[u8]) -> Vec<u8> {
    let mut size = data.len();
    let mut entry = vec![0x30 | u8::try_from(size & 15).expect("four bits fit a byte")];
    size >>= 4;
    while size > 0 {
        *entry.last_mut().expect("an entry's first byte") |= 0x80;
        entry.push(low_seven(size as u64));
        size >>= 7;
    }
    entry.extend(miniz_oxide::deflate::compress_to_vec_zlib(data, 1));
    entry
}

/// `count` bytes that won't compress, the same each time.
fn noise(count: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_u32;
    (0..count)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()[0]
        })
        .collect()
}

#[test]
fn sizes_and_distances_of_several_bytes_are_read_whole() {
    let names = names(2);
    // 20,000 bytes, sized in three of a header's bytes and placed over by
    // a delta whose sizes and distance back take three bytes each and its
    // copy two.
    let data = noise(20_000);
    let mut entries = Entries::new();
    entries.push(big_blob(&data));
    entries.place_over(
        0,
        &[
            0xa0, 0x9c, 0x01, 0xa1, 0x9c, 0x01, 0xb0, 0x20, 0x4e, 0x01, b'!',
        ],
    );
    assert!(
        entries.starts[1] > 16_511,
        "the base lies too near for its distance to take three bytes"
    );
    let folders = packed(&names, &[entries.entries]);
    let mut grown = data.clone();
    grown.push(b'!');
    assert_eq!(
        read_each(&folders.0[0], &names).0,
        vec![Ok(Some(data)), Ok(Some(grown))],
        "a size or a distance of several bytes wasn't read whole"
    );
}

#[test]
fn an_object_larger_than_what_inflates_at_once_is_read_whole() {
    let names = names(1);
    let data = noise(200_000);
    let entry = big_blob(&data);
    assert!(
        entry.len() > 2 << 16,
        "the stream fits in fewer than three of the reads it's taken in"
    );
    let folders = packed(&names, &[vec![entry]]);
    assert_eq!(
        read_from(&folders.0[0], names[0]),
        Ok(Some(data)),
        "an object inflating past one window wasn't read whole"
    );
}

#[test]
fn a_size_is_refused_before_its_bits_would_drop_and_a_large_one_takes_no_room() {
    let name = names(1)[0];
    // A blob whose tenth size byte would shift its bits past 64, then one
    // saying it's of near 2^60 bytes whose stream gives one.
    let mut past = vec![0xbf];
    past.extend([0xff; 8]);
    past.push(0x01);
    let mut large = vec![0xbf];
    large.extend([0xff; 7]);
    large.push(0x7f);
    large.extend(miniz_oxide::deflate::compress_to_vec_zlib(b"x", 1));
    for (entry, refused) in [(past, "pack entry size"), (large, "pack object's size")] {
        let folders = packed(&[name], &[vec![entry]]);
        let objects = folders.0[0].clone();
        assert_eq!(
            within(20, move || read_from(&objects, name)),
            Ok(Err(refused.into())),
            "a size past a word wasn't refused, or a large one made room it never filled"
        );
    }
}

#[test]
fn a_base_distance_is_refused_once_its_bits_would_pass_a_word() {
    let names = names(1);
    // Deltas whose distance takes ten bytes: the first nine saying
    // 2^57 - 2, whose next byte fits a word, then 2^57 - 1, whose next
    // doesn't, then one far past a word. A distance that fits a word yet
    // lies before the pack is refused for where it lies instead.
    let near = [0x80, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe];
    let ending = |last: [u8; 2]| near.iter().chain(&last).copied().collect::<Vec<u8>>();
    for (distance, refused) in [
        (ending([0xfe, 0x00]), "delta base offset"),
        (ending([0xff, 0x00]), "delta offset"),
        (
            [0xff; 10].iter().copied().chain([0x7f]).collect(),
            "delta offset",
        ),
    ] {
        let mut entry = vec![0x64];
        entry.extend(distance);
        let folders = packed(&names, &[vec![entry]]);
        assert_eq!(
            read_from(&folders.0[0], names[0]),
            Err(refused.into()),
            "a distance of ten bytes or more wasn't refused as its {refused}"
        );
    }
}

#[test]
fn a_delta_past_a_word_saying_more_than_it_gives_or_writing_past_its_size_is_refused() {
    let base = b"abc";
    let mut sizes = vec![3];
    sizes.extend([0xff; 9]);
    sizes.push(0x01);
    // A delta saying it makes near 2^63 bytes, and giving none.
    let mut large = vec![3];
    large.extend([0xff; 8]);
    large.push(0x7f);
    // Each command writing past the size is followed by one that can't
    // be read, so only a delta refused before it writes says its size.
    for (delta, refused) in [
        (sizes, "delta size"),
        (large, "delta's result size"),
        (vec![3, 2, 0x90, 3, 0], "delta's result size"),
        (vec![3, 1, 2, b'x', b'y', 0], "delta's result size"),
    ] {
        assert_eq!(
            apply_delta(base, &delta).map_err(|error| error.to_string()),
            Err(refused.into()),
            "a delta past a word, short of its size or writing past it wasn't refused: {delta:?}"
        );
    }
}

#[test]
fn an_object_is_taken_only_where_its_stream_ends_at_its_size() {
    let name = names(1)[0];
    let stream = |data: &[u8]| miniz_oxide::deflate::compress_to_vec_zlib(data, 1);
    let entry = |head: u8, stream: Vec<u8>| [vec![head], stream].concat();
    // The sound stream with its checksum spoilt, so it fills the object
    // yet doesn't end as a stream should.
    let mut spoilt = stream(b"x");
    *spoilt.last_mut().expect("a stream ends in its checksum") ^= 1;
    let size = || Err("pack object's size".to_string());
    // Each is read under a clock, since a stream filling the room made for
    // it before it ends would, unrefused, be read again and again.
    for (entry, read, what) in [
        (entry(0x31, stream(b"x")), Ok(Some(b"x".to_vec())), "sound"),
        (
            entry(0x30, stream(b"")),
            Ok(Some(Vec::new())),
            "sound of no bytes",
        ),
        (entry(0x30, stream(b"x")), size(), "of no bytes giving one"),
        (
            entry(0x31, stream(b"xy")),
            size(),
            "giving one past its size",
        ),
        (
            entry(0x31, stream(b"xyz")),
            size(),
            "giving more past its size than the room made",
        ),
        (
            entry(0x31, spoilt),
            Err("pack object's stream".into()),
            "spoilt",
        ),
    ] {
        let folders = packed(&[name], &[vec![entry]]);
        let objects = folders.0[0].clone();
        assert_eq!(
            within(20, move || read_from(&objects, name)),
            Ok(read),
            "an object {what} wasn't taken only where its stream ends at its size"
        );
    }
}

/// A delta's size, in as many bytes as it takes.
fn delta_size(mut size: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let low = u8::try_from(size & 0x7f).expect("seven bits fit a byte");
        size >>= 7;
        if size == 0 {
            bytes.push(low);
            return bytes;
        }
        bytes.push(low | 0x80);
    }
}

#[test]
fn a_delta_grows_toward_its_size_and_copies_from_where_it_says() {
    // A 1.5 MiB base, copied whole and then 1 MiB of it from its second
    // byte: past the room first made, then past twice what's held.
    let base = noise(3 << 19);
    let made = base.len() + (1 << 20);
    let delta = [
        delta_size(base.len()),
        delta_size(made),
        vec![0xf0, 0x00, 0x00, 0x18],
        vec![0xf1, 0x01, 0x00, 0x00, 0x10],
    ]
    .concat();
    let read = apply_delta(&base, &delta).expect("the delta applies");
    assert_eq!(
        (
            read.len(),
            read.capacity() <= made,
            &read[..base.len()] == base.as_slice()
        ),
        (made, true, true),
        "a delta made room past its size, or didn't copy its base whole"
    );
    assert!(
        read[base.len()..] == base[1..=1 << 20],
        "a delta's copy didn't start at the offset it says"
    );
}

#[test]
fn a_delta_whose_data_wont_inflate_is_refused() {
    let names = names(2);
    let mut broken = vec![0x70 | 4];
    broken.extend(names[1]);
    broken.extend([0xff; 8]);
    let folders = packed(&names, &[vec![broken, blob(b"a")]]);
    assert_eq!(
        read_from(&folders.0[0], names[0]),
        Err("pack object's stream".into()),
        "a delta whose data won't inflate wasn't refused for it"
    );
}

/// An object of `kind`, of `len` bytes.
fn object(kind: Kind, len: usize) -> Object {
    Object {
        kind,
        data: vec![b'c'; len],
    }
}

#[test]
fn bases_and_trees_are_kept_within_their_budgets_and_counted_as_replaced() {
    let folders = Removed(vec![objects_over(Vec::new(), Vec::new())]);
    let mut odb = Odb::open(&folders.0[0]);
    odb.keep_base_within((0, 0, 1), &object(Kind::Commit, 4), 10);
    odb.keep_base_within((0, 0, 2), &object(Kind::Commit, 4), 10);
    odb.keep_base_within((0, 0, 2), &object(Kind::Commit, 2), 10);
    let replaced = (odb.bases.len(), odb.base_bytes);
    odb.keep_base_within((0, 0, 3), &object(Kind::Commit, 11), 10);
    odb.keep_base_within((0, 0, 4), &object(Kind::Blob, 1), 10);
    let refused = (odb.bases.len(), odb.base_bytes);
    odb.keep_base_within((0, 0, 3), &object(Kind::Commit, 5), 10);
    let again = (odb.bases.len(), odb.base_bytes);
    assert_eq!(
        (replaced, refused, again),
        ((2, 6), (2, 6), (1, 5)),
        "bases weren't kept within their budget, or a base kept again was counted twice"
    );
    let tree = |len| object(Kind::Tree, len);
    odb.keep_tree_within(&Oid([1; 20]), &tree(6), 10);
    odb.keep_tree_within(&Oid([2; 20]), &tree(11), 10);
    let kept = (odb.trees.len(), odb.tree_bytes);
    odb.keep_tree_within(&Oid([3; 20]), &tree(5), 10);
    odb.keep_tree_within(&Oid([4; 20]), &object(Kind::Blob, 1), 10);
    assert_eq!(
        (kept, (odb.trees.len(), odb.tree_bytes)),
        ((1, 6), (1, 5)),
        "trees weren't kept within their budget, or something not a tree was"
    );
}

#[test]
fn a_tree_kept_is_read_from_memory_and_handed_to_a_delta_naming_it() {
    let names = names(2);
    // A tree, and a delta over it by name.
    let mut entries = Entries::new();
    entries.push({
        let mut tree = vec![0x20 | 1];
        tree.extend(miniz_oxide::deflate::compress_to_vec_zlib(b"t", 1));
        tree
    });
    entries.push(named(names[0], &appending(1, b'b')));
    let tree_end = entries.starts[1];
    let folders = packed(&names, &[entries.entries]);
    let mut odb = Odb::open(&folders.0[0]);
    let mut read = |name| {
        odb.read(&Oid(name))
            .map(|object| object.map(|object| (object.kind, object.data)))
            .map_err(|error| error.to_string())
    };
    let first = read(names[0]);
    // The tree's stream spoilt in its pack, as the base's is above.
    let pack = folders.0[0].join("pack/pack-00.pack");
    let mut bytes = std::fs::read(&pack).expect("the pack reads");
    let spoilt = usize::try_from(tree_end).expect("a small pack") + 12;
    bytes[13..spoilt].fill(0xff);
    std::fs::write(&pack, bytes).expect("the temp folder takes a file");
    let tree = Ok(Some((Kind::Tree, b"t".to_vec())));
    assert_eq!(
        (first, read(names[0]), read(names[1])),
        (tree.clone(), tree, Ok(Some((Kind::Tree, b"tb".to_vec())))),
        "a tree kept was read again from its pack, or not handed to a delta naming it"
    );
}

#[test]
fn a_base_kept_is_read_from_memory_the_next_time_a_delta_over_it_is() {
    let names = names(2);
    // A commit, and a delta over it by offset.
    let mut entries = Entries::new();
    entries.push({
        let mut commit = vec![0x10 | 1];
        commit.extend(miniz_oxide::deflate::compress_to_vec_zlib(b"a", 1));
        commit
    });
    entries.place_over(0, &appending(1, b'b'));
    let base_end = entries.starts[1];
    let folders = packed(&names, &[entries.entries]);
    let mut odb = Odb::open(&folders.0[0]);
    let mut read = || {
        odb.read(&Oid(names[1]))
            .map(|object| object.map(|object| object.data))
            .map_err(|error| error.to_string())
    };
    let first = read();
    // The commit's stream spoilt in its pack: past its header, where the
    // pack's own twelve bytes end.
    let pack = folders.0[0].join("pack/pack-00.pack");
    let mut bytes = std::fs::read(&pack).expect("the pack reads");
    let spoilt = usize::try_from(base_end).expect("a small pack") + 12;
    bytes[13..spoilt].fill(0xff);
    std::fs::write(&pack, bytes).expect("the temp folder takes a file");
    assert_eq!(
        (first, read()),
        (Ok(Some(b"ab".to_vec())), Ok(Some(b"ab".to_vec()))),
        "a base kept was read again from its pack"
    );
}
