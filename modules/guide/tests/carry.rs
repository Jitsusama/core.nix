//! A machine's home folders carried across its reinstall, held against
//! what lsblk printed in the guide's VM (fixtures/qemu-canokey/): packed
//! only when the person chooses, onto a drive that can hold the archive,
//! sealed, read back whole before anything is wiped, and put back owned
//! by the numbers the new system gives each account.

mod common;

use common::Scratch;
use std::path::PathBuf;

use guide::install::carry::{Carried, ChooseCarry, PackHome, RestoreHome, WhenCarried, accounts};
use guide::install::{INSTALL_KEY, TARGET, Target, VAULT_KEY};
use guide::recover::OpenDisk;
use guide::stand_in::StandIn;
use guide::vault::store::Directory;
use guide::vault::{Manifest, Packer, VaultKey};
use guide::{Answer, Answers, Ask, Fault, Machine, Program, Secret, Standing, Step, Tool};

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

const FOLDER: &str = "/run/guide/carry/machine carried home";
const ARCHIVE: &str = "/run/guide/carry/machine carried home/home.tar.zst.age";
const HELD_KEY: &str = "/run/guide/carry-key";
const UUID: &str = "5e3c-0001";

/// What tar printed for the archive, standing in for its bytes.
const PACKED: &[u8] = b"every home folder, as tar packed it";

const OLD_PASSWD: &str = "root:x:0:0::/root:/bin/sh\nalice:x:1000:100::/home/alice:/bin/sh\nnixbld1:x:30001:30000::/var/empty:/bin/false\n";

fn target(leave_out: &[&str]) -> Target {
    Target {
        machine: "machine".into(),
        disk: "/dev/disk/by-id/virtio-root".into(),
        disk_name: "system".into(),
        install_key: INSTALL_KEY.into(),
        layout_packages: vec![],
        system: "/nix/store/abc-nixos-system-machine".into(),
        account: "alice".into(),
        yubikeys: vec!["the Nano".into(), "the 5C".into()],
        recipients: vec![],
        start_from_stick: None,
        clear_secure_boot_keys: None,
        leave_out: leave_out.iter().map(ToString::to_string).collect(),
    }
}

/// A USB drive as lsblk lists it, its one partition holding `filesystem`.
fn usb_drive(name: &str, filesystem: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name, "path": format!("/dev/{name}"), "type": "disk",
        "model": "Flash Drive", "serial": "0123", "size": 64_000_000_000u64,
        "tran": "usb", "rm": true, "mountpoints": [null],
        "children": [{
            "name": format!("{name}1"), "path": format!("/dev/{name}1"), "type": "part",
            "size": 63_000_000_000u64, "tran": null, "rm": true, "partn": 1,
            "fstype": filesystem, "uuid": UUID, "mountpoints": [null]
        }]
    })
}

/// The VM's machine, its stick's vault open holding this target, beside
/// these drives, its old system mounted at the root.
fn on_the_stick(
    scratch: &std::path::Path,
    target: &Target,
    drives: &[serde_json::Value],
) -> StandIn {
    let mut store = Directory::new(PathBuf::from(scratch));
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer
        .add(TARGET, serde_json::to_vec(target).unwrap().as_slice())
        .unwrap();
    packer
        .finish(&Manifest {
            format: 1,
            machine: target.machine.clone(),
            opened_by: vec![],
        })
        .unwrap();
    let mut listing: serde_json::Value =
        serde_json::from_str(fixture!("lsblk-laid-out.json")).unwrap();
    listing["blockdevices"]
        .as_array_mut()
        .unwrap()
        .extend(drives.iter().cloned());
    let mut stand_in = StandIn::new();
    stand_in
        .with_vault(Box::new(store))
        .file(VAULT_KEY, key.to_secret().expose().as_bytes())
        .prints(
            Tool::Lsblk,
            &["--json", "--output-all", "--bytes"],
            &listing.to_string(),
        )
        .file("/mnt/etc/passwd", OLD_PASSWD.as_bytes())
        .no_file(HELD_KEY)
        .no_file("/run/guide/packed");
    stand_in
}

fn compressor() -> String {
    format!(
        "--use-compress-program={} -T0",
        Tool::Zstd.program().expect("the dev shell names zstd")
    )
}

