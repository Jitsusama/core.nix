//! The pack index and multi-pack-index readers' tests, beside them as
//! the store's are, over indexes built byte by byte as git writes them.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// Names spread over first bytes, one of them sharing a first byte with
// another, as the fan-out must count.
const NAMES: [[u8; 20]; 4] = [[0x01; 20], [0x7f; 20], [0x80; 20], [0xfe; 20]];
const SAME_FIRST: [u8; 20] = {
    let mut name = [0x7f; 20];
    name[19] = 0x80;
    name
};

pub(crate) fn sorted() -> Vec<[u8; 20]> {
    let mut names = NAMES.to_vec();
    names.push(SAME_FIRST);
    names.sort_unstable();
    names
}

fn be32(value: u32) -> [u8; 4] {
    value.to_be_bytes()
}

fn count(names: &[[u8; 20]]) -> u32 {
    u32::try_from(names.len()).expect("a handful of names")
}

fn fanout(names: &[[u8; 20]]) -> Vec<u8> {
    (0..=255u8)
        .flat_map(|byte| {
            be32(count(
                &names
                    .iter()
                    .copied()
                    .filter(|name| name[0] <= byte)
                    .collect::<Vec<_>>(),
            ))
        })
        .collect()
}

/// A temp path no other test, nor other run, writes to.
pub(crate) fn unique(name: &str) -> std::path::PathBuf {
    static TAKEN: AtomicUsize = AtomicUsize::new(0);
    let n = TAKEN.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("git-engine-{}-{n}-{name}", std::process::id()))
}

/// Writes `bytes` where a test alone reads them.
fn file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = unique(name);
    std::fs::write(&path, bytes).expect("the temp folder takes a file");
    path
}

/// What `open` makes of `bytes` in a file of their own, which goes once
/// it's opened: a reader keeps its file open, so it reads on.
fn opened<T>(name: &str, bytes: &[u8], open: impl FnOnce(&Path) -> T) -> T {
    let path = file(name, bytes);
    let opened = open(&path);
    std::fs::remove_file(&path).expect("the temp file goes");
    opened
}

/// Git ends each index with the checksums of its pack and of itself,
/// which nothing here reads but a window of names may run into.
fn trailed(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.extend([0; 40]);
    bytes
}

/// Where each name lies in its pack, the last two past what four bytes
/// hold, so a table of large offsets has more than one entry to step by.
pub(crate) fn offsets(names: &[[u8; 20]]) -> Vec<u64> {
    (0..names.len())
        .map(|at| match names.len() - at {
            1 => 0x1_2345_6789,
            2 => 0x1_0000_0004,
            _ => 12 + 100 * at as u64,
        })
        .collect()
}

fn version_1(names: &[[u8; 20]]) -> Vec<u8> {
    let mut bytes = fanout(names);
    for (name, offset) in names.iter().zip(offsets(names)) {
        bytes.extend(be32(u32::try_from(offset).unwrap_or(0)));
        bytes.extend(name);
    }
    trailed(bytes)
}

pub(crate) fn version_2(names: &[[u8; 20]]) -> Vec<u8> {
    let mut bytes = b"\xfftOc".to_vec();
    bytes.extend(be32(2));
    bytes.extend(fanout(names));
    bytes.extend(names.concat());
    bytes.extend(vec![0; 4 * names.len()]);
    let mut large: Vec<u64> = Vec::new();
    for offset in offsets(names) {
        if let Ok(small) = u32::try_from(offset)
            && small & 0x8000_0000 == 0
        {
            bytes.extend(be32(small));
        } else {
            let at = u32::try_from(large.len()).expect("few large offsets");
            bytes.extend(be32(0x8000_0000 | at));
            large.push(offset);
        }
    }
    bytes.extend(large.iter().flat_map(|offset| offset.to_be_bytes()));
    trailed(bytes)
}

