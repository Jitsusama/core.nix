//! The object store's tests, beside it as the pack index readers' are,
//! over folders of objects built byte by byte as git writes them.

use super::*;

#[test]
fn deltas_copy_and_insert() {
    let base = b"hello, world";
    // base 12, result 12: copy 7 from 0 ("hello, "), insert "there".
    let delta = [12, 12, 0x90, 7, 5, b't', b'h', b'e', b'r', b'e'];
    assert_eq!(apply_delta(base, &delta).unwrap(), b"hello, there");
    assert!(
        apply_delta(b"short", &delta).is_err(),
        "a delta over a base of 12 was applied to one of 5"
    );
}

#[test]
fn a_multi_pack_index_refused_leaves_each_pack_found_through_its_own_index() {
    use crate::pack_index::tests::{midx, moved, offsets, sorted, unique, version_2};
    let objects = unique("objects");
    let pack = objects.join("pack");
    std::fs::create_dir_all(&pack).expect("the temp folder takes a folder");
    let names = sorted();
    // Its object names are said to run into its checksum.
    let whole = midx(&["pack-a.idx"], 1, None);
    let broken = moved(whole.clone(), 2, whole.len() as u64 - 21);
    for (name, bytes) in [
        ("multi-pack-index", broken),
        ("pack-a.idx", version_2(&names)),
        ("pack-a.pack", b"PACK".to_vec()),
    ] {
        std::fs::write(pack.join(name), bytes).expect("the temp folder takes a file");
    }
    let store = Store::open(&objects);
    assert!(
        store.midx.is_none(),
        "a multi-pack-index past its end was used"
    );
    assert_eq!(
        store.locate(&Oid(names[0]), &mut None),
        Some((0, offsets(&names)[0])),
        "the pack's own index wasn't read in the refused index's place"
    );
}

/// An objects folder holding `midx` and, for each of `packs`, its own
/// index and an empty pack.
fn objects_over(midx: Vec<u8>, packs: Vec<(&str, Vec<u8>)>) -> PathBuf {
    use crate::pack_index::tests::unique;
    let objects = unique("objects");
    let pack = objects.join("pack");
    std::fs::create_dir_all(&pack).expect("the temp folder takes a folder");
    std::fs::write(pack.join("multi-pack-index"), midx).expect("the temp folder takes a file");
    for (name, index) in packs {
        std::fs::write(pack.join(format!("{name}.idx")), index)
            .expect("the temp folder takes a file");
        std::fs::write(pack.join(format!("{name}.pack")), b"PACK")
            .expect("the temp folder takes a file");
    }
    objects
}

/// Where `oid` lies in `store`, and what it kept as refused.
fn located(store: &Store, oid: [u8; 20]) -> (Option<(usize, u64)>, Option<String>) {
    let mut refused = None;
    let found = store.locate(&Oid(oid), &mut refused);
    (found, refused.map(|error| error.to_string()))
}

