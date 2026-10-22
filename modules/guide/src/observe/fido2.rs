//! The FIDO2 keys plugged in, as `fido2-token -L` lists them. Telling one
//! YubiKey from another needs a credential check rather than this list,
//! since both say the same; this only says what's there and where, and
//! which of them could be one of the person's keys.

use crate::{Fault, Machine, Tool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Its hidraw device, which every other fido2-token call names.
    pub path: String,
    pub vendor: u16,
    pub product: u16,
    /// Its maker's name and its own, as it gives them; either may be empty.
    pub name: String,
}

/// Yubico's USB vendor id.
pub const YUBICO: u16 = 0x1050;

pub fn list(machine: &mut dyn Machine) -> Result<Vec<Token>, Fault> {
    parse(&machine.run(Tool::Fido2Token, &["-L"])?.ok()?)
}

/// The keys plugged in that could be one of the person's: everything the
/// guide seals to a key (the disk's FIDO2 slot, the vault, a rescue
/// bundle) is a secret the key derives with hmac-secret behind its PIN.
/// A platform authenticator a laptop lists with no key plugged in, such
/// as linux-id, does neither, so it never counts as a key plugged in.
pub fn plugged(machine: &mut dyn Machine) -> Result<Vec<Token>, Fault> {
    let mut plugged = vec![];
    for token in list(machine)? {
        let info = machine.run(Tool::Fido2Token, &["-I", &token.path])?.ok()?;
        if seals(&info) {
            plugged.push(token);
        }
    }
    Ok(plugged)
}

/// Whether `fido2-token -I` says the key derives secrets with hmac-secret
/// and has a PIN, set or not: libfido2 prints an option it lacks not at
/// all, one it has as its name, and one turned off with "no" before it
/// (tools/token.c).
fn seals(info: &str) -> bool {
    let listed = |label: &str| {
        info.lines()
            .find_map(|line| line.strip_prefix(label))
            .map(|rest| rest.split(',').map(str::trim).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    let options = listed("options:");
    listed("extension strings:").contains(&"hmac-secret")
        && (options.contains(&"clientPin") || options.contains(&"noclientPin"))
}

/// A key's refusal, once what was typed was tried as `passphrase` first
/// and wasn't it either, so the person reads that both were tried.
pub fn neither(fault: Fault, passphrase: &str) -> Fault {
    match fault {
        Fault::Unopened { what, why } => Fault::Unopened {
            what,
            why: format!("that isn't {passphrase}, and {why}"),
        },
        other => other,
    }
}

/// What age's FIDO2 plugin said when a key refused, in plain words, for
/// the refusals a person can do something about.
pub fn refusal(said: &str) -> Option<&'static str> {
    if said.contains("pin invalid") {
        Some("it isn't the PIN of the key plugged in")
    } else if said.contains("pin auth blocked") {
        Some("the key plugged in refuses its PIN until it's unplugged and plugged back in")
    } else if said.contains("no credentials") || said.contains("No matching keys found") {
        Some("the key plugged in isn't one it was sealed to")
    } else {
        None
    }
}

/// libfido2 prints each as `path: vendor=0x%04x, product=0x%04x (%s %s)`
/// (tools/token.c), maker then product.
pub fn parse(text: &str) -> Result<Vec<Token>, Fault> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let unreadable = || Fault::unreadable("a FIDO2 key fido2-token listed", line);
            let (path, rest) = line.split_once(": vendor=0x").ok_or_else(unreadable)?;
            let (vendor, rest) = rest.split_once(", product=0x").ok_or_else(unreadable)?;
            let (product, rest) = rest.split_once(" (").ok_or_else(unreadable)?;
            let name = rest.strip_suffix(')').ok_or_else(unreadable)?;
            let hex = |digits: &str| u16::from_str_radix(digits, 16).map_err(|_| unreadable());
            Ok(Token {
                path: path.to_string(),
                vendor: hex(vendor)?,
                product: hex(product)?,
                name: name.trim().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_xps_lists_only_its_own_virtual_key_with_no_yubikey_in() {
        let tokens = parse(xps!("fido2-token-L.txt")).unwrap();
        assert_eq!(
            tokens,
            [Token {
                path: "/dev/hidraw6".into(),
                vendor: 0x15d9,
                product: 0x0a37,
                name: "linux-id".into(),
            }]
        );
        assert!(tokens.iter().all(|token| token.vendor != YUBICO));
    }

    #[test]
    fn a_yubikey_is_known_by_its_vendor() {
        let tokens =
            parse("/dev/hidraw3: vendor=0x1050, product=0x0407 (Yubico YubiKey OTP+FIDO+CCID)\n")
                .unwrap();
        assert_eq!(tokens[0].vendor, YUBICO);
        assert_eq!(tokens[0].name, "Yubico YubiKey OTP+FIDO+CCID");
    }

    #[test]
    fn a_key_with_hmac_secret_and_a_pin_can_be_the_persons() {
        assert!(seals(qemu!("fido2-token-I.txt")));
        let unset = qemu!("fido2-token-I.txt").replace(" clientPin,", " noclientPin,");
        assert!(seals(&unset), "a key whose PIN isn't set yet");
        let no_hmac = qemu!("fido2-token-I.txt").replace(" hmac-secret,", "");
        assert!(!seals(&no_hmac));
    }

    #[test]
    fn the_xps_virtual_key_can_never_be_the_persons() {
        assert!(!seals(xps!("fido2-token-I.txt")));
    }

    #[test]
    fn the_plugins_pin_refusals_are_said_plainly() {
        let said = "(stanza 0 0) failed to get assertion: pin invalid";
        assert_eq!(
            refusal(said),
            Some("it isn't the PIN of the key plugged in")
        );
        // What age itself says when no stanza is the plugged-in key's,
        // as the vault said it on a stick sealed to other keys.
        assert_eq!(
            refusal("No matching keys found"),
            Some("the key plugged in isn't one it was sealed to")
        );
        assert_eq!(refusal("failed to open the file"), None);
    }

    #[test]
    fn no_keys_is_an_empty_list() {
        assert_eq!(parse("").unwrap(), []);
    }

    #[test]
    fn a_line_in_another_shape_is_refused() {
        assert!(parse("/dev/hidraw3 vendor 1050\n").is_err());
    }
}
