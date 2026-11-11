use super::store::Directory;
use super::*;

#[cfg(target_os = "linux")]
#[test]
fn an_ended_plugin_left_unwaited_is_waited_on_and_nothing_else() {
    let directory = crate::Scratch::new("vault-reap");
    std::fs::create_dir_all(&directory).unwrap();
    let plugin = directory.join("age-plugin-test");
    let other = directory.join("not-a-plugin");
    std::os::unix::fs::symlink("/bin/sh", &plugin).unwrap();
    std::os::unix::fs::symlink("/bin/sh", &other).unwrap();
    let run = |program: &std::path::Path| {
        std::process::Command::new(program)
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap()
    };
    // Dropped unwaited, as age drops its plugin.
    let plugin = run(&plugin).id() as libc::pid_t;
    let mut other = run(&other);
    // Both have ended, so a reaper that took any child would take both.
    let ended = || {
        let all = ended_children("");
        all.contains(&plugin) && all.contains(&(other.id() as libc::pid_t))
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ended() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(ended(), "the children never showed as ended");

    reap_plugins();

    assert!(
        !std::path::Path::new(&format!("/proc/{plugin}")).exists(),
        "the plugin {plugin} was left unreaped"
    );
    // Waiting fails only when the child was taken; how it ended doesn't
    // matter, and a build's busybox sh ends any name it lacks with 127.
    assert!(other.wait().is_ok(), "another child was taken");
}

fn manifest() -> Manifest {
    Manifest {
        format: 1,
        machine: "a laptop".into(),
        opened_by: vec!["the stick's passphrase".into()],
    }
}

#[test]
fn what_was_packed_comes_back_across_objects() {
    let directory = crate::Scratch::new("vault-back");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let big: Vec<u8> = (0..10_000u32).flat_map(u32::to_le_bytes).collect();
    let mut packer = Packer::new(&mut store, &key);
    packer.object_bytes = 4096;
    packer.add("cache/nar/big.nar.zst", &big[..]).unwrap();
    packer.add("empty", &b""[..]).unwrap();
    packer.finish(&manifest()).unwrap();

    let mut vault = Vault::open(&mut store, key).unwrap();
    let paths: Vec<&str> = vault
        .files()
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(paths, ["cache/nar/big.nar.zst", "empty"]);
    let mut back = vec![];
    vault.extract("cache/nar/big.nar.zst", &mut back).unwrap();
    assert_eq!(back, big);
    let mut empty = vec![];
    vault.extract("empty", &mut empty).unwrap();
    assert_eq!(empty, b"");
}

#[test]
fn a_finished_vault_takes_a_file_and_one_in_place_of_another() {
    let directory = crate::Scratch::new("vault-put");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let again = || VaultKey::from_secret(&key.to_secret()).unwrap();
    let mut packer = Packer::new(&mut store, &key);
    packer.add("target.json", &b"{}"[..]).unwrap();
    packer.add("sbctl/GUID", &b"old"[..]).unwrap();
    packer.finish(&manifest()).unwrap();

    let mut vault = Vault::open(&mut store, again()).unwrap();
    vault.put("sbctl/GUID", b"new").unwrap();
    vault.put("sbctl/keys/db/db.pem", b"the db").unwrap();
    // Put again unchanged, nothing is written: no object, and not the
    // index, which sealing again would change.
    let listed = || {
        let mut names: Vec<_> = std::fs::read_dir(&*directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    let (names, index) = (listed(), std::fs::read(directory.join(INDEX)).unwrap());
    vault.put("sbctl/keys/db/db.pem", b"the db").unwrap();
    assert_eq!(listed(), names);
    assert_eq!(std::fs::read(directory.join(INDEX)).unwrap(), index);

    let mut vault = Vault::open(&mut store, again()).unwrap();
    let mut paths: Vec<&str> = vault
        .files()
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    paths.sort_unstable();
    assert_eq!(paths, ["sbctl/GUID", "sbctl/keys/db/db.pem", "target.json"]);
    for (path, expected) in [
        ("target.json", &b"{}"[..]),
        ("sbctl/GUID", b"new"),
        ("sbctl/keys/db/db.pem", b"the db"),
    ] {
        let mut bytes = vec![];
        vault.extract(path, &mut bytes).unwrap();
        assert_eq!(bytes, expected, "{path}");
    }
    // The manifest still says whose vault it is.
    assert_eq!(super::manifest(&mut store).unwrap(), manifest());
}

#[test]
fn a_vault_stopped_as_its_new_index_takes_the_old_ones_place_opens_as_it_is_now() {
    let directory = crate::Scratch::new("vault-put-stopped");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let again = || VaultKey::from_secret(&key.to_secret()).unwrap();
    let mut packer = Packer::new(&mut store, &key);
    packer.add("sbctl/GUID", &b"old"[..]).unwrap();
    packer.finish(&manifest()).unwrap();
    let mut vault = Vault::open(&mut store, again()).unwrap();
    vault
        .put_all([
            ("sbctl/GUID".to_string(), &b"new"[..]),
            ("sbctl/keys/db/db.pem".to_string(), &b"the db"[..]),
        ])
        .unwrap();
    assert!(
        !directory.join(INDEX_NEW).exists(),
        "the new index was left beside the one it replaced"
    );

    // As FAT leaves it between taking the old index away and naming
    // the new one.
    std::fs::rename(directory.join(INDEX), directory.join(INDEX_NEW)).unwrap();
    let mut vault = Vault::open(&mut store, again()).unwrap();
    let mut bytes = vec![];
    vault.extract("sbctl/GUID", &mut bytes).unwrap();
    assert_eq!(bytes, b"new");
}

#[test]
fn no_name_shows_outside_the_manifest() {
    let directory = crate::Scratch::new("vault-names");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer
        .add("secure-boot/db.key", &b"a private key"[..])
        .unwrap();
    packer.finish(&manifest()).unwrap();
    let mut shown = String::new();
    for entry in walk(&directory) {
        if entry.ends_with(MANIFEST) {
            continue;
        }
        shown += &entry;
        shown += &String::from_utf8_lossy(&std::fs::read(&entry).unwrap());
    }
    assert!(!shown.contains("secure-boot"), "{shown}");
    assert!(
        !shown.contains("a private key"),
        "a sealed file shows a private key: {shown}"
    );
}

fn walk(directory: &std::path::Path) -> Vec<String> {
    let mut found = vec![];
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path.display().to_string());
        }
    }
    found
}

