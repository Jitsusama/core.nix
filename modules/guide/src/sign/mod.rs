//! Signing an EFI program for Secure Boot with a machine's own db key, so
//! the stick boots on that machine with Secure Boot enforced. The
//! signature is byte for byte what sbsign makes from the same key at the
//! same moment, which every machine here boots, rather than a smaller one
//! of our own design no firmware has been tried against. sbsign itself is
//! Linux's alone, and a Mac writes sticks too. A program may be signed
//! beside the signatures it carries, by several machines' keys, so one
//! stick boots on each of them.

pub mod der;
pub mod pe;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::Fault;
use der::{OCTET_STRING, algorithm, oid, sequence, set_of, small_integer, value};
use pe::Program;

/// The program's Authenticode digest, over what its signature covers.
fn digest(program: &Program) -> [u8; 32] {
    let mut hash = Sha256::new();
    for part in program.covered() {
        hash.update(part);
    }
    hash.finalize().into()
}

const SHA256: &[u64] = &[2, 16, 840, 1, 101, 3, 4, 2, 1];
const RSA_ENCRYPTION: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const SIGNED_DATA: &[u64] = &[1, 2, 840, 113_549, 1, 7, 2];
const CONTENT_TYPE: &[u64] = &[1, 2, 840, 113_549, 1, 9, 3];
const MESSAGE_DIGEST: &[u64] = &[1, 2, 840, 113_549, 1, 9, 4];
const SIGNING_TIME: &[u64] = &[1, 2, 840, 113_549, 1, 9, 5];
const SMIME_CAPABILITIES: &[u64] = &[1, 2, 840, 113_549, 1, 9, 15];
const SPC_INDIRECT_DATA: &[u64] = &[1, 3, 6, 1, 4, 1, 311, 2, 1, 4];
const SPC_PE_IMAGE_DATA: &[u64] = &[1, 3, 6, 1, 4, 1, 311, 2, 1, 15];

/// What signing a SHA-256 digest with PKCS #1 v1.5 puts before it.
const SHA256_DIGEST_INFO: &[u8] = &[
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
    0x00, 0x04, 0x20,
];

/// "<<<Obsolete>>>" in UTF-16, where Authenticode once named the file.
const OBSOLETE: &[u8] = &[
    0x00, 0x3c, 0x00, 0x3c, 0x00, 0x3c, 0x00, 0x4f, 0x00, 0x62, 0x00, 0x73, 0x00, 0x6f, 0x00, 0x6c,
    0x00, 0x65, 0x00, 0x74, 0x00, 0x65, 0x00, 0x3e, 0x00, 0x3e, 0x00, 0x3e,
];

/// The S/MIME capabilities OpenSSL lists in every signature it makes. They
/// mean nothing to Authenticode, and are here so the signature is sbsign's.
fn smime_capabilities() -> Vec<u8> {
    let cipher = |arcs: &[u64]| sequence(&[&oid(arcs)]);
    let rc2 = |bits: u8| sequence(&[&oid(&[1, 2, 840, 113_549, 3, 2]), &small_integer(bits)]);
    sequence(&[
        &cipher(&[2, 16, 840, 1, 101, 3, 4, 1, 42]),
        &cipher(&[2, 16, 840, 1, 101, 3, 4, 1, 22]),
        &cipher(&[2, 16, 840, 1, 101, 3, 4, 1, 2]),
        &cipher(&[1, 2, 840, 113_549, 3, 7]),
        &rc2(128),
        &rc2(64),
        &cipher(&[1, 3, 14, 3, 2, 7]),
        &rc2(40),
    ])
}

fn attribute(kind: &[u64], value: &[u8]) -> Vec<u8> {
    sequence(&[&oid(kind), &set_of(vec![value.to_vec()])])
}

fn refused(what: &str) -> impl Fn(rsa::Error) -> Fault + '_ {
    move |why| Fault::unreadable(what, why)
}

/// A machine's db key and its certificate, as sbctl keeps them.
pub struct Signer {
    key: RsaPrivateKey,
    certificate: Vec<u8>,
    issuer_and_serial: Vec<u8>,
}

impl Signer {
    /// From sbctl's PKCS #8 key and PEM certificate. Refused unless the
    /// certificate is the key's, since firmware would refuse what they sign.
    pub fn new(key: &str, certificate: &str) -> Result<Signer, Fault> {
        let key = RsaPrivateKey::from_pkcs8_pem(key)
            .map_err(|why| Fault::unreadable("the db key", why))?;
        let (label, certificate) = rsa::pkcs8::der::pem::decode_vec(certificate.as_bytes())
            .map_err(|why| Fault::unreadable("the db certificate", why))?;
        if label != "CERTIFICATE" {
            return Err(Fault::unreadable(
                "the db certificate",
                "it isn't a certificate",
            ));
        }
        let what = "the db certificate";
        let (whole, _) = der::read(&certificate, what)?;
        let signed = *der::children(whole, what)?
            .first()
            .ok_or_else(|| Fault::unreadable(what, "it's empty"))?;
        let fields = der::children(signed, what)?;
        // A version, when there is one, comes first and is tagged [0].
        let fields = if fields.first().is_some_and(|field| field.tag == 0xa0) {
            &fields[1..]
        } else {
            &fields[..]
        };
        let [serial, _, issuer, _, _, public, ..] = fields else {
            return Err(Fault::unreadable(
                what,
                "it's missing fields a certificate has",
            ));
        };
        let public = RsaPublicKey::from_public_key_der(public.encoding)
            .map_err(|why| Fault::unreadable(what, why))?;
        if public != key.to_public_key() {
            return Err(Fault::Blocked {
                step: "sign-stick",
                why:
                    "the db certificate isn't the db key's, so firmware would refuse what it signs"
                        .into(),
            });
        }
        Ok(Signer {
            issuer_and_serial: sequence(&[issuer.encoding, serial.encoding]),
            key,
            certificate,
        })
    }