/// What packing runs tar with, leaving out caches and these.
fn packing(leave_out: &[&str]) -> Vec<String> {
    let mut args: Vec<String> = [
        "--create",
        "--file=-",
        "--sparse",
        &compressor(),
        "--numeric-owner",
        "--acls",
        "--xattrs",
        "--xattrs-include=*",
        "--exclude=.cache",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    if !leave_out.is_empty() {
        args.extend(["--anchored".into(), "--no-wildcards-match-slash".into()]);
        args.extend(leave_out.iter().map(|path| format!("--exclude=./*/{path}")));
    }
    args.extend(["--directory=/mnt/home".into(), ".".into()]);
    args
}

fn listing() -> Vec<String> {
    vec!["--list".into(), "--file=-".into(), compressor()]
}

fn strs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

/// A drive with `free` bytes left, home folders of `needed` bytes, which
/// tar packs finishing with `packed` and reads back finishing with `read`.
fn ready_to_pack(
    scratch: &std::path::Path,
    leave_out: &[&str],
    free: u64,
    packed: i32,
    read: i32,
) -> StandIn {
    let mut stand_in = on_the_stick(scratch, &target(leave_out), &[usb_drive("sdb", "exfat")]);
    stand_in.journal().record("carry-home", "carry").unwrap();
    stand_in
        .prints(
            Tool::Mount,
            &["-o", "X-mount.mkdir", "/dev/sdb1", "/run/guide/carry"],
            "",
        )
        .prints(
            Tool::Du,
            &["--summarize", "--bytes", "--exclude=.cache", "/mnt/home"],
            "3000000000\t/mnt/home\n",
        )
        .prints(
            Tool::Df,
            &["--output=avail", "--block-size=1", "/run/guide/carry"],
            &format!("Avail\n{free}\n"),
        )
        .answers(
            Program::Tool(Tool::Tar),
            &strs(&packing(leave_out)),
            packed,
            std::str::from_utf8(PACKED).unwrap(),
            "tar: ./alice/notes: Cannot open: Input/output error",
        )
        .answers(
            Program::Tool(Tool::Tar),
            &strs(&listing()),
            read,
            "",
            "tar: Unexpected EOF in archive",
        )
        .prints(Tool::Umount, &["/run/guide/carry"], "");
    stand_in
}

fn chosen(id: &'static str, at: usize) -> Answers {
    let mut answers = Answers::new();
    answers.give(id, Answer::Chosen(at));
    answers
}

fn programs(stand_in: &StandIn) -> Vec<&str> {
    stand_in
        .calls
        .iter()
        .filter_map(|call| call.split_whitespace().next())
        .filter(|program| {
            program.ends_with("mount") || ["du", "df", "tar", "chown"].contains(program)
        })
        .collect()
}

#[test]
fn the_person_chooses_and_leaving_them_skips_what_carrying_needs() {
    let scratch = Scratch::new("carry-choose");
    let mut stand_in = on_the_stick(&scratch, &target(&[]), &[]);
    let Ok(Standing::Needed { asks }) = ChooseCarry.observe(&mut stand_in) else {
        panic!("not asked")
    };
    assert!(
        matches!(asks.as_slice(), [Ask::Choose { choices, .. }] if choices.len() == 2),
        "{asks:?}"
    );
    ChooseCarry.run(&mut stand_in, &chosen("carry", 1)).unwrap();
    // Nothing asked or run to open the old disk, and nothing packed.
    let open = WhenCarried(Box::new(OpenDisk));
    assert!(matches!(
        open.observe(&mut stand_in),
        Ok(Standing::Done { .. })
    ));
    assert!(matches!(
        PackHome.observe(&mut stand_in),
        Ok(Standing::Done { .. })
    ));
    assert!(matches!(
        RestoreHome.observe(&mut stand_in),
        Ok(Standing::Done { .. })
    ));
    assert_eq!(programs(&stand_in), Vec::<&str>::new());
}

#[test]
fn a_drive_is_offered_only_where_the_archive_fits_never_fat_or_the_stick() {
    let scratch = Scratch::new("carry-drives");
    let mut stand_in = on_the_stick(
        &scratch,
        &target(&[]),
        &[usb_drive("sdb", "vfat"), usb_drive("sdc", "ext4")],
    );
    let Ok(Standing::Needed { asks }) = PackHome.observe(&mut stand_in) else {
        panic!("not asked")
    };
    let [
        Ask::Choose {
            choices, question, ..
        },
    ] = asks.as_slice()
    else {
        panic!("{asks:?}")
    };
    assert_eq!(
        choices,
        &["Flash Drive, 64 GB, serial 0123, its ext4 partition"]
    );
    assert!(question.contains("\"machine carried home\""), "{question}");

    let mut fat_only = on_the_stick(&scratch, &target(&[]), &[usb_drive("sdb", "vfat")]);
    let standing = PackHome.observe(&mut fat_only);
    assert!(
        matches!(&standing, Ok(Standing::Blocked { remedy, .. }) if remedy.contains("FAT won't do")),
        "{standing:?}"
    );
}

#[test]
fn the_home_folders_are_packed_sealed_and_read_back_before_anything_is_wiped() {
    let scratch = Scratch::new("carry-pack");
    let leave_out = [".config/gh/config.yml", ".tpm2_pkcs11"];
    let mut stand_in = ready_to_pack(&scratch, &leave_out, 50_000_000_000, 0, 0);
    PackHome.run(&mut stand_in, &chosen("drive", 0)).unwrap();
    assert_eq!(
        programs(&stand_in),
        ["mount", "du", "df", "tar", "tar", "umount"],
        "{:#?}",
        stand_in.calls
    );
    // The archive opens only with its own key, and holds what tar packed,
    // which is what was read back.
    let key = VaultKey::from_secret(&Secret::new(
        String::from_utf8(stand_in.holds(HELD_KEY).unwrap().to_vec()).unwrap(),
    ))
    .unwrap();
    let sealed = stand_in.holds(ARCHIVE).unwrap().to_vec();
    assert_ne!(sealed, PACKED);
    let mut opened = vec![];
    std::io::Read::read_to_end(&mut key.unsealing(sealed.as_slice()).unwrap(), &mut opened)
        .unwrap();
    assert_eq!(opened, PACKED);
    assert_eq!(
        stand_in.fed.last().map(|(_, fed)| fed.as_slice()),
        Some(PACKED)
    );
    // The stick's vault opens the same key, for a start after a restart.
    let stick = guide::install::vault_key(&mut stand_in).unwrap().unwrap();
    let for_stick = stand_in
        .holds(&format!("{FOLDER}/key-for-the-stick.age"))
        .unwrap()
        .to_vec();
    assert_eq!(
        stick.decrypt(&for_stick).unwrap().as_slice(),
        key.to_secret().expose().as_bytes()
    );
    let carried: Carried =
        serde_json::from_slice(stand_in.holds(&format!("{FOLDER}/carried.json")).unwrap()).unwrap();
    assert_eq!(carried.accounts, accounts(OLD_PASSWD));
    assert_eq!(carried.accounts.len(), 1);
    assert!(!carried.unread);
    let Ok(Standing::Done { evidence }) = PackHome.observe(&mut stand_in) else {
        panic!("not done")
    };
    assert!(evidence.contains("read back whole"), "{evidence}");
}

#[test]
fn a_drive_too_small_packs_nothing() {
    let scratch = Scratch::new("carry-small");
    let mut stand_in = ready_to_pack(&scratch, &[], 1_000_000_000, 0, 0);
    let refused = PackHome.run(&mut stand_in, &chosen("drive", 0));
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("3 GB") && why.contains("1 GB free")),
        "{refused:?}"
    );
    assert_eq!(programs(&stand_in), ["mount", "du", "df", "umount"]);
    assert!(!matches!(
        PackHome.observe(&mut stand_in),
        Ok(Standing::Done { .. })
    ));
}

