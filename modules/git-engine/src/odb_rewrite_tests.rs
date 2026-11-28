//! The object store's tests of a pack or its index rewritten in place, as
//! `cp` copies a backup over one: that moves the file's own stamp but not
//! its folder's, so only its store is opened again, and the copies it
//! marked while its packs held are read again, whatever moved with them.

use super::delta_tests::{blob, names, packed};
use super::tests::{Removed, objects_over};
use super::*;

/// What `odb` reads for `name`: its data, or what refused it.
fn read(odb: &mut Odb, name: [u8; 20]) -> Result<Option<Vec<u8>>, String> {
    odb.read(&Oid(name))
        .map(|object| object.map(|object| object.data))
        .map_err(|error| error.to_string())
}

/// A pack holding `entry` alone, at offset 12, as git writes one.
fn pack_of(entry: &[u8]) -> Vec<u8> {
    [b"PACK\0\0\0\x02\0\0\0\x01", entry, &[0; 20]].concat()
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
fn a_copy_marked_while_the_packs_hold_is_read_again_once_its_pack_is_patched_in_place() {
    use std::os::unix::fs::{FileExt, MetadataExt};
    let names = names(1);
    // names[0] a blob whose first byte says a kind git has none of, so the
    // copy is the size it is sound, and its own bytes' fault.
    let sound = blob(b"a");
    let mut wrong = sound.clone();
    wrong[0] = 0x50 | 1;
    let folders = packed(&names, &[vec![wrong]]);
    let objects = &folders.0[0];
    crate::stamp::tests::ahead(3);
    let mut odb = Odb::open(objects);
    let marked = read(&mut odb, names[0]);
    // The kind patched in place, and the time it was modified put back, as
    // a copy keeping its times leaves it, so only its bytes and its ctime
    // move. The time is put back again till the ctime passes the clock's
    // tick it had, which may be the one it was written in.
    let path = objects.join("pack/pack-00.pack");
    let pack = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the temp pack opens to be written");
    let before = pack.metadata().expect("the temp pack is asked its stamp");
    pack.write_at(&sound[..1], 12)
        .expect("the temp pack takes a byte");
    let ctime = |meta: &std::fs::Metadata| (meta.ctime(), meta.ctime_nsec());
    let mut after = pack.metadata().expect("the temp pack is asked its stamp");
    for _ in 0..1000 {
        pack.set_modified(
            before
                .modified()
                .expect("the temp pack has a time modified"),
        )
        .expect("the temp pack's time modified is put back");
        after = pack.metadata().expect("the temp pack is asked its stamp");
        if ctime(&after) != ctime(&before) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let held = (
        after.len() == before.len() && after.mtime() == before.mtime(),
        after.mtime_nsec() == before.mtime_nsec() && ctime(&after) != ctime(&before),
    );
    let mut odb = Odb::reopen(Some(odb), objects);
    odb.forget_passing_marks();
    assert_eq!(
        (marked, held, read(&mut odb, names[0])),
        (
            Err("pack entry kind".into()),
            (true, true),
            Ok(Some(b"a".to_vec()))
        ),
        "a copy marked while the packs hold stayed marked once its pack was patched in \
         place at its size and time modified, or the patch moved more than the pack's \
         ctime and bytes"
    );
}

#[test]
fn a_copy_marked_in_a_multi_pack_index_s_pack_is_read_again_once_it_is_rewritten_in_place() {
    use crate::pack_index::tests::{midx, sorted, version_2_at};
    let names = sorted();
    // The multi-pack-index says names[0] lies at 12 in its one pack, an
    // entry of a kind git has none of there, its own bytes' fault.
    let objects = Removed(vec![objects_over(
        midx(&["pack-00.idx"], 1, None),
        vec![("pack-00", version_2_at(&names[..1], &[12]))],
    )]);
    let pack = objects.0[0].join("pack/pack-00.pack");
    std::fs::write(&pack, pack_of(&[0x50 | 1, 0])).expect("the temp folder takes a file");
    crate::stamp::tests::ahead(3);
    let mut odb = Odb::open(&objects.0[0]);
    let covered = odb.stores[0].midx.is_some();
    let marked = read(&mut odb, names[0]);
    // A backup holding a sound blob there written over it in place.
    std::fs::write(&pack, pack_of(&blob(b"a"))).expect("the temp pack is written again");
    let mut odb = Odb::reopen(Some(odb), &objects.0[0]);
    odb.forget_passing_marks();
    assert_eq!(
        (covered, marked, read(&mut odb, names[0])),
        (true, Err("pack entry kind".into()), Ok(Some(b"a".to_vec()))),
        "a store's multi-pack-index wasn't used, or a copy it found marked while the packs \
         hold stayed marked once its pack was rewritten in place"
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
