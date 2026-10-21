//! A LUKS2 header as `cryptsetup luksDump --dump-json-metadata` prints it:
//! its keyslots, and the tokens that say what opens each one.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{Fault, Machine, Tool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub keyslots: BTreeSet<u32>,
    /// The keyslots tried first, set with `cryptsetup config --priority
    /// prefer`.
    pub preferred: BTreeSet<u32>,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub id: u32,
    pub kind: Kind,
    pub keyslots: BTreeSet<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A FIDO2 key, by the credential it made, as base64.
    Fido2 {
        credential: String,
    },
    Tpm2,
    Recovery,
    Other(String),
}

impl Header {
    /// The FIDO2 credentials that open it.
    pub fn credentials(&self) -> Vec<&str> {
        self.tokens
            .iter()
            .filter_map(|token| match &token.kind {
                Kind::Fido2 { credential } => Some(credential.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn has(&self, kind: &Kind) -> bool {
        self.tokens.iter().any(|token| &token.kind == kind)
    }

    /// Keyslots no token speaks for: a passphrase's, or a key file's.
    pub fn bare(&self) -> BTreeSet<u32> {
        let spoken: BTreeSet<u32> = self
            .tokens
            .iter()
            .flat_map(|token| token.keyslots.iter().copied())
            .collect();
        self.keyslots.difference(&spoken).copied().collect()
    }
}

fn number(text: &str) -> Result<u32, Fault> {
    text.parse().map_err(|_| {
        Fault::unreadable("a LUKS header", format!("\"{text}\" isn't a slot's number"))
    })
}

pub fn parse(json: &str) -> Result<Header, Fault> {
    let unreadable = |why: &str| Fault::unreadable("a LUKS header", why);
    let root: Value =
        serde_json::from_str(json).map_err(|why| Fault::unreadable("a LUKS header", why))?;
    let slots = root["keyslots"]
        .as_object()
        .ok_or_else(|| unreadable("it has no keyslots"))?;
    let keyslots = slots
        .keys()
        .map(|slot| number(slot))
        .collect::<Result<_, _>>()?;
    // LUKS2 writes a keyslot's priority only when it isn't normal: 0 to
    // ignore it, 2 to prefer it.
    let preferred = slots
        .iter()
        .filter(|(_, slot)| slot["priority"].as_u64() == Some(2))
        .map(|(id, _)| number(id))
        .collect::<Result<_, _>>()?;
    let mut tokens = vec![];
    if let Some(found) = root["tokens"].as_object() {
        for (id, token) in found {
            let kind = match token["type"].as_str().unwrap_or_default() {
                "systemd-fido2" => Kind::Fido2 {
                    credential: token["fido2-credential"]
                        .as_str()
                        .ok_or_else(|| unreadable("a FIDO2 token has no credential"))?
                        .into(),
                },
                "systemd-tpm2" => Kind::Tpm2,
                "systemd-recovery" => Kind::Recovery,
                other => Kind::Other(other.into()),
            };
            let keyslots = token["keyslots"]
                .as_array()
                .ok_or_else(|| unreadable("a token names no keyslots"))?
                .iter()
                .map(|slot| {
                    slot.as_str()
                        .ok_or_else(|| unreadable("a keyslot isn't named by a string"))
                        .and_then(number)
                })
                .collect::<Result<_, _>>()?;
            tokens.push(Token {
                id: number(id)?,
                kind,
                keyslots,
            });
        }
    }
    tokens.sort_by_key(|token| token.id);
    Ok(Header {
        keyslots,
        preferred,
        tokens,
    })
}

pub fn observe(machine: &mut dyn Machine, device: &str) -> Result<Header, Fault> {
    parse(
        &machine
            .run(
                Tool::Cryptsetup,
                &["luksDump", "--dump-json-metadata", device],
            )?
            .ok()?,
    )
}

/// The header as root reads it, on an installed machine whose disk a
/// person can't read.
pub fn observe_as_root(machine: &mut dyn Machine, device: &str) -> Result<Header, Fault> {
    let ran = machine.run_given(
        crate::Program::Tool(Tool::Cryptsetup),
        &["luksDump", "--dump-json-metadata", device],
        crate::Given {
            as_root: true,
            ..crate::Given::default()
        },
    )?;
    parse(&ran.ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_files_slot_is_bare_beside_the_tokens() {
        let header = parse(qemu!("luks-dump-before-wipe.json")).unwrap();
        assert_eq!(header.keyslots, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(header.bare(), BTreeSet::from([0]));
        assert_eq!(header.credentials().len(), 2);
        assert_ne!(header.credentials()[0], header.credentials()[1]);
        assert!(header.has(&Kind::Recovery));
    }

    #[test]
    fn once_it_is_wiped_every_slot_has_its_token() {
        let header = parse(qemu!("luks-dump-installed.json")).unwrap();
        assert!(header.bare().is_empty(), "{header:?}");
        assert_eq!(header.keyslots.len(), 3);
        assert_eq!(
            header
                .tokens
                .iter()
                .map(|token| token.keyslots.clone())
                .collect::<Vec<_>>(),
            [
                BTreeSet::from([1]),
                BTreeSet::from([2]),
                BTreeSet::from([3])
            ]
        );
    }

    #[test]
    fn what_isnt_a_header_is_refused() {
        assert!(parse("{}").is_err());
        assert!(parse(r#"{"keyslots": {"x": {}}}"#).is_err());
    }
}