#[test]
fn an_archive_that_doesnt_read_back_whole_is_never_taken_as_packed() {
    let scratch = Scratch::new("carry-unread");
    let mut stand_in = ready_to_pack(&scratch, &[], 50_000_000_000, 0, 2);
    let refused = PackHome.run(&mut stand_in, &chosen("drive", 0));
    assert!(
        matches!(&refused, Err(Fault::Unproven { standing, .. }) if standing.contains("nothing was wiped")),
        "{refused:?}"
    );
    assert!(stand_in.calls.iter().any(|call| call.starts_with("umount")));
    assert!(!matches!(
        PackHome.observe(&mut stand_in),
        Ok(Standing::Done { .. })
    ));
}

#[test]
fn what_tar_could_not_read_is_kept_beside_the_archive() {
    let scratch = Scratch::new("carry-some-unread");
    let mut stand_in = ready_to_pack(&scratch, &[], 50_000_000_000, 2, 0);
    PackHome.run(&mut stand_in, &chosen("drive", 0)).unwrap();
    assert_eq!(
        stand_in.holds(&format!("{FOLDER}/not carried.txt")),
        Some("tar: ./alice/notes: Cannot open: Input/output error".as_bytes())
    );
    let Ok(Standing::Done { evidence }) = PackHome.observe(&mut stand_in) else {
        panic!("not done")
    };
    assert!(evidence.contains("couldn't be read"), "{evidence}");
}

