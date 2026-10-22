//! The guide's signatures against sbsign's: the same key, the same
//! program and the same moment have to give the same bytes, since what
//! sbsign makes is what every machine here boots. The programs are the
//! ones systemd builds for EFI, from `GUIDE_TEST_EFI`, and the keys are
//! made as sbctl makes them, by openssl, from the package's check inputs
//! or the dev shell's.

mod common;

use common::Scratch;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use guide::Fault;
use guide::sign::Signer;

fn run(program: &str, args: &[&std::ffi::OsStr]) -> std::process::Output {
    let output = Command::new(program).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{program}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// A key and certificate as sbctl makes a db key: RSA-4096, PKCS #8.
fn make_key(directory: &Path, bits: u32) -> (PathBuf, PathBuf) {
    let (key, certificate) = (directory.join("db.key"), directory.join("db.pem"));
    run(
        "openssl",
        &[
            "req".as_ref(),
            "-new".as_ref(),
            "-x509".as_ref(),
            "-newkey".as_ref(),
            format!("rsa:{bits}").as_ref(),
            "-nodes".as_ref(),
            "-days".as_ref(),
            "3650".as_ref(),
            "-subj".as_ref(),
            "/CN=Database Key".as_ref(),
            "-keyout".as_ref(),
            key.as_os_str(),
            "-out".as_ref(),
            certificate.as_os_str(),
        ],
    );
    (key, certificate)
}

/// One db key for every test, since making an RSA-4096 key takes seconds.
fn db_key() -> &'static (PathBuf, PathBuf) {
    static KEY: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    KEY.get_or_init(|| {
        let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("guide-sign-db");
        fs::create_dir_all(&directory).unwrap();
        make_key(&directory, 4096)
    })
}

fn programs() -> Vec<PathBuf> {
    let directory =
        std::env::var("GUIDE_TEST_EFI").expect("GUIDE_TEST_EFI names systemd's EFI programs");
    let found: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().contains(".efi"))
        .collect();
    assert!(found.len() >= 2, "{found:?}");
    found
}

/// The signing time sbsign put in a signature, as the moment it names.
fn signing_time(signed: &[u8]) -> SystemTime {
    let marker = [
        0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x05, 0x31, 0x0f, 0x17, 0x0d,
    ];
    let at = signed
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("a signing time")
        + marker.len();
    let text = std::str::from_utf8(&signed[at..at + 12]).unwrap();
    let field = |range: std::ops::Range<usize>| text[range].parse::<i64>().unwrap();
    let (year, month, day) = (2000 + field(0..2), field(2..4), field(4..6));
    // Howard Hinnant's civil-to-days.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let seconds = days * 86_400 + field(6..8) * 3600 + field(8..10) * 60 + field(10..12);
    UNIX_EPOCH + Duration::from_secs(seconds as u64)
}

#[test]
fn every_signature_is_byte_for_byte_what_sbsign_makes() {
    let directory = Scratch::new("sbsign");
    let (key, certificate) = db_key();
    let signer = Signer::open(key.parent().unwrap()).unwrap();
    for program in programs() {
        let by_sbsign = directory.join("sbsign.efi");
        run(
            "sbsign",
            &[
                "--key".as_ref(),
                key.as_os_str(),
                "--cert".as_ref(),
                certificate.as_os_str(),
                "--output".as_ref(),
                by_sbsign.as_os_str(),
                program.as_os_str(),
            ],
        );
        let expected = fs::read(&by_sbsign).unwrap();
        let signed = signer
            .sign(fs::read(&program).unwrap(), signing_time(&expected))
            .unwrap();
        assert!(
            signed == expected,
            "{} differs from sbsign's",
            program.display()
        );

        // Signed again, as a stick written for a machine is once its
        // install has its keys, it's what signing it the first time made.
        let again = signer
            .sign(signed.clone(), signing_time(&expected))
            .unwrap();
        assert!(
            again == expected,
            "{} signed again differs",
            program.display()
        );

        let by_guide = directory.join("guide.efi");
        fs::write(&by_guide, &signed).unwrap();
        run(
            "sbverify",
            &[
                "--cert".as_ref(),
                certificate.as_os_str(),
                by_guide.as_os_str(),
            ],
        );
    }
}

#[test]
fn a_certificate_that_isnt_the_keys_is_refused() {
    let other = Scratch::new("other");
    let (_, other_certificate) = make_key(&other, 2048);
    let (key, _) = db_key();
    let result = Signer::new(
        &fs::read_to_string(key).unwrap(),
        &fs::read_to_string(other_certificate).unwrap(),
    );
    assert!(
        matches!(result, Err(Fault::Blocked { .. })),
        "{:?}",
        result.err()
    );
}

#[test]
fn a_key_that_isnt_pkcs8_is_refused() {
    let (_, certificate) = db_key();
    let result = Signer::new(
        "-----BEGIN TSS2 PRIVATE KEY-----\n-----END TSS2 PRIVATE KEY-----\n",
        &fs::read_to_string(certificate).unwrap(),
    );
    assert!(
        matches!(result, Err(Fault::Unreadable { .. })),
        "{:?}",
        result.err()
    );
}