    /// From sbctl's db key directory, holding `db.key` and `db.pem`.
    pub fn open(directory: &Path) -> Result<Signer, Fault> {
        let read = |name: &str| {
            let path = directory.join(name);
            std::fs::read_to_string(&path)
                .map(Zeroizing::new)
                .map_err(|why| Fault::unreadable(path.display().to_string(), why))
        };
        Signer::new(&read("db.key")?, &read("db.pem")?)
    }

    /// Whether the signature `program` carries is this signer's, over the
    /// program as it is now: it holds this signer's certificate and the
    /// program's digest. The firmware checks the rest as it boots it.
    pub fn has_signed(&self, program: &[u8]) -> bool {
        let Some(signature) = pe::signature(program) else {
            return false;
        };
        let Ok(parsed) = Program::parse(program.to_vec()) else {
            return false;
        };
        let holds = |part: &[u8]| signature.windows(part.len()).any(|window| window == part);
        holds(&self.certificate) && holds(&digest(&parsed))
    }

    /// The program signed, as of `at`, this its only signature.
    pub fn sign(&self, program: Vec<u8>, at: SystemTime) -> Result<Vec<u8>, Fault> {
        let program = Program::parse(program)?;
        let signature = self.signature(&program, at)?;
        Ok(program.signed(&signature))
    }

    /// The program signed, as of `at`, after the signatures it carries
    /// that aren't this signer's, which this one takes the place of.
    pub fn sign_beside(&self, program: Vec<u8>, at: SystemTime) -> Result<Vec<u8>, Fault> {
        let program = Program::parse(program)?;
        let signature = self.signature(&program, at)?;
        let certificate = &self.certificate;
        Ok(program.signed_beside(&signature, |entry| {
            !entry
                .windows(certificate.len())
                .any(|window| window == certificate.as_slice())
        }))
    }

    /// The PKCS #7 signature over the program, as of `at`.
    fn signature(&self, program: &Program, at: SystemTime) -> Result<Vec<u8>, Fault> {
        let digest = digest(program);

        let content = sequence(&[
            &sequence(&[
                &oid(SPC_PE_IMAGE_DATA),
                &sequence(&[
                    &[der::BIT_STRING, 1, 0],
                    &value(0xa0, &[&value(0xa2, &[&value(0x80, &[OBSOLETE])])]),
                ]),
            ]),
            &sequence(&[&algorithm(SHA256), &value(OCTET_STRING, &[&digest])]),
        ]);
        let (indirect, _) = der::read(&content, "the signature")?;

        let seconds = at
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Fault::unreadable("the clock", "it's set before 1970"))?
            .as_secs();
        let attributes = set_of(vec![
            attribute(CONTENT_TYPE, &oid(SPC_INDIRECT_DATA)),
            attribute(SIGNING_TIME, &der::time(seconds)),
            attribute(
                MESSAGE_DIGEST,
                &value(OCTET_STRING, &[&Sha256::digest(indirect.contents)]),
            ),
            attribute(SMIME_CAPABILITIES, &smime_capabilities()),
        ]);
        let digest_info = [SHA256_DIGEST_INFO, &Sha256::digest(&attributes)].concat();
        let signature = self
            .key
            .sign_with_rng(
                &mut rand_core::OsRng,
                Pkcs1v15Sign::new_unprefixed(),
                &digest_info,
            )
            .map_err(refused("the db key"))?;

        let signer = sequence(&[
            &small_integer(1),
            &self.issuer_and_serial,
            &algorithm(SHA256),
            // The same attributes, tagged [0] where they're kept.
            &[&[0xa0][..], &attributes[1..]].concat(),
            &algorithm(RSA_ENCRYPTION),
            &value(OCTET_STRING, &[&signature]),
        ]);
        let signed_data = sequence(&[
            &small_integer(1),
            &der::value(der::SET, &[&algorithm(SHA256)]),
            &sequence(&[&oid(SPC_INDIRECT_DATA), &value(0xa0, &[&content])]),
            &value(0xa0, &[&self.certificate]),
            &der::value(der::SET, &[&signer]),
        ]);
        Ok(sequence(&[
            &oid(SIGNED_DATA),
            &value(0xa0, &[&signed_data]),
        ]))
    }
}
