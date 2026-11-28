//! The object store's tests of a pack or its index rewritten in place, as
//! `cp` copies a backup over one: that moves the file's own stamp but not
//! its folder's, so only its store is opened again, and the copies it
//! marked while its packs held are read again, whatever moved with them.

use super::delta_tests::{blob, names, packed};
use super::*;

/// What `odb` reads for `name`: its data, or what refused it.
fn read(odb: &mut Odb, name: [u8; 20]) -> Result<Option<Vec<u8>>, String> {
    odb.read(&Oid(name))
        .map(|object| object.map(|object| object.data))
        .map_err(|error| error.to_string())
}

#[test]
fn a_copy_marked_while_the_packs_hold_is_read_again_once_its_pack_is_rewritten_in_place() {
    let names = names(1);
    // names[0] an entry of a kind git has none of, its own bytes' fault;
    // and a backup of its pack holding a sound blob at the same place.
    let folders = packed(&names, &[vec![vec![0x50 | 1, 0]]]);
    let backup = packed(&names, &[vec![blob(b"a")]]);
    let objects = &folders.0[0];
    // Settled stamps, so only a change to what the store reads moves them.
    crate::stamp::tests::ahead(3);
    let read = |odb: &mut Odb| {
        odb.read(&Oid(names[0]))
            .map(|object| object.map(|object| object.data))
            .map_err(|error| error.to_string())
    };
    let mut odb = Odb::open(objects);
    let marked = read(&mut odb);
    // Copied over the pack in place, as `cp` copies, so the pack's folder
    // stays as it was.
    let pack = "pack/pack-00.pack";
    std::fs::copy(backup.0[0].join(pack), objects.join(pack))
        .expect("the temp folder takes a copy");
    let mut odb = Odb::reopen(Some(odb), objects);
    odb.forget_passing_marks();
    assert_eq!(
        (marked, read(&mut odb)),
        (Err("pack entry kind".into()), Ok(Some(b"a".to_vec()))),
        "a copy marked while the packs hold stayed marked once its pack was rewritten in place"
    );
}

#[test]
fn a_copy_marked_while_the_packs_hold_is_read_again_once_its_index_alone_is_rewritten() {
    use crate::pack_index::tests::version_2_at;
    let names = names(2);
    // names[0] an entry of a kind git has none of, at 12, its own bytes'
    // fault, and names[1] a sound blob just past it, at 14.
    let folders = packed(&names, &[vec![vec![0x50 | 1, 0], blob(b"a")]]);
    let objects = &folders.0[0];
    crate::stamp::tests::ahead(3);
    let mut odb = Odb::open(objects);
    let marked = read(&mut odb, names[0]);
    // A backup of the index written over it in place, saying names[0] is
    // the blob: it names one object fewer, so only an index opened again,
    // not one kept with the count it had, finds where it lies.
    std::fs::write(
        objects.join("pack/pack-00.idx"),
        version_2_at(&names[..1], &[14]),
    )
    .expect("the temp index is written again");
    let mut odb = Odb::reopen(Some(odb), objects);
    odb.forget_passing_marks();
    assert_eq!(
        (marked, read(&mut odb, names[0])),
        (Err("pack entry kind".into()), Ok(Some(b"a".to_vec()))),
        "a copy marked while the packs hold wasn't read again once its index alone was \
         rewritten in place, or the index was read by the count it had before"
    );
}