/// A multi-pack-index over `packs`, its names spread across them in
/// turn, claiming `pack_count` packs, with its pack names' chunk at
/// `names_at` when given and where they lie otherwise.
pub(crate) fn midx(packs: &[&str], pack_count: u32, names_at: Option<u64>) -> Vec<u8> {
    let names = sorted();
    let mut pnam = packs.join("\0").into_bytes();
    pnam.push(0);
    while !pnam.len().is_multiple_of(4) {
        pnam.push(0);
    }
    let mut ooff = Vec::new();
    let mut loff = Vec::new();
    for (at, offset) in offsets(&names).into_iter().enumerate() {
        ooff.extend(be32(
            u32::try_from(at % packs.len().max(1)).expect("few packs"),
        ));
        if offset > 0x7fff_ffff {
            ooff.extend(be32(
                0x8000_0000 | u32::try_from(loff.len() / 8).expect("few"),
            ));
            loff.extend(offset.to_be_bytes());
        } else {
            ooff.extend(be32(u32::try_from(offset).expect("small")));
        }
    }
    let chunks: [(&[u8; 4], Vec<u8>); 5] = [
        (b"PNAM", pnam),
        (b"OIDF", fanout(&names)),
        (b"OIDL", names.concat()),
        (b"OOFF", ooff),
        (b"LOFF", loff),
    ];
    let mut at = 12 + 12 * (chunks.len() as u64 + 1);
    let mut bytes = b"MIDX".to_vec();
    bytes.extend([1, 1, u8::try_from(chunks.len()).expect("five"), 0]);
    bytes.extend(be32(pack_count));
    for (id, chunk) in &chunks {
        bytes.extend(*id);
        let start = if *id == b"PNAM" {
            names_at.unwrap_or(at)
        } else {
            at
        };
        bytes.extend(start.to_be_bytes());
        at += chunk.len() as u64;
    }
    bytes.extend([0; 4]);
    bytes.extend(at.to_be_bytes());
    for (_, chunk) in chunks {
        bytes.extend(chunk);
    }
    // Its own checksum, as git's ends with.
    bytes.extend([0; 20]);
    bytes
}

fn found(index: &Index) -> Vec<Option<u64>> {
    let mut absent = [0x7f; 20];
    absent[19] = 0x01;
    sorted()
        .into_iter()
        .chain([absent])
        .map(|name| index.offset(&Oid(name)).expect("the index reads"))
        .collect()
}

#[test]
fn a_version_1_index_finds_each_name_where_its_pack_holds_it() {
    let names = sorted();
    let index = opened("v1.idx", &version_1(&names), Index::open).expect("it opens");
    // Version 1 holds four bytes of offset, so a larger one reads as
    // the fixture writes it in their place.
    let mut want: Vec<Option<u64>> = offsets(&names)
        .into_iter()
        .map(|offset| Some(u32::try_from(offset).map_or(0, u64::from)))
        .collect();
    want.push(None);
    assert_eq!(found(&index), want);
}

/// `count` names sharing one first byte, their leading values spread
/// evenly, as random names are.
fn crowded(count: u32) -> Vec<[u8; 20]> {
    let step = (1u64 << 56) / u64::from(count);
    (0..count)
        .map(|at| {
            let mut name = [0; 20];
            let lead = (0x42u64 << 56) | (u64::from(at) * step + 7);
            name[..8].copy_from_slice(&lead.to_be_bytes());
            name[16..].copy_from_slice(&at.to_be_bytes());
            name
        })
        .collect()
}

#[test]
fn names_too_many_to_scan_at_once_are_found_by_guessing_where_they_lie() {
    // More than two windows under one first byte, so the search guesses
    // rather than reading the whole span.
    let names = crowded(WINDOW * 5);
    let index = opened("crowded.idx", &version_2(&names), Index::open).expect("it opens");
    for (name, offset) in names.iter().zip(offsets(&names)) {
        assert_eq!(
            index.offset(&Oid(*name)).expect("the index reads"),
            Some(offset),
            "{name:02x?} wasn't found where its pack holds it"
        );
        // Its leading bytes with another tail is no name of the pack.
        let mut absent = *name;
        absent[19] ^= 0xff;
        assert_eq!(
            index.offset(&Oid(absent)).expect("the index reads"),
            None,
            "{absent:02x?} was found, though the pack holds no such name"
        );
    }
}

#[test]
fn a_version_2_index_finds_an_offset_past_four_bytes_in_its_large_table() {
    let names = sorted();
    let index = opened("v2.idx", &version_2(&names), Index::open).expect("it opens");
    let mut want: Vec<Option<u64>> = offsets(&names).into_iter().map(Some).collect();
    want.push(None);
    assert_eq!(found(&index), want);
}

