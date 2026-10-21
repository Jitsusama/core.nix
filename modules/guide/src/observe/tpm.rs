//! The TPM's dictionary-attack lockout, from `tpm2_getcap`: how many wrong
//! PINs it has counted, how many it allows, and whether it's locked out.
//! A firmware flash without its unlock code once pushed an XPS 14's count to
//! 28 of 32, so the guide reads this before asking for any TPM PIN.
//!
//! tpm2-tools tries the access broker over D-Bus before the kernel's
//! resource manager and says so on stderr, so every call names the device.

use std::collections::BTreeMap;

use crate::{Fault, Machine, Tool};

const DEVICE: &[&str] = &["-T", "device:/dev/tpmrm0"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lockout {
    /// Wrong authorisations counted so far.
    pub failures: u32,
    /// How many it takes to lock the TPM out.
    pub allowed: u32,
    pub locked: bool,
    /// Whether the lockout hierarchy has a password of its own, which
    /// clearing the TPM from the system then needs.
    pub guarded: bool,
    /// Seconds before one counted failure is forgiven.
    pub forgives_after: u32,
    /// Seconds a lockout lasts once it's reached.
    pub locked_for: u32,
}

impl Lockout {
    pub fn observe(machine: &mut dyn Machine) -> Result<Lockout, Fault> {
        let args = [DEVICE, &["properties-variable"]].concat();
        Lockout::parse(&machine.run(Tool::Tpm2Getcap, &args)?.ok()?)
    }

    pub fn parse(text: &str) -> Result<Lockout, Fault> {
        let properties = Properties::parse(text);
        Ok(Lockout {
            failures: properties.number("TPM2_PT_LOCKOUT_COUNTER")?,
            allowed: properties.number("TPM2_PT_MAX_AUTH_FAIL")?,
            locked: properties.flag("TPM2_PT_PERMANENT.inLockout")?,
            guarded: properties.flag("TPM2_PT_PERMANENT.lockoutAuthSet")?,
            forgives_after: properties.number("TPM2_PT_LOCKOUT_INTERVAL")?,
            locked_for: properties.number("TPM2_PT_LOCKOUT_RECOVERY")?,
        })
    }

    /// Wrong PINs left before the TPM locks.
    pub fn left(&self) -> u32 {
        self.allowed.saturating_sub(self.failures)
    }
}

/// The NV indices the TPM holds, from `tpm2_getcap handles-nv-index`, one
/// `- 0x...` line each. Clearing the TPM takes every index an owner made,
/// systemd-pcrlock's policy among them.
pub fn nv_indices(text: &str) -> Result<Vec<u32>, Fault> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.strip_prefix("- 0x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .ok_or_else(|| {
                    Fault::unreadable("the TPM's NV indices", format!("a line reads {line:?}"))
                })
        })
        .collect()
}

/// The arguments that list the TPM's NV indices.
pub const NV_INDICES: &[&str] = &["-T", "device:/dev/tpmrm0", "handles-nv-index"];

/// What the TPM says of itself that never changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixed {
    /// The maker's code of up to four letters, such as `NTC` for Nuvoton.
    pub manufacturer: String,
}

impl Fixed {
    pub fn observe(machine: &mut dyn Machine) -> Result<Fixed, Fault> {
        let args = [DEVICE, &["properties-fixed"]].concat();
        Fixed::parse(&machine.run(Tool::Tpm2Getcap, &args)?.ok()?)
    }

    pub fn parse(text: &str) -> Result<Fixed, Fault> {
        let properties = Properties::parse(text);
        Ok(Fixed {
            manufacturer: properties
                .text("TPM2_PT_MANUFACTURER.value")?
                .trim_matches('"')
                .to_string(),
        })
    }
}

/// tpm2_getcap's YAML-shaped listing, flattened: a property's own value by
/// its name, and each field indented under it as `PROPERTY.field`.
struct Properties(BTreeMap<String, String>);

impl Properties {
    fn parse(text: &str) -> Properties {
        let mut properties = BTreeMap::new();
        let mut heading = String::new();
        for line in text.lines() {
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().to_string();
            if line.starts_with(char::is_whitespace) {
                properties.insert(format!("{heading}.{}", name.trim()), value);
            } else {
                heading = name.to_string();
                if !value.is_empty() {
                    properties.insert(heading.clone(), value);
                }
            }
        }
        Properties(properties)
    }

    fn text(&self, name: &str) -> Result<&str, Fault> {
        self.0
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| Fault::unreadable("the TPM's properties", format!("no {name}")))
    }

    fn number(&self, name: &str) -> Result<u32, Fault> {
        let value = self.text(name)?;
        let parsed = match value.strip_prefix("0x") {
            Some(hex) => u32::from_str_radix(hex, 16),
            None => value.parse(),
        };
        parsed.map_err(|_| Fault::unreadable("the TPM's properties", format!("{name} is {value}")))
    }

    fn flag(&self, name: &str) -> Result<bool, Fault> {
        match self.number(name)? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(Fault::unreadable(
                "the TPM's properties",
                format!("{name} is {other}"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::Fake;

    #[test]
    fn the_surfaces_tpm_has_counted_nothing_and_guards_its_lockout() {
        let mut fake = Fake::new();
        fake.prints(
            Tool::Tpm2Getcap,
            &["-T", "device:/dev/tpmrm0", "properties-variable"],
            surface!("tpm2-properties-variable.txt"),
        );
        let lockout = Lockout::observe(&mut fake).unwrap();
        assert_eq!(
            lockout,
            Lockout {
                failures: 0,
                allowed: 32,
                locked: false,
                guarded: true,
                forgives_after: 7200,
                locked_for: 86400,
            }
        );
        assert_eq!(lockout.left(), 32);
    }

    #[test]
    fn the_xpss_tpm_leaves_its_lockout_unguarded() {
        let lockout = Lockout::parse(xps!("tpm2-properties-variable.txt")).unwrap();
        assert!(!lockout.guarded && !lockout.locked);
        assert_eq!(lockout.left(), 32);
    }

    #[test]
    fn the_surfaces_tpm_is_nuvotons() {
        let fixed = Fixed::parse(surface!("tpm2-properties-fixed.txt")).unwrap();
        assert_eq!(
            fixed,
            Fixed {
                manufacturer: "NTC".into(),
            }
        );
    }

    #[test]
    fn a_counted_failure_is_read() {
        let text = surface!("tpm2-properties-variable.txt").replace(
            "TPM2_PT_LOCKOUT_COUNTER: 0x0",
            "TPM2_PT_LOCKOUT_COUNTER: 0x1C",
        );
        assert_eq!(Lockout::parse(&text).unwrap().left(), 4);
    }

    #[test]
    fn a_listing_without_the_counter_is_refused() {
        let text = surface!("tpm2-properties-variable.txt")
            .replace("TPM2_PT_LOCKOUT_COUNTER", "TPM2_PT_SOMETHING_ELSE");
        assert!(matches!(
            Lockout::parse(&text),
            Err(Fault::Unreadable { why, .. }) if why == "no TPM2_PT_LOCKOUT_COUNTER"
        ));
    }
}
