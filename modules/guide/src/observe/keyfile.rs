//! A key kept in the TPM's name, read from the file ssh-tpm-keygen leaves
//! beside its public half: the TCG's TSS2 private key format, a PEM
//! `TSS2 PRIVATE KEY` holding one ASN.1 sequence (go-tpm-keyfiles'
//! der.go). The guide reads only what loading it into the TPM takes: its
//! kind, its parent and its two halves, as the TPM gave them.
//!
//! tpm2-tools 5.8 reads the same files, but not one with a description,
//! which ssh-tpm-keygen writes its comment into, so the guide takes the
//! halves out for it.

use base64::Engine;

use crate::Fault;

/// The object identifier of a key the TPM loads under its parent,
/// 2.23.133.10.1.3, as DER encodes it. The format's other kinds, a key to
/// import and a sealed secret, aren't what ssh-tpm-keygen makes.
const LOADABLE: &[u8] = &[0x67, 0x81, 0x05, 0x0a, 0x01, 0x03];

/// The parent a key names when it was made under the storage key the
/// owner's seed gives, as ssh-tpm-keygen's are: `TPM_RH_OWNER`. A TPM
/// cleared since has another seed, and so refuses the key.
pub const OWNER: u32 = 0x4000_0001;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFile {
    /// Whether it's a key the TPM loads, rather than one to import.
    pub loadable: bool,
    /// The handle of what it was made under.
    pub parent: u32,
    /// Its public area, a marshalled `TPM2B_PUBLIC`.
    pub public: Vec<u8>,
    /// Its private area, sealed to its parent, a marshalled `TPM2B_PRIVATE`.
    pub private: Vec<u8>,
}

impl KeyFile {
    pub fn parse(pem: &[u8]) -> Result<KeyFile, Fault> {
        let unreadable = |why: &str| Fault::unreadable("a TPM key's file", why);
        let text = String::from_utf8_lossy(pem);
        let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
        if lines.next() != Some("-----BEGIN TSS2 PRIVATE KEY-----") {
            return Err(unreadable("it doesn't begin as a TSS2 private key"));
        }
        let body: String = lines
            .take_while(|line| *line != "-----END TSS2 PRIVATE KEY-----")
            .collect();
        let der = base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|_| unreadable("its body isn't base64"))?;
        let (tag, sequence, rest) = element(&der).ok_or_else(|| unreadable("it ends early"))?;
        if tag != SEQUENCE || !rest.is_empty() {
            return Err(unreadable("it isn't one ASN.1 sequence"));
        }
        let (tag, kind, mut rest) = element(sequence).ok_or_else(|| unreadable("it ends early"))?;
        if tag != OBJECT_IDENTIFIER {
            return Err(unreadable("it doesn't open with its kind"));
        }
        // emptyAuth, policy, secret, authPolicy and description, each
        // optional and tagged [0] to [4], come before its parent.
        let parent = loop {
            let (tag, value, after) = element(rest).ok_or_else(|| unreadable("it ends early"))?;
            rest = after;
            match tag {
                OPTIONAL_FIRST..=OPTIONAL_LAST => {}
                INTEGER => break value,
                _ => return Err(unreadable("its parent isn't where the format puts it")),
            }
        };
        let (tag, public, rest) = element(rest).ok_or_else(|| unreadable("it ends early"))?;
        if tag != OCTET_STRING {
            return Err(unreadable("its public half isn't where the format puts it"));
        }
        let (tag, private, _) = element(rest).ok_or_else(|| unreadable("it ends early"))?;
        if tag != OCTET_STRING {
            return Err(unreadable(
                "its private half isn't where the format puts it",
            ));
        }
        Ok(KeyFile {
            loadable: kind == LOADABLE,
            parent: handle(parent).ok_or_else(|| unreadable("its parent isn't a handle"))?,
            public: public.to_vec(),
            private: private.to_vec(),
        })
    }
}

const SEQUENCE: u8 = 0x30;
const OBJECT_IDENTIFIER: u8 = 0x06;
const INTEGER: u8 = 0x02;
const OCTET_STRING: u8 = 0x04;
/// The explicitly tagged optional fields, [0] to [4], as constructed
/// context-specific tags.
const OPTIONAL_FIRST: u8 = 0xa0;
const OPTIONAL_LAST: u8 = 0xa4;

/// The first DER element: its tag, its value and what follows it, or
/// nothing when the bytes end before it does.
fn element(bytes: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = bytes.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (length, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let (digits, rest) = rest.split_at(count);
        let length = digits
            .iter()
            .fold(0usize, |length, &digit| (length << 8) | usize::from(digit));
        (length, rest)
    };
    if rest.len() < length {
        return None;
    }
    let (value, rest) = rest.split_at(length);
    Some((tag, value, rest))
}

/// A DER integer as a TPM handle: positive, and four bytes once the
/// leading zero a set top bit needs is gone.
fn handle(integer: &[u8]) -> Option<u32> {
    let digits = match integer {
        [0, rest @ ..] if !rest.is_empty() => rest,
        [first, ..] if first & 0x80 != 0 => return None,
        digits => digits,
    };
    if digits.is_empty() || digits.len() > 4 {
        return None;
    }
    Some(
        digits
            .iter()
            .fold(0u32, |handle, &digit| (handle << 8) | u32::from(digit)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/tpm-simulator/ssh-tpm-keygen.tpm"
    ));

    #[test]
    fn ssh_tpm_keygens_key_is_loadable_under_the_owners_storage_key() {
        let key = KeyFile::parse(KEY.as_bytes()).unwrap();
        assert!(key.loadable);
        assert_eq!(key.parent, OWNER);
        // Each half as the TPM marshals it: its size, then that many bytes.
        for half in [&key.public, &key.private] {
            let size = usize::from(u16::from_be_bytes([half[0], half[1]]));
            assert_eq!(size + 2, half.len());
        }
        assert_eq!(key.public.len(), 88);
        assert_eq!(key.private.len(), 128);
    }

    #[test]
    fn a_file_that_isnt_a_tpm_key_is_unreadable() {
        for file in [
            "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----\n",
            "-----BEGIN TSS2 PRIVATE KEY-----\nnot base64!\n-----END TSS2 PRIVATE KEY-----\n",
            "-----BEGIN TSS2 PRIVATE KEY-----\nMAA=\n-----END TSS2 PRIVATE KEY-----\n",
        ] {
            assert!(
                matches!(
                    KeyFile::parse(file.as_bytes()),
                    Err(Fault::Unreadable { .. })
                ),
                "{file}"
            );
        }
    }

    #[test]
    fn a_key_cut_short_is_unreadable() {
        let cut: String = KEY.lines().take(3).collect::<Vec<_>>().join("\n");
        assert!(matches!(
            KeyFile::parse(cut.as_bytes()),
            Err(Fault::Unreadable { .. })
        ));
    }

    #[test]
    fn a_handle_reads_with_or_without_the_zero_a_top_bit_needs() {
        assert_eq!(handle(&[0x40, 0, 0, 1]), Some(OWNER));
        assert_eq!(handle(&[0, 0x81, 0, 0, 1]), Some(0x8100_0001));
        assert_eq!(handle(&[0x81, 0, 0, 1]), None);
        assert_eq!(handle(&[1, 2, 3, 4, 5]), None);
        assert_eq!(handle(&[]), None);
    }
}
