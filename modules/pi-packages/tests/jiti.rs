//! What jiti names the entry it keeps for a source, and the removal of
//! a version's entries from a cache, leaving every other entry.

use std::fs;
use std::path::Path;

use pi_packages::jiti;

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[test]
fn md5_agrees_with_the_rfc_and_md5sum() {
    assert_eq!(
        hex(&jiti::md5(b"")),
        "d41d8cd98f00b204e9800998ecf8427e",
        "the empty input"
    );
    assert_eq!(
        hex(&jiti::md5(b"abc")),
        "900150983cd24fb0d6963f7d28e17f72",
        "the RFC's abc"
    );
    assert_eq!(
        hex(&jiti::md5(b"The quick brown fox jumps over the lazy dog")),
        "9e107d9d372bb6826bd81d3542a419d6",
        "a sentence"
    );
}

#[test]
fn an_entry_is_named_by_its_folder_stem_and_path() {
    // A path as long as a version's, so it spans three blocks; its hash
    // is md5sum's.
    let file = Path::new(
        "/home/someone/.local/share/pi-packages/github.com/owner/package/versions/\
         0123456789abcdef0123456789abcdef01234567/extensions/advisor/index.ts",
    );

    let names = jiti::entry_names(file);

    for name in [
        "advisor-index.a520531c.mjs",
        "advisor-index.a520531c.cjs",
        "advisor-index+map.a520531c.mjs",
        "advisor-index.i.a520531c.mjs",
        "advisor-index+map.i.a520531c.cjs",
    ] {
        assert!(names.contains(&name.to_owned()), "{name} not in {names:?}");
    }
    assert_eq!(names.len(), 8, "{names:?}");
}

#[test]
fn a_jsx_source_also_has_its_names_with_an_x() {
    let names = jiti::entry_names(Path::new("/a/view/panel.tsx"));

    assert_eq!(names.len(), 16, "{names:?}");
    assert!(
        names.iter().any(|name| name.starts_with("view-panel.")
            && Path::new(name).extension().is_some_and(|it| it == "mjsx")),
        "{names:?}"
    );
}

#[test]
fn forgetting_a_version_removes_its_entries_and_no_others() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let gone = scratch.path().join("versions/gone");
    let kept = scratch.path().join("versions/kept");
    for version in [&gone, &kept] {
        fs::create_dir_all(version.join("extensions/advisor")).expect("a version");
        fs::write(version.join("extensions/advisor/index.ts"), "").expect("a source");
    }
    let cache = jiti::cache_of(&scratch.path().join("tmp"));
    fs::create_dir_all(&cache).expect("a cache");
    // pi hands jiti the real path, and the scratch folder may sit behind
    // a link.
    let name_of = |version: &Path| {
        let real = fs::canonicalize(version).expect("a version");
        jiti::entry_names(&real.join("extensions/advisor/index.ts"))[0].clone()
    };
    let (gone_entry, kept_entry) = (name_of(&gone), name_of(&kept));
    for name in [
        &gone_entry,
        &kept_entry,
        &"other-thing.12345678.mjs".to_owned(),
    ] {
        fs::write(cache.join(name), "").expect("an entry");
    }

    let removed = jiti::forget(&[gone], &[cache.clone(), scratch.path().join("none")]);

    assert_eq!(removed, 1, "only the gone version's entry goes");
    assert!(
        !cache.join(&gone_entry).exists(),
        "the gone version's entry was kept"
    );
    assert!(
        cache.join(&kept_entry).exists(),
        "a kept version's entry went"
    );
    assert!(
        cache.join("other-thing.12345678.mjs").exists(),
        "an unrelated entry went"
    );
}
