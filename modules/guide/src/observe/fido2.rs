//! The FIDO2 keys plugged in, as `fido2-token -L` lists them. Telling one
//! YubiKey from another needs a credential check rather than this list,
//! since both say the same; this only says what's there and where.

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
    fn no_keys_is_an_empty_list() {
        assert_eq!(parse("").unwrap(), []);
    }

    #[test]
    fn a_line_in_another_shape_is_refused() {
        assert!(parse("/dev/hidraw3 vendor 1050\n").is_err());
    }
}
