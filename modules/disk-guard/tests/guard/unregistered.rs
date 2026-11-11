use crate::garbage::{LISTING, script};
use disk_guard::garbage::list;
use std::fs;

#[test]
fn what_nix_never_registered_is_garbage_as_large_as_its_blocks() {
    // A build killed before Nix registered its output leaves it in the
    // store, where the listing names it dead and `nix path-info` knows
    // nothing of it: on 11 November, 616 such entries held 67.5 GiB of a
    // laptop's disk, mostly disk images, and no pass ever offered one.
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    let left = store.join("aaaa-laptop-disko-images");
    fs::create_dir_all(&left).unwrap();
    fs::write(left.join("written"), vec![7u8; 64 << 10]).unwrap();
    // An image's length is far more than it holds: one of 2 TiB held 22.
    fs::File::create(left.join("main.qcow2"))
        .unwrap()
        .set_len(1 << 30)
        .unwrap();
    let gone = store.join("bbbb-gone");
    // The lock a builder takes on its output is listed dead too, and may
    // be held as the guard looks.
    let lock = store.join("cccc-laptop-disko-images.lock");
    fs::write(&lock, "").unwrap();
    let nix = dir.path().join("nix");
    script(
        &dir.path().join("nix-store"),
        &format!(
            "[ \"$*\" = '{LISTING}' ] || exit 9\n\
             echo /nix/store/p0\necho {}\necho {}\necho {}\n",
            left.display(),
            gone.display(),
            lock.display()
        ),
    );
    script(
        &nix,
        "[ \"$1 $2\" = '--extra-experimental-features nix-command' ] || exit 9\nshift 2\n\
         [ \"$1 $2 $3\" = 'path-info --json --offline' ] || exit 9\nshift 3\n\
         printf '{\"/nix/store/p0\":{\"narSize\":1,\"registrationTime\":5,\"references\":[]}'\n\
         printf ',\"%s\":null,\"%s\":null,\"%s\":null}' \"$2\" \"$3\" \"$4\"\n",
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let dead = list(&nix, false).unwrap();
    let paths: Vec<&str> = dead.iter().map(|dead| dead.path.as_str()).collect();
    assert_eq!(
        paths,
        ["/nix/store/p0", left.to_str().unwrap()],
        "what Nix never registered is listed, and neither what's gone nor a lock is"
    );
    let found = &dead[1];
    assert!(
        (64 << 10..1 << 20).contains(&found.size),
        "an unregistered entry weighs its blocks, not its length: {}",
        found.size
    );
    assert!(
        (now - found.registered).abs() < 60.0,
        "an unregistered entry is dated by when it was last written: {} against {now}",
        found.registered
    );
    assert!(found.refers.is_empty(), "nothing registered refers to it");
}
