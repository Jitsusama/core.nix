//! chrome-pack-extension packs a folder as a Chrome extension file (CRX3),
//! signed with a key given, so Chrome can install it from a local file as
//! Linux lets a preferences file name one. Chrome has no way to pack in a
//! build, since it won't run there, and branded Chrome stopped loading an
//! unpacked folder in version 137.
//!
//! A CRX3 is "Cr24", the version 3, the length of a header and the header,
//! then the folder zipped. The header holds the public key and its RSA
//! signature over `CRX3 SignedData`, a nul, the length of the signed data,
//! the signed data (the extension's id) and the zip, in Chrome's protocol
//! buffer layout (`crx3.proto`). The id is the first sixteen bytes of the
//! public key's SHA-256, each half-byte a letter from a to p.
//!
//! usage: chrome-pack-extension FOLDER KEY.pem OUT.crx, printing the id.

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rsa::RsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::{DecodePrivateKey, EncodePublicKey};
use rsa::signature::{SignatureEncoding, Signer};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

/// What leads what's signed, as Chrome reads it.
const SIGNED_DATA: &[u8] = b"CRX3 SignedData\0";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [folder, key, out] = args.as_slice() else {
        eprintln!("usage: chrome-pack-extension FOLDER KEY.pem OUT.crx");
        return ExitCode::from(2);
    };
    match pack(
        Path::new(folder),
        &fs::read_to_string(key).unwrap_or_default(),
    ) {
        Ok((id, crx)) => match fs::write(out, crx) {
            Ok(()) => {
                println!("{id}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("chrome-pack-extension: can't write {out}: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("chrome-pack-extension: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The folder packed and signed with the key, and the extension's id.
fn pack(folder: &Path, key: &str) -> Result<(String, Vec<u8>), String> {
    let key = RsaPrivateKey::from_pkcs8_pem(key)
        .map_err(|error| format!("the key isn't an RSA key in PKCS #8: {error}"))?;
    let public = key
        .to_public_key()
        .to_public_key_der()
        .map_err(|error| format!("can't say the public key: {error}"))?
        .into_vec();
    let crx_id: [u8; 16] = Sha256::digest(&public)[..16]
        .try_into()
        .expect("SHA-256 is longer than sixteen bytes");
    let zipped = zipped(folder)?;
    let signed = field(1, &crx_id);
    let mut message = SIGNED_DATA.to_vec();
    message.extend_from_slice(&u32_le(signed.len()));
    message.extend_from_slice(&signed);
    message.extend_from_slice(&zipped);
    let signature = SigningKey::<Sha256>::new(key).sign(&message).to_vec();
    let proof = [field(1, &public), field(2, &signature)].concat();
    let header = [field(2, &proof), field(10000, &signed)].concat();
    let mut crx = b"Cr24".to_vec();
    crx.extend_from_slice(&3_u32.to_le_bytes());
    crx.extend_from_slice(&u32_le(header.len()));
    crx.extend_from_slice(&header);
    crx.extend_from_slice(&zipped);
    Ok((id(&crx_id), crx))
}

/// The id Chrome gives an extension: each half-byte as a to p.
fn id(crx_id: &[u8; 16]) -> String {
    crx_id
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .map(|half| char::from(b'a' + half))
        .collect()
}

/// The folder's files zipped in the order of their paths, every one dated
/// alike, so the same folder packs to the same bytes.
fn zipped(folder: &Path) -> Result<Vec<u8>, String> {
    let mut files = Vec::new();
    gather(folder, folder, &mut files)?;
    files.sort();
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (name, path) in files {
        let bytes = fs::read(&path).map_err(|error| format!("can't read {name}: {error}"))?;
        zip.start_file(name.as_str(), options)
            .and_then(|()| zip.write_all(&bytes).map_err(Into::into))
            .map_err(|error| format!("can't zip {name}: {error}"))?;
    }
    zip.finish()
        .map(Cursor::into_inner)
        .map_err(|error| format!("can't zip the folder: {error}"))
}

/// Every file under `at`, each with its path from the folder's top.
fn gather(top: &Path, at: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let entries =
        fs::read_dir(at).map_err(|error| format!("can't read {}: {error}", at.display()))?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            gather(top, &path, files)?;
        } else {
            let name = path
                .strip_prefix(top)
                .expect("found under the top")
                .to_string_lossy()
                .into_owned();
            files.push((name, path));
        }
    }
    Ok(())
}

/// A length-delimited field of a protocol buffer: its tag, its length and
/// its bytes.
fn field(number: u32, bytes: &[u8]) -> Vec<u8> {
    let mut said = varint(u64::from(number) << 3 | 2);
    said.extend(varint(bytes.len() as u64));
    said.extend_from_slice(bytes);
    said
}

fn varint(mut value: u64) -> Vec<u8> {
    let mut said = Vec::new();
    loop {
        let low = u8::try_from(value & 0x7f).expect("seven bits");
        value >>= 7;
        if value == 0 {
            said.push(low);
            return said;
        }
        said.push(low | 0x80);
    }
}

fn u32_le(length: usize) -> [u8; 4] {
    u32::try_from(length)
        .expect("an extension is smaller than four gigabytes")
        .to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::RsaPublicKey;
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;

    // A key of the tests' own, whose id openssl gives as
    // `openssl pkey -pubout -outform DER | sha256sum`, each half-byte a to p.
    const KEY: &str = include_str!("test-key.pem");

    fn folder() -> tempfolder::Folder {
        let folder = tempfolder::Folder::new();
        fs::write(folder.path().join("manifest.json"), "{}").unwrap();
        fs::create_dir(folder.path().join("lib")).unwrap();
        fs::write(folder.path().join("lib/a.js"), "a").unwrap();
        folder
    }

    /// Reads a length-delimited field from the start of `bytes`: its
    /// number, its bytes and what follows.
    fn read_field(bytes: &[u8]) -> (u32, &[u8], &[u8]) {
        let (tag, rest) = read_varint(bytes);
        let (length, rest) = read_varint(rest);
        let length = usize::try_from(length).unwrap();
        (
            u32::try_from(tag >> 3).unwrap(),
            &rest[..length],
            &rest[length..],
        )
    }

    fn read_varint(bytes: &[u8]) -> (u64, &[u8]) {
        let mut value = 0;
        for (at, byte) in bytes.iter().enumerate() {
            value |= u64::from(byte & 0x7f) << (7 * at);
            if byte & 0x80 == 0 {
                return (value, &bytes[at + 1..]);
            }
        }
        panic!("a varint runs off the end");
    }

    #[test]
    fn the_id_is_the_public_keys_digest_as_letters() {
        assert_eq!(
            id(&[
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0, 0, 0, 0, 0, 0, 0, 0xff
            ]),
            "abcdefghijklmnopaaaaaaaaaaaaaapp"
        );
        let (said, _) = pack(folder().path(), KEY).unwrap();
        assert_eq!(said, "oaencadengkhpbigmiekloeedfpgadam");
    }

    #[test]
    fn the_same_folder_packs_to_the_same_bytes() {
        let folder = folder();
        assert_eq!(
            pack(folder.path(), KEY).unwrap().1,
            pack(folder.path(), KEY).unwrap().1
        );
    }

    #[test]
    fn a_crx3_holds_the_key_signing_its_id_and_zip_as_chrome_checks() {
        let (said, crx) = pack(folder().path(), KEY).unwrap();
        assert_eq!(&crx[..4], b"Cr24");
        assert_eq!(u32::from_le_bytes(crx[4..8].try_into().unwrap()), 3);
        let length = usize::try_from(u32::from_le_bytes(crx[8..12].try_into().unwrap())).unwrap();
        let (header, zipped) = crx[12..].split_at(length);

        let (number, proof, rest) = read_field(header);
        assert_eq!(number, 2);
        let (number, signed, rest) = read_field(rest);
        assert_eq!((number, rest.len()), (10000, 0));
        let (number, public, rest) = read_field(proof);
        assert_eq!(number, 1);
        let (number, signature, rest) = read_field(rest);
        assert_eq!((number, rest.len()), (2, 0));
        let (number, crx_id, rest) = read_field(signed);
        assert_eq!((number, rest.len()), (1, 0));
        assert_eq!(id(crx_id.try_into().unwrap()), said);
        assert_eq!(&Sha256::digest(public)[..16], crx_id);

        let mut message = SIGNED_DATA.to_vec();
        message.extend_from_slice(&u32_le(signed.len()));
        message.extend_from_slice(signed);
        message.extend_from_slice(zipped);
        let key = VerifyingKey::<Sha256>::new(RsaPublicKey::from_public_key_der(public).unwrap());
        key.verify(&message, &Signature::try_from(signature).unwrap())
            .unwrap();
        message.push(0);
        assert!(
            key.verify(&message, &Signature::try_from(signature).unwrap())
                .is_err()
        );

        let mut archive = zip::ZipArchive::new(Cursor::new(zipped)).unwrap();
        let names: Vec<_> = (0..archive.len())
            .map(|at| archive.by_index(at).unwrap().name().to_owned())
            .collect();
        assert_eq!(names, ["lib/a.js", "manifest.json"]);
    }

    #[test]
    fn a_key_that_isnt_one_is_said() {
        let error = pack(folder().path(), "not a key").unwrap_err();
        assert!(error.starts_with("the key isn't an RSA key"), "{error}");
    }

    /// A folder of its own under the system's, gone when dropped.
    mod tempfolder {
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COUNT: AtomicUsize = AtomicUsize::new(0);

        pub struct Folder(PathBuf);

        impl Folder {
            pub fn new() -> Self {
                let at = std::env::temp_dir().join(format!(
                    "chrome-pack-extension-{}-{}",
                    std::process::id(),
                    COUNT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&at).unwrap();
                Folder(at)
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for Folder {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}