#[test]
fn a_changed_object_is_refused() {
    let directory = crate::Scratch::new("vault-changed");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer.add("state", &b"what was carried"[..]).unwrap();
    packer.finish(&manifest()).unwrap();
    let object = directory.join("00000000.age");
    let mut bytes = std::fs::read(&object).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    std::fs::write(&object, bytes).unwrap();

    let mut vault = Vault::open(&mut store, key).unwrap();
    assert!(
        matches!(
            vault.extract("state", &mut vec![]),
            Err(Fault::Unopened { .. })
        ),
        "a changed object was extracted as if packed"
    );
    assert!(
        matches!(vault.read_back(), Err(Fault::Unopened { .. })),
        "reading the vault back took a changed object"
    );
}

#[test]
fn reading_a_vault_back_names_each_file_by_its_hash() {
    let directory = crate::Scratch::new("vault-read-back");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer.add("sbctl/db.key", &b"a key"[..]).unwrap();
    packer.add("empty", &b""[..]).unwrap();
    packer.finish(&manifest()).unwrap();

    let listed: Vec<(String, String)> = Vault::open(&mut store, key)
        .unwrap()
        .read_back()
        .unwrap()
        .into_iter()
        .map(|file| (file.sha256, file.path))
        .collect();
    assert_eq!(
        listed,
        [
            (hex(&Sha256::digest(b"a key")), "sbctl/db.key".to_string()),
            (hex(&Sha256::digest(b"")), "empty".to_string()),
        ]
    );
}

#[test]
fn objects_swapped_between_files_are_refused() {
    let directory = crate::Scratch::new("vault-swapped");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer.add("first", &b"one thing"[..]).unwrap();
    packer.add("second", &b"other one"[..]).unwrap();
    packer.finish(&manifest()).unwrap();
    let first = directory.join("00000000.age");
    let second = directory.join("00000001.age");
    let (a, b) = (
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap(),
    );
    std::fs::write(&first, b).unwrap();
    std::fs::write(&second, a).unwrap();

    let mut vault = Vault::open(&mut store, key).unwrap();
    assert!(
        matches!(
            vault.extract("first", &mut vec![]),
            Err(Fault::Unopened { why, .. }) if why == "it isn't what was packed"
        ),
        "swapped objects were extracted as if packed"
    );
}

#[test]
fn the_key_opens_with_either_key_or_the_passphrase_and_nothing_else() {
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    let five = age::x25519::Identity::generate();
    let sealed = key
        .seal(&[&nano.to_public() as &dyn age::Recipient, &five.to_public()])
        .unwrap();
    let passphrase = Secret::new("correct horse battery staple".into());
    let sealed_with_passphrase = key.seal_with_passphrase(&passphrase, Some(10)).unwrap();
    let same = |opened: VaultKey| opened.recipient().to_string() == key.recipient().to_string();

    assert!(
        same(VaultKey::open(&sealed, &nano).unwrap()),
        "opening with nano gave another vault key"
    );
    assert!(
        same(VaultKey::open(&sealed, &five).unwrap()),
        "opening with five gave another vault key"
    );
    assert!(
        same(VaultKey::open_with_passphrase(&sealed_with_passphrase, &passphrase).unwrap()),
        "opening with the passphrase gave another vault key"
    );
    let stranger = age::x25519::Identity::generate();
    assert!(
        VaultKey::open(&sealed, &stranger).is_err(),
        "a stranger's key opened the vault key"
    );
    let wrong = Secret::new("incorrect horse".into());
    assert!(
        VaultKey::open_with_passphrase(&sealed_with_passphrase, &wrong).is_err(),
        "a wrong passphrase opened the vault key"
    );
}

#[test]
fn a_vault_never_finished_wont_open() {
    let directory = crate::Scratch::new("vault-unfinished");
    let mut store = Directory::new(&directory);
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer.add("state", &b"half"[..]).unwrap();
    drop(packer);
    assert!(
        matches!(
            Vault::open(&mut store, key),
            Err(Fault::Unopened { why, .. }) if why == "it was never finished"
        ),
        "a vault never finished opened"
    );
}

#[test]
fn only_relative_paths_are_packed() {
    for path in ["/etc/shadow", "../outside", "a//b", "a/./b", ""] {
        assert!(checked(path).is_err(), "{path:?}");
    }
    assert_eq!(
        checked("home/.ssh/id_ed25519_sk").unwrap(),
        "home/.ssh/id_ed25519_sk"
    );
}