#[test]
fn a_lookup_the_multi_pack_index_cant_answer_is_read_from_its_packs_own_indexes() {
    use crate::pack_index::tests::{midx, moved, offsets, sorted, version_2};
    let names = sorted();
    let last = names.len() - 1;
    let whole = midx(&["pack-a.idx"], 1, None);
    // Its large offsets moved to the last 8 bytes before its checksum
    // hold one, so the last name's, its second, lies past its chunk.
    let short = moved(whole.clone(), 4, whole.len() as u64 - 28);
    let objects = objects_over(short.clone(), vec![("pack-a", version_2(&names))]);
    let store = Store::open(&objects);
    assert!(store.midx.is_some(), "a short LOFF was refused at open");
    assert_eq!(
        located(&store, names[0]),
        (Some((0, offsets(&names)[0])), None),
        "a sound entry wasn't read from the multi-pack-index"
    );
    assert_eq!(
        located(&store, [0x02; 20]),
        (None, None),
        "a name the multi-pack-index lacks was found or refused"
    );
    assert!(
        store.covered.get().is_none(),
        "a sound entry, or a name it lacks, opened the covered packs' own indexes"
    );
    assert_eq!(
        located(&store, names[last]),
        (
            Some((0, offsets(&names)[last])),
            Some("multi-pack-index large offset past its chunk".into())
        ),
        "a large offset past its chunk wasn't read from the pack's own index, its refusal kept"
    );
    // With the pack's own index not holding it either, the refusal is
    // kept for the caller rather than passing for a missing object.
    let lacking = objects_over(short.clone(), vec![("pack-a", version_2(&names[..last]))]);
    assert_eq!(
        located(&Store::open(&lacking), names[last]),
        (
            None,
            Some("multi-pack-index large offset past its chunk".into())
        ),
        "a broken entry no index answers wasn't kept as refused"
    );
    // A covered index that won't read is passed over, the covered pack
    // after it still read, and the multi-pack-index's own error kept.
    let both = midx(&["pack-a.idx", "pack-b.idx"], 2, None);
    let two = moved(both.clone(), 4, both.len() as u64 - 28);
    let mut broken = version_2(&names);
    broken.truncate(8 + 1024);
    let objects = objects_over(two, vec![("pack-a", broken), ("pack-b", version_2(&names))]);
    let (found, refused) = located(&Store::open(&objects), names[last]);
    assert_eq!(
        found,
        Some((1, offsets(&names)[last])),
        "a covered index that won't read hid the one after it"
    );
    assert_eq!(
        refused.as_deref(),
        Some("multi-pack-index large offset past its chunk"),
        "the multi-pack-index's own error wasn't the one kept"
    );
    // Its names alternate between two packs, but it counts one, so
    // every other name is said to lie in a pack it doesn't name: read
    // from the packs' own indexes, and kept as refused where they lack
    // it, for the caller to let stand only should no other place hold it.
    let past = midx(&["pack-a.idx", "pack-b.idx"], 1, None);
    let objects = objects_over(past.clone(), vec![("pack-a", version_2(&names))]);
    let store = Store::open(&objects);
    assert_eq!(store.midx_packs, 1, "the index's count wasn't kept");
    assert_eq!(
        located(&store, names[1]),
        (
            Some((0, offsets(&names)[1])),
            Some("multi-pack-index names a pack past those it covers".into())
        ),
        "a name in a pack past the index's count wasn't read from its packs' indexes"
    );
    let lacking = objects_over(past, vec![("pack-a", version_2(&names[..1]))]);
    assert_eq!(
        located(&Store::open(&lacking), names[1]),
        (
            None,
            Some("multi-pack-index names a pack past those it covers".into())
        ),
        "a pack past the index's count left no refusal where nothing held the name"
    );
}

#[test]
fn a_covered_pack_whose_index_wont_open_keeps_its_place_in_the_multi_pack_index() {
    use crate::pack_index::tests::{midx, moved, offsets, sorted, version_2};
    let names = sorted();
    let last = names.len() - 1;
    let both = midx(&["pack-a.idx", "pack-b.idx"], 2, None);
    // Its large offsets cut short, so the last name is read from the
    // covered packs' own indexes, the first of which won't open.
    let short = moved(both.clone(), 4, both.len() as u64 - 28);
    let objects = objects_over(
        short,
        vec![("pack-a", b"nope".to_vec()), ("pack-b", version_2(&names))],
    );
    let (found, _) = located(&Store::open(&objects), names[last]);
    std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
    assert_eq!(
        found,
        Some((1, offsets(&names)[last])),
        "a covered index that won't open moved the packs after it out of their places"
    );
}

/// A pack holding one blob, `data`, at offset 12, just past its header,
/// where an index of names sorting first among three says it lies.
fn one_blob_pack(data: &[u8]) -> Vec<u8> {
    let mut bytes = b"PACK".to_vec();
    bytes.extend(2u32.to_be_bytes());
    bytes.extend(1u32.to_be_bytes());
    // A blob, type 3, of fewer than 16 bytes says its size in one byte.
    bytes.push(0x30 | u8::try_from(data.len()).expect("a short blob"));
    bytes.extend(miniz_oxide::deflate::compress_to_vec_zlib(data, 1));
    bytes.extend([0; 20]);
    bytes
}

