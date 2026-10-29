//! What files take on the disk, read from the extent items a Btrfs tree
//! search gives. The searches themselves need root on Btrfs, so the VM test
//! proves them; these build the result buffers the kernel would.

use disk_guard::footprint::{Extent, Tally, extents};

/// A search result for one item: its header, then the item's bytes.
fn item(offset: u64, kind: u32, body: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&7u64.to_ne_bytes());
    bytes.extend_from_slice(&257u64.to_ne_bytes());
    bytes.extend_from_slice(&offset.to_ne_bytes());
    bytes.extend_from_slice(&kind.to_ne_bytes());
    bytes.extend_from_slice(&u32::try_from(body.len()).unwrap().to_ne_bytes());
    bytes.extend_from_slice(body);
    bytes
}

/// A file extent item's head: generation, size once read, compression,
/// encryption, other encoding and its kind.
fn head(read: u64, compression: u8, kind: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u64.to_ne_bytes());
    bytes.extend_from_slice(&read.to_ne_bytes());
    bytes.extend_from_slice(&[compression, 0, 0, 0, kind]);
    bytes
}

/// A run of the disk: a regular extent item `bytes` long on the disk from
/// `at`, holding `read` bytes once read.
fn run(at: u64, bytes: u64, read: u64) -> Vec<u8> {
    let mut body = head(read, 3, 1);
    for field in [at, bytes, 0, read] {
        body.extend_from_slice(&field.to_ne_bytes());
    }
    body
}

#[test]
fn a_compressed_run_counts_what_it_takes_on_the_disk_not_what_it_holds() {
    let buffer = item(0, 108, &run(1 << 20, 4096, 128 << 10));
    assert_eq!(
        extents(&buffer, 1),
        [(
            0,
            Extent::Run {
                at: 1 << 20,
                bytes: 4096
            }
        )]
    );
}

#[test]
fn inline_data_counts_its_own_bytes_and_a_hole_counts_nothing() {
    let mut inline = head(300, 0, 0);
    inline.extend_from_slice(&[b'x'; 120]);
    let mut hole = head(8192, 0, 1);
    for field in [0u64, 0, 0, 8192] {
        hole.extend_from_slice(&field.to_ne_bytes());
    }
    let mut buffer = item(0, 108, &inline);
    buffer.extend(item(4096, 108, &hole));
    assert_eq!(
        extents(&buffer, 2),
        [(0, Extent::Inline(120)), (4096, Extent::Hole)]
    );
}

#[test]
fn a_run_named_twice_counts_once() {
    let mut buffer = item(0, 108, &run(1 << 20, 4096, 128 << 10));
    buffer.extend(item(128 << 10, 108, &run(2 << 20, 8192, 128 << 10)));
    // The same run again, as a file copied by reference names it.
    buffer.extend(item(256 << 10, 108, &run(1 << 20, 4096, 128 << 10)));
    let mut tally = Tally::default();
    for (_, extent) in extents(&buffer, 3) {
        tally.add(extent);
    }
    assert_eq!(tally.disk, 4096 + 8192);
}

#[test]
fn items_of_other_kinds_are_passed_over() {
    let mut buffer = item(0, 1, &[0; 160]);
    buffer.extend(item(0, 108, &run(1 << 20, 4096, 4096)));
    assert_eq!(
        extents(&buffer, 2),
        [(
            0,
            Extent::Run {
                at: 1 << 20,
                bytes: 4096
            }
        )]
    );
}

#[test]
fn a_buffer_cut_short_ends_the_list_where_it_is_cut() {
    let mut buffer = item(0, 108, &run(1 << 20, 4096, 4096));
    let whole = buffer.len();
    buffer.extend(item(4096, 108, &run(2 << 20, 4096, 4096)));
    buffer.truncate(whole + 40);
    assert_eq!(
        extents(&buffer, 2),
        [(
            0,
            Extent::Run {
                at: 1 << 20,
                bytes: 4096
            }
        )]
    );
}

#[test]
fn an_account_gets_no_footprint_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file"), b"bytes").unwrap();
    // Off Btrfs there are no extent items to read, and on it the search
    // is refused to anyone but root: either way, nothing is answered.
    assert_eq!(disk_guard::footprint::footprint(dir.path()), None);
}