#[test]
fn a_multi_pack_index_says_which_pack_holds_each_name_and_where() {
    let packs = ["pack-a.idx", "pack-b.idx"];
    let midx = opened("multi-pack-index", &midx(&packs, 2, None), Midx::open).expect("it opens");
    assert_eq!(midx.packs, packs);
    let names = sorted();
    let located: Vec<_> = names
        .iter()
        .map(|name| midx.locate(&Oid(*name)).expect("the index reads"))
        .collect();
    let want: Vec<_> = offsets(&names)
        .into_iter()
        .enumerate()
        .map(|(at, offset)| Some((at % 2, offset)))
        .collect();
    assert_eq!(located, want);
    assert_eq!(
        midx.locate(&Oid([0x02; 20])).expect("the index reads"),
        None
    );
}

/// Why `bytes` aren't taken as a multi-pack-index, as its error says.
fn refusal(name: &str, bytes: &[u8]) -> String {
    match opened(name, bytes, Midx::open) {
        Ok(midx) => panic!("{name} was taken, naming {:?}", midx.packs),
        Err(error) => error.to_string(),
    }
}

#[test]
fn a_multi_pack_index_naming_a_file_outside_the_pack_folder_is_refused() {
    for name in [
        "../objects.idx",
        "sub/pack-a.idx",
        ".hidden.idx",
        "pack-a.pack",
        "pack-a\\b.idx",
    ] {
        let bytes = midx(&["pack-a.idx", name], 2, None);
        assert_eq!(
            refusal("named-outside", &bytes),
            "multi-pack-index pack name",
            "a pack named {name:?} wasn't refused for its name"
        );
    }
    let mut unreadable = midx(&["pack-a.idx", "pack-b.idx"], 2, None);
    let at = unreadable
        .windows(6)
        .position(|window| window == b"pack-b")
        .expect("the fixture names pack-b");
    unreadable[at + 4] = 0xff;
    assert_eq!(
        refusal("named-unreadably", &unreadable),
        "multi-pack-index pack name",
        "a pack name that isn't UTF-8 wasn't refused for its name"
    );
}

#[test]
fn pack_names_past_one_read_are_read_whole_across_its_edge() {
    // Fifty bytes a name, so 100 of them take 5000 bytes, past one read,
    // and one name lies across its edge.
    let owned: Vec<String> = (0..100).map(|at| format!("pack-{at:040}.idx")).collect();
    let packs: Vec<&str> = owned.iter().map(String::as_str).collect();
    assert!(
        owned.iter().map(|name| name.len() as u64 + 1).sum::<u64>() > NAMES_READ,
        "the names fit in one read, so its edge goes untried"
    );
    let midx = opened("many-packs", &midx(&packs, 100, None), Midx::open).expect("it opens");
    assert_eq!(
        midx.packs, owned,
        "a name across a read's edge was read wrong"
    );
}

#[test]
fn a_multi_pack_index_naming_fewer_packs_than_it_holds_is_refused() {
    let bytes = midx(&["pack-a.idx"], 2, None);
    assert_eq!(
        refusal("named-fewer", &bytes),
        "multi-pack-index names fewer packs than it holds",
        "one name wasn't refused for two packs"
    );
}

/// `bytes` with the chunk at `entry` of its table said to lie at `at`.
pub(crate) fn moved(mut bytes: Vec<u8>, entry: usize, at: u64) -> Vec<u8> {
    let field = 12 + 12 * entry + 4;
    bytes[field..field + 8].copy_from_slice(&at.to_be_bytes());
    bytes
}

#[test]
fn a_multi_pack_index_with_a_chunk_past_its_end_is_refused_at_open() {
    let whole = midx(&["pack-a.idx"], 1, None);
    let body = whole.len() as u64 - 20;
    // Each said to end one byte into the checksum, with the length it
    // needs for its five names; the names and large offsets, whose
    // length the table alone gives, are said to start in it.
    for (entry, len, why) in [
        (1, 1024, "multi-pack-index fan-out past its end"),
        (2, 20 * 5, "multi-pack-index object names past its end"),
        (3, 8 * 5, "multi-pack-index offsets past its end"),
        (4, 0, "multi-pack-index large offsets past its end"),
        (0, 0, "multi-pack-index names past its end"),
    ] {
        let at = body + 1 - len;
        assert_eq!(
            refusal("chunk-far", &moved(whole.clone(), entry, at)),
            why,
            "chunk {entry} at {at} wasn't refused for lying past the end"
        );
    }
    assert_eq!(
        refusal("chunk-wraps", &moved(whole.clone(), 2, u64::MAX - 1)),
        "multi-pack-index object names past its end",
        "a chunk whose end wraps past 64 bits wasn't refused"
    );
}