#[test]
fn an_object_another_store_packs_is_read_past_a_refusal_in_the_first() {
    use crate::pack_index::tests::{midx, sorted, version_2};
    let names = sorted();
    // The first store's index counts one pack of two, so names[1], said
    // to lie in the second, is refused there.
    let past = midx(&["pack-a.idx", "pack-b.idx"], 1, None);
    let first = objects_over(past, vec![("pack-a", version_2(&names[..1]))]);
    // The store its alternates name packs names[1] first of three.
    let other = objects_over(Vec::new(), Vec::new());
    let pack = other.join("pack");
    std::fs::remove_file(pack.join("multi-pack-index")).expect("the empty index goes");
    std::fs::write(pack.join("pack-z.idx"), version_2(&names[1..4]))
        .expect("the temp folder takes a file");
    std::fs::write(pack.join("pack-z.pack"), one_blob_pack(b"hi"))
        .expect("the temp folder takes a file");
    std::fs::create_dir_all(first.join("info")).expect("the temp folder takes a folder");
    std::fs::write(
        first.join("info/alternates"),
        other.to_str().expect("a UTF-8 temp path"),
    )
    .expect("the temp folder takes a file");
    let read = Odb::open(&first)
        .read(&Oid(names[1]))
        .map(|object| object.map(|object| object.data));
    std::fs::remove_dir_all(&first).expect("the temp folder gives its folder back");
    std::fs::remove_dir_all(&other).expect("the temp folder gives its folder back");
    assert_eq!(
        read.map_err(|error| error.to_string()),
        Ok(Some(b"hi".to_vec())),
        "a refusal in the first store hid the object another store packs"
    );
}

#[test]
fn a_pack_index_that_wont_read_is_kept_as_refused_and_the_packs_after_it_read() {
    use crate::pack_index::tests::{offsets, sorted, unique, version_2};
    let names = sorted();
    let objects = unique("objects");
    let pack = objects.join("pack");
    std::fs::create_dir_all(&pack).expect("the temp folder takes a folder");
    // Its fan-out says it holds every name, but its names stop short.
    let mut broken = version_2(&names);
    broken.truncate(8 + 1024);
    for (name, bytes) in [
        ("pack-a.idx", broken),
        ("pack-a.pack", b"PACK".to_vec()),
        ("pack-b.idx", version_2(&names)),
        ("pack-b.pack", b"PACK".to_vec()),
    ] {
        std::fs::write(pack.join(name), bytes).expect("the temp folder takes a file");
    }
    let (found, refused) = located(&Store::open(&objects), names[0]);
    std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
    // Packs are read in name order, so pack-a's index is read first.
    assert_eq!(
        found,
        Some((1, offsets(&names)[0])),
        "a pack index that won't read hid the pack after it"
    );
    assert!(
        refused.is_some(),
        "a pack index that won't read was passed over unkept"
    );
}

#[test]
fn a_refusal_stands_only_once_the_loose_objects_lack_the_object_too() {
    use crate::pack_index::tests::{midx, moved, sorted, version_2};
    let names = sorted();
    let last = names.len() - 1;
    let whole = midx(&["pack-a.idx"], 1, None);
    let short = moved(whole.clone(), 4, whole.len() as u64 - 28);
    let objects = objects_over(short, vec![("pack-a", version_2(&names[..last]))]);
    let refused = Odb::open(&objects)
        .read(&Oid(names[last]))
        .err()
        .map(|error| error.to_string());
    assert_eq!(
        refused.as_deref(),
        Some("multi-pack-index large offset past its chunk"),
        "a broken entry nothing else answers passed for a missing object"
    );
    let hex = Oid(names[last]).hex();
    let folder = objects.join(&hex[..2]);
    std::fs::create_dir_all(&folder).expect("the temp folder takes a folder");
    std::fs::write(
        folder.join(&hex[2..]),
        miniz_oxide::deflate::compress_to_vec_zlib(b"blob 2\0hi", 1),
    )
    .expect("the temp folder takes a file");
    let read = Odb::open(&objects)
        .read(&Oid(names[last]))
        .expect("a loose object is read past a broken entry")
        .map(|object| object.data);
    std::fs::remove_dir_all(&objects).expect("the temp folder gives its folder back");
    assert_eq!(
        read.as_deref(),
        Some(&b"hi"[..]),
        "a broken entry hid the loose object"
    );
}

#[test]
fn commits_read_their_parts() {
    let tree = Oid::of("tree", b"");
    let text = format!(
        "tree {}\nparent {}\nauthor A <a@b> 1 +0000\ncommitter C <c@d> 1700000000 -0500\n\
         encoding ISO-8859-1\n\n\nSummary line\n\nBody\n",
        tree.hex(),
        tree.hex()
    );
    let commit = parse_commit(text.as_bytes()).unwrap();
    assert_eq!(commit.tree, tree);
    assert_eq!(commit.parents, vec![tree]);
    assert_eq!(commit.time, 1_700_000_000);
    assert_eq!(commit.encoding, "ISO-8859-1");
    assert_eq!(commit.summary, "Summary line");
}