#[test]
fn a_leave_out_reaching_out_of_a_home_folder_is_refused() {
    let scratch = Scratch::new("carry-escape");
    let mut stand_in = ready_to_pack(&scratch, &["../../etc"], 50_000_000_000, 0, 0);
    let refused = PackHome.run(&mut stand_in, &chosen("drive", 0));
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("isn't a path inside a home folder")),
        "{refused:?}"
    );
    assert!(!stand_in.calls.iter().any(|call| call.starts_with("tar")));
}

/// Packed, then the stick restarted, forgetting the key it held, onto a
/// new system whose passwd gives alice `uid`.
fn packed_then_restarted(scratch: &std::path::Path, uid: u32) -> StandIn {
    let mut stand_in = ready_to_pack(scratch, &[], 50_000_000_000, 0, 0);
    PackHome.run(&mut stand_in, &chosen("drive", 0)).unwrap();
    stand_in.no_file(HELD_KEY).file(
        "/mnt/etc/passwd",
        format!("root:x:0:0::/root:/bin/sh\nalice:x:{uid}:100::/home/alice:/bin/sh\n").as_bytes(),
    );
    let extract: Vec<String> = [
        "--extract",
        "--file=-",
        "--same-permissions",
        &compressor(),
        "--directory=/mnt/home",
        "--numeric-owner",
        "--acls",
        "--xattrs",
        "--xattrs-include=*",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    stand_in
        .link(&format!("/dev/disk/by-uuid/{UUID}"), Some("/dev/sdc1"))
        .prints(
            Tool::Mount,
            &["-o", "X-mount.mkdir,ro", "/dev/sdc1", "/run/guide/carry"],
            "",
        )
        .prints(Tool::Tar, &strs(&extract), "");
    stand_in.calls.clear();
    stand_in
}

#[test]
fn the_home_folders_go_back_after_a_restart_with_the_sticks_copy_of_the_key() {
    let scratch = Scratch::new("carry-restore");
    let mut stand_in = packed_then_restarted(&scratch, 1000);
    assert!(matches!(
        RestoreHome.observe(&mut stand_in),
        Ok(Standing::Needed { .. })
    ));
    RestoreHome.run(&mut stand_in, &Answers::new()).unwrap();
    // Plugged in again as another device, it's found by its filesystem.
    assert_eq!(
        programs(&stand_in),
        ["mount", "tar", "umount"],
        "{:#?}",
        stand_in.calls
    );
    assert_eq!(
        stand_in.fed.last().map(|(_, fed)| fed.as_slice()),
        Some(PACKED)
    );
    let Ok(Standing::Done { evidence }) = RestoreHome.observe(&mut stand_in) else {
        panic!("not done")
    };
    assert!(evidence.contains("back"), "{evidence}");
}

#[test]
fn an_account_numbered_otherwise_on_the_new_system_gets_its_files() {
    let scratch = Scratch::new("carry-renumbered");
    let mut stand_in = packed_then_restarted(&scratch, 1001);
    stand_in.prints(
        Tool::Chown,
        &[
            "--recursive",
            "--no-dereference",
            "--from=1000",
            "1001",
            "/mnt/home/alice",
        ],
        "",
    );
    RestoreHome.run(&mut stand_in, &Answers::new()).unwrap();
    assert_eq!(programs(&stand_in), ["mount", "tar", "chown", "umount"]);
}

#[test]
fn the_drive_unplugged_is_asked_for_before_anything_is_put_back() {
    let scratch = Scratch::new("carry-unplugged");
    let mut stand_in = packed_then_restarted(&scratch, 1000);
    stand_in.link(&format!("/dev/disk/by-uuid/{UUID}"), None);
    let standing = RestoreHome.observe(&mut stand_in);
    assert!(
        matches!(&standing, Ok(Standing::Blocked { why, .. }) if why.contains("isn't plugged in")),
        "{standing:?}"
    );
}

#[test]
fn only_accounts_with_a_home_folder_are_carried() {
    assert_eq!(
        accounts(OLD_PASSWD)
            .iter()
            .map(|account| (account.name.as_str(), account.uid, account.gid))
            .collect::<Vec<_>>(),
        [("alice", 1000, 100)]
    );
}