#[test]
fn a_multi_pack_index_naming_a_pack_past_what_a_file_name_holds_is_refused() {
    for (len, taken) in [(255, true), (256, false), (300, false)] {
        let long = format!("{}.idx", "x".repeat(len - 4));
        let bytes = midx(&["pack-a.idx", &long], 2, None);
        match opened("names-long", &bytes, Midx::open) {
            Ok(_) => assert!(taken, "a pack name of {len} bytes was taken"),
            Err(error) => assert!(
                !taken && error.to_string() == "multi-pack-index pack name",
                "a pack name of {len} bytes was refused: {error}"
            ),
        }
    }
}

// A names chunk whose apparent size is half a gigabyte of a sparse
// file's holes, with a header claiming two million packs, so a reader
// trusting either sets that much aside and reads it all: 17 s in a
// debug build. Its first window is enough to refuse it.
#[test]
fn a_multi_pack_index_claiming_more_packs_than_it_names_is_refused_at_its_first_gap() {
    let whole = midx(&["pack-a.idx"], 1 << 21, None);
    // The names said to start where the holes do, running to the end.
    let len = whole.len() as u64;
    let path = file("names-sparse", &moved(whole, 0, len));
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_len(len + (512 << 20)))
        .expect("the temp file grows");
    let started = std::time::Instant::now();
    let refused = Midx::open(&path).err().map(|error| error.to_string());
    let took = started.elapsed();
    std::fs::remove_file(&path).expect("the temp file goes");
    assert_eq!(
        refused.as_deref(),
        Some("multi-pack-index names fewer packs than it holds"),
        "a names chunk of holes wasn't refused for naming fewer packs"
    );
    assert!(
        took < std::time::Duration::from_secs(1),
        "refusing a names chunk of holes took {took:?}"
    );
}

#[test]
fn a_pack_name_its_chunk_never_ends_is_refused_rather_than_read_into_the_next() {
    let mut bytes = midx(&["pack-a.idx"], 1, None);
    // Twelve bytes, the whole chunk, with no NUL: the fan-out after it
    // opens with one, which would end the name there.
    let at = chunk_at(&bytes, 0);
    bytes[at..at + 12].copy_from_slice(b"pack-abc.idx");
    assert_eq!(
        refusal("name-unended", &bytes),
        "multi-pack-index names fewer packs than it holds",
        "a name running past its chunk was read on into the next"
    );
}

/// Where the chunk at `entry` of the table in `bytes` lies.
fn chunk_at(bytes: &[u8], entry: usize) -> usize {
    let field = 12 + 12 * entry + 4;
    let at = u64::from_be_bytes(bytes[field..field + 8].try_into().expect("8 bytes"));
    usize::try_from(at).expect("a fixture's offset")
}

#[test]
fn a_multi_pack_index_whose_fan_out_falls_is_refused_at_open() {
    let mut bytes = midx(&["pack-a.idx"], 1, None);
    // Names under 0x01 said to be more than all of them.
    let at = chunk_at(&bytes, 1) + 4;
    bytes[at..at + 4].copy_from_slice(&be32(0x0100_0000));
    assert_eq!(
        refusal("fan-out-falls", &bytes),
        "multi-pack-index fan-out out of order",
        "a fan-out whose counts fall was taken"
    );
}

#[test]
fn a_large_offset_past_its_chunk_is_refused_rather_than_read_from_the_next() {
    let whole = midx(&["pack-a.idx"], 1, None);
    let body = whole.len() as u64 - 20;
    let names = sorted();
    let last = Oid(names[names.len() - 1]);
    let second_last = Oid(names[names.len() - 2]);
    // The fixture's two large offsets, for its last two names, with
    // their chunk said to hold none, half of one, then one, the last
    // with the object names' chunk after it rather than the checksum.
    let names_at = chunk_at(&whole, 2) as u64;
    for (at, held) in [(body, 0), (body - 4, 0), (body - 8, 1), (names_at - 8, 1)] {
        let midx =
            opened("loff-short", &moved(whole.clone(), 4, at), Midx::open).expect("it opens");
        let refused = midx.locate(&last).err().map(|error| error.to_string());
        assert_eq!(
            refused.as_deref(),
            Some("multi-pack-index large offset past its chunk"),
            "the second large offset was read from a chunk holding {held}"
        );
        assert_eq!(
            midx.locate(&second_last).is_ok(),
            held == 1,
            "the first large offset was read, or refused, from a chunk holding {held}"
        );
    }
}
