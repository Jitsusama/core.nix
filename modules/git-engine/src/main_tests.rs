//! The tests of the engine's answers: the flags it takes, the repository
//! it reads, and what it keeps between answers and reads again.

use super::*;
use crate::stamp::tests::settled;

/// A repository with no commits yet, made afresh under `name`.
fn repository(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("git-engine-main-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".git/objects/pack")).unwrap();
    std::fs::create_dir_all(dir.join(".git/refs/heads")).unwrap();
    std::fs::write(dir.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    dir
}

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn takes_every_flag_gitstatus_gives_it() {
    let given = words("-s 1 -u -1 -c 1 -d 1 -m -1 -e -U -W -D -t 32 -v FATAL");
    assert_eq!(stray(&given), None, "gitstatus's own flags were refused");
}

#[test]
fn refuses_a_bare_word() {
    assert_eq!(
        stray(&words("-s 1 -U here")),
        Some("here"),
        "a word no flag takes went unseen"
    );
}

#[test]
fn lets_go_of_a_repository_whose_packs_changed() {
    let dir = repository("repacked");
    let mut engine = Engine::default();
    engine.ask(&dir, &options(&[]), false);
    assert_eq!(engine.repos.len(), 1);
    // What a repack does: writes a pack and removes the old ones.
    std::fs::write(dir.join(".git/objects/pack/pack-new.idx"), b"").unwrap();
    engine.let_go_of_changed();
    assert_eq!(
        engine.repos.len(),
        0,
        "a repository whose packs changed was kept"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_repository_started_afresh_after_an_error_keeps_the_copies_its_store_marked() {
    let dir = repository("marked");
    let name = [7; 20];
    std::fs::write(
        dir.join(".git/refs/heads/main"),
        format!("{}\n", hash::Oid(name).hex()),
    )
    .unwrap();
    // HEAD's one copy, first of a kind git has none of, then rewritten
    // in place as a blob, which moves none of the stamps the store
    // watches; a store opened again reads it, one kept passes it over.
    let pack = |entry: &[u8]| [b"PACK\0\0\0\x02\0\0\0\x01", entry, &[0; 20]].concat();
    let packs = dir.join(".git/objects/pack");
    std::fs::write(
        packs.join("pack-00.idx"),
        pack_index::tests::version_2_at(&[name], &[12]),
    )
    .unwrap();
    std::fs::write(packs.join("pack-00.pack"), pack(&[0x51, 0])).unwrap();
    settled(&dir);
    let mut engine = Engine::default();
    let refused = engine.ask(&dir, &options(&[]), false).is_none();
    // Its refs, kept in a reftable that isn't there, refuse the next
    // opening, which leaves the store kept rather than dropping it.
    let config = dir.join(".git/config");
    std::fs::write(&config, "[extensions]\n\trefStorage = reftable\n").unwrap();
    let unopened = engine.ask(&dir, &options(&[]), false).is_none();
    std::fs::remove_file(&config).unwrap();
    let blob = [
        &[0x31][..],
        &miniz_oxide::deflate::compress_to_vec_zlib(b"a", 1),
    ]
    .concat();
    std::fs::write(packs.join("pack-00.pack"), pack(&blob)).unwrap();
    let again = engine.ask(&dir, &options(&[]), false).is_none();
    let kept = engine.kept.len();
    let afresh = Engine::default().ask(&dir, &options(&[]), false).is_some();
    std::fs::write(packs.join("pack-new.idx"), b"").unwrap();
    engine.let_go_of_changed();
    assert_eq!(
        (refused, unopened, again, kept, afresh, engine.kept.len()),
        (true, true, true, 1, true, 0),
        "a repository started afresh unpacked a copy its store had marked again, \
         dropped its store when its refs wouldn't open, or kept it past a change to \
         the packs"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_copy_marked_for_a_base_no_store_held_is_read_again_once_the_base_is_written() {
    let dir = repository("unbased");
    let (name, base) = ([7; 20], [8; 20]);
    std::fs::write(
        dir.join(".git/refs/heads/main"),
        format!("{}\n", hash::Oid(name).hex()),
    )
    .unwrap();
    // HEAD's one copy, a delta over a base nothing holds, which turns
    // its one byte into an x.
    let delta = miniz_oxide::deflate::compress_to_vec_zlib(&[1, 1, 1, b'x'], 1);
    let entry = [&[0x74][..], &base, &delta].concat();
    let packs = dir.join(".git/objects/pack");
    std::fs::write(
        packs.join("pack-00.idx"),
        pack_index::tests::version_2_at(&[name], &[12]),
    )
    .unwrap();
    std::fs::write(
        packs.join("pack-00.pack"),
        [b"PACK\0\0\0\x02\0\0\0\x01", &entry[..], &[0; 20]].concat(),
    )
    .unwrap();
    crate::stamp::tests::ahead(3);
    let mut engine = Engine::default();
    let refused = engine.ask(&dir, &options(&[]), false).is_none();
    // The store is kept, marks and all, so a store opened afresh can't
    // be what reads the copy next.
    let kept = engine.kept.len();
    // The base written loose, which moves none of the stamps the store
    // watches, so only a mark that passes lets the copy read.
    let hex = hash::Oid(base).hex();
    let loose = dir.join(".git/objects").join(&hex[..2]);
    std::fs::create_dir_all(&loose).unwrap();
    std::fs::write(
        loose.join(&hex[2..]),
        miniz_oxide::deflate::compress_to_vec_zlib(b"blob 1\0a", 1),
    )
    .unwrap();
    let answered = engine.ask(&dir, &options(&[]), false).is_some();
    assert_eq!(
        (refused, kept, answered, engine.kept.len()),
        (true, 1, true, 0),
        "a copy whose base no store held was answered before the base was written, \
         its store wasn't kept, or it stayed marked once the base was"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A repository whose HEAD is `commit` and whose one tag, v1, names the
/// annotated tag `tag`, with the folder its loose copy would lie in, and
/// that tag's object as git would write it loose, zlib and all.
fn tagged(name: &str, commit: [u8; 20], tag: [u8; 20]) -> (PathBuf, PathBuf, Vec<u8>) {
    let dir = repository(name);
    let hex = |oid: [u8; 20]| hash::Oid(oid).hex();
    std::fs::write(
        dir.join(".git/refs/heads/main"),
        format!("{}\n", hex(commit)),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join(".git/refs/tags")).unwrap();
    std::fs::write(dir.join(".git/refs/tags/v1"), format!("{}\n", hex(tag))).unwrap();
    let loose = dir.join(".git/objects").join(&hex(tag)[..2]);
    std::fs::create_dir_all(&loose).unwrap();
    let object = format!("object {}\ntype commit\ntag v1\n\nv1\n", hex(commit));
    let raw = [
        format!("tag {}\0", object.len()).as_bytes(),
        object.as_bytes(),
    ]
    .concat();
    let zlib = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 1);
    (dir, loose, zlib)
}

#[test]
fn a_tag_whose_object_is_wrong_settles_till_a_new_copy_is_renamed_in() {
    let (commit, tag) = ([9; 20], [10; 20]);
    let (dir, loose, zlib) = tagged("unpeeled", commit, tag);
    // The annotated tag's object, loose and not zlib at all. git won't
    // write over a copy it has, so a sound one comes only by a rename,
    // which moves its folder's stamp; a write in place moves none.
    let file = loose.join(&hash::Oid(tag).hex()[2..]);
    std::fs::write(&file, b"not zlib").unwrap();
    settled(&dir);
    let mut engine = Engine::default();
    let mut tag_said = || {
        engine
            .ask(&dir, &options(&[]), false)
            .map(|answer| answer.tag)
    };
    let wrong = tag_said();
    std::fs::write(&file, &zlib).unwrap();
    let in_place = tag_said();
    let written = loose.join("tmp_obj");
    std::fs::write(&written, &zlib).unwrap();
    std::fs::rename(&written, &file).unwrap();
    assert_eq!(
        (wrong, in_place, tag_said()),
        (Some(String::new()), Some(String::new()), Some("v1".into())),
        "a tag whose object was wrong was named, read again every answer, or \
         still settled once a sound copy was renamed over it"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_tag_whose_object_is_missing_settles_till_its_object_is_written() {
    let (commit, tag) = ([11; 20], [12; 20]);
    let (dir, loose, zlib) = tagged("missing", commit, tag);
    // No folder for it yet, which git makes as the object arrives.
    std::fs::remove_dir(&loose).unwrap();
    settled(&dir);
    let mut engine = Engine::default();
    let mut tag_said = || {
        engine
            .ask(&dir, &options(&[]), false)
            .map(|answer| answer.tag)
    };
    let missing = tag_said();
    // v1 rewritten in place to name the commit itself, which moves no
    // stamp the tags are kept by, so tags settled aren't read again;
    // then back, so only its object, once read, can name it.
    let v1 = dir.join(".git/refs/tags/v1");
    std::fs::write(&v1, format!("{}\n", hash::Oid(commit).hex())).unwrap();
    let settled = tag_said();
    std::fs::write(&v1, format!("{}\n", hash::Oid(tag).hex())).unwrap();
    // The object written as git writes one: its folder made, and the
    // object written beside it and renamed into place.
    std::fs::create_dir(&loose).unwrap();
    std::fs::write(loose.join("written"), &zlib).unwrap();
    std::fs::rename(
        loose.join("written"),
        loose.join(&hash::Oid(tag).hex()[2..]),
    )
    .unwrap();
    assert_eq!(
        (missing, settled, tag_said()),
        (Some(String::new()), Some(String::new()), Some("v1".into())),
        "a tag whose object is missing was named, was read again with nothing \
         moved, or stayed unread once its object was written"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_tag_whose_object_is_missing_is_read_once_a_pack_brings_it() {
    let (commit, tag) = ([13; 20], [14; 20]);
    let (dir, _, _) = tagged("packed-in", commit, tag);
    settled(&dir);
    let mut engine = Engine::default();
    let mut tag_said = || {
        engine
            .ask(&dir, &options(&[]), false)
            .map(|answer| answer.tag)
    };
    let missing = tag_said();
    // A pack of the one tag object, its size of 71 bytes said in its
    // header's two bytes, as a fetch writes one.
    let object = format!(
        "object {}\ntype commit\ntag v1\n\nv1\n",
        hash::Oid(commit).hex()
    );
    assert_eq!(object.len(), 71, "the header below says another size");
    let entry = [
        &[0xC7, 0x04][..],
        &miniz_oxide::deflate::compress_to_vec_zlib(object.as_bytes(), 1),
    ]
    .concat();
    let packs = dir.join(".git/objects/pack");
    std::fs::write(
        packs.join("pack-01.pack"),
        [b"PACK\0\0\0\x02\0\0\0\x01", &entry[..], &[0; 20]].concat(),
    )
    .unwrap();
    std::fs::write(
        packs.join("pack-01.idx"),
        pack_index::tests::version_2_at(&[tag], &[12]),
    )
    .unwrap();
    assert_eq!(
        (missing, tag_said()),
        (Some(String::new()), Some("v1".into())),
        "a tag whose object is missing was named, or stayed unread once a pack \
         brought its object"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_tag_whose_object_wouldnt_read_leaves_the_others_settled() {
    use std::os::unix::fs::PermissionsExt;
    let (commit, tag) = ([15; 20], [16; 20]);
    let (dir, loose, zlib) = tagged("one-unread", commit, tag);
    let hex = |oid: [u8; 20]| hash::Oid(oid).hex();
    // v1's object is sound but closed to its reader, a fault that
    // passes, so v1 is peeled again each answer; w0, after it by name,
    // names the commit too.
    let object = loose.join(&hex(tag)[2..]);
    std::fs::write(&object, zlib).unwrap();
    let opened = |mode| std::fs::set_permissions(&object, std::fs::Permissions::from_mode(mode));
    opened(0o000).expect("the object closed");
    let w0 = dir.join(".git/refs/tags/w0");
    std::fs::write(&w0, format!("{}\n", hex(commit))).unwrap();
    crate::stamp::tests::ahead(3);
    let mut engine = Engine::default();
    let mut tag_said = || {
        engine
            .ask(&dir, &options(&[]), false)
            .map(|answer| answer.tag)
    };
    let first = tag_said();
    // w0 rewritten in place to name another commit, which moves no
    // stamp, so only tags read again for v1's sake would lose it.
    std::fs::write(&w0, format!("{}\n", hex([17; 20]))).unwrap();
    let second = tag_said();
    // Opened again in place, which moves no stamp either: only a tag
    // left unread is read again to find it.
    opened(0o644).expect("the object opened");
    let opened_again = tag_said();
    assert_eq!(
        (first, second, opened_again),
        (Some("w0".into()), Some("w0".into()), Some("v1".into())),
        "a tag whose object wouldn't read had every other tag read again, or was \
         settled as one that won't read till its file is written again"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_rebase_step_that_isnt_a_file_is_left_unsaid_rather_than_waited_on() {
    let dir = repository("rebase-fifo");
    let merge = dir.join(".git/rebase-merge");
    std::fs::create_dir_all(&merge).unwrap();
    std::fs::write(merge.join("interactive"), "").unwrap();
    std::fs::write(merge.join("end"), "5\n").unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(merge.join("msgnum"))
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made no FIFO in {}", merge.display());
    let (send, heard) = std::sync::mpsc::channel();
    let asked = dir.clone();
    // Apart, so a read that waited on it fails by the message below.
    std::thread::spawn(move || {
        let action = Engine::default()
            .ask(&asked, &options(&[]), false)
            .map(|answer| answer.action);
        let _ = send.send(action);
    });
    let heard = heard.recv_timeout(std::time::Duration::from_secs(30));
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        heard.expect("the engine was still waiting on a rebase's step after 30 s"),
        Some("rebase-i".to_owned()),
        "a rebase whose step isn't a file wasn't said without its step"
    );
}

#[test]
fn a_packed_tag_peels_to_what_its_packed_line_says_without_its_object() {
    let dir = repository("packed-peel");
    let hex = |oid: [u8; 20]| hash::Oid(oid).hex();
    let (commit, tag) = ([18; 20], [19; 20]);
    std::fs::write(
        dir.join(".git/refs/heads/main"),
        format!("{}\n", hex(commit)),
    )
    .unwrap();
    // The tag object isn't held anywhere: only its `^` line can peel it.
    std::fs::write(
        dir.join(".git/packed-refs"),
        format!("{} refs/tags/v1\n^{}\n", hex(tag), hex(commit)),
    )
    .unwrap();
    crate::stamp::tests::ahead(3);
    let tag_said = Engine::default()
        .ask(&dir, &options(&[]), false)
        .map(|answer| answer.tag);
    assert_eq!(
        tag_said,
        Some("v1".into()),
        "a packed tag's peeled line was passed over for its object"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn keeps_a_repository_nothing_changed_in() {
    let dir = repository("still");
    // Asked once its files have settled, since one just made may yet
    // change within the tick it was made in.
    crate::stamp::tests::ahead(3);
    let mut engine = Engine::default();
    engine.ask(&dir, &options(&[]), false);
    engine.let_go_of_changed();
    assert_eq!(engine.repos.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
