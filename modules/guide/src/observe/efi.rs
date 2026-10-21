//! Secure Boot as the firmware itself says it, from its global variables,
//! which every UEFI machine keeps and anyone can read: whether it enforces,
//! and which of UEFI's modes it's in. A firmware older than UEFI 2.5 keeps
//! no AuditMode or DeployedMode, as the Surface Laptop 4's doesn't.

use std::fmt;

use crate::{Fault, Machine};

/// The vendor GUID UEFI keeps its global variables under.
const GLOBAL: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";
/// Where the firmware keeps db, the keys it trusts boot files from.
const IMAGE_SECURITY: &str = "d719b2cb-3d3a-4596-a3bc-dad00e67656f";
/// EFI_CERT_X509_GUID, as it's laid out in a signature list.
const X509: [u8; 16] = [
    0xa1, 0x59, 0xc0, 0xa5, 0xe4, 0x94, 0xa7, 0x4a, 0x87, 0xb5, 0xab, 0x15, 0x5c, 0x2b, 0xf0, 0x72,
];

/// The X.509 certificates the firmware's db trusts, each as DER, or none
/// when it keeps no db, as in setup mode.
pub fn db_certificates(machine: &mut dyn Machine) -> Result<Vec<Vec<u8>>, Fault> {
    match machine.read(&format!("/sys/firmware/efi/efivars/db-{IMAGE_SECURITY}"))? {
        Some(bytes) => certificates(&bytes),
        None => Ok(vec![]),
    }
}

/// The X.509 certificates in a signature database variable as efivarfs
/// gives it: four bytes of attributes, then EFI_SIGNATURE_LISTs, each a
/// type, its sizes, a header, and signatures that are an owner GUID and
/// the data (UEFI 2.10, section 32.4.1).
pub fn certificates(variable: &[u8]) -> Result<Vec<Vec<u8>>, Fault> {
    let unreadable = |why: &str| Fault::unreadable("the firmware's db", why);
    let u32_at = |bytes: &[u8], at: usize| -> Result<usize, Fault> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")) as usize)
            .ok_or_else(|| unreadable("it ends inside a signature list's header"))
    };
    let mut found = vec![];
    let mut rest = variable
        .get(4..)
        .ok_or_else(|| unreadable("it has no attributes"))?;
    while !rest.is_empty() {
        let list = u32_at(rest, 16)?;
        let header = u32_at(rest, 20)?;
        let size = u32_at(rest, 24)?;
        if list < 28 + header || list > rest.len() || size <= 16 {
            return Err(unreadable("a signature list's sizes don't fit it"));
        }
        if rest[..16] == X509 {
            let signatures = &rest[28 + header..list];
            if !signatures.len().is_multiple_of(size) {
                return Err(unreadable("a signature list doesn't hold whole signatures"));
            }
            found.extend(
                signatures
                    .chunks(size)
                    .map(|signature| signature[16..].to_vec()),
            );
        }
        rest = &rest[list..];
    }
    Ok(found)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecureBoot {
    /// Whether the firmware checks every boot file's signature.
    pub enforcing: bool,
    pub mode: Mode,
}

/// UEFI's Secure Boot modes (UEFI 2.10, section 32.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// No platform key: anyone may enrol keys, which is how an install
    /// enrols the machine's own.
    Setup,
    /// Setup mode that measures and logs rather than refusing.
    Audit,
    /// A platform key is enrolled, and keys change only through it.
    User,
    /// User mode the firmware's own setup can't leave without the key.
    Deployed,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Mode::Setup => "setup",
            Mode::Audit => "audit",
            Mode::User => "user",
            Mode::Deployed => "deployed",
        })
    }
}

impl SecureBoot {
    /// What the firmware says, or nothing on a machine that didn't boot
    /// through UEFI or keeps no Secure Boot.
    pub fn observe(machine: &mut dyn Machine) -> Result<Option<SecureBoot>, Fault> {
        let mut read =
            |name: &str| machine.read(&format!("/sys/firmware/efi/efivars/{name}-{GLOBAL}"));
        let secure_boot = read("SecureBoot")?;
        let setup_mode = read("SetupMode")?;
        let audit_mode = read("AuditMode")?;
        let deployed_mode = read("DeployedMode")?;
        SecureBoot::from_variables(
            secure_boot.as_deref(),
            setup_mode.as_deref(),
            audit_mode.as_deref(),
            deployed_mode.as_deref(),
        )
    }

    /// From the variables' contents, as efivarfs gives them: four bytes of
    /// attributes, then the value.
    pub fn from_variables(
        secure_boot: Option<&[u8]>,
        setup_mode: Option<&[u8]>,
        audit_mode: Option<&[u8]>,
        deployed_mode: Option<&[u8]>,
    ) -> Result<Option<SecureBoot>, Fault> {
        let (Some(secure_boot), Some(setup_mode)) = (secure_boot, setup_mode) else {
            return Ok(None);
        };
        let flag = |name: &str, bytes: Option<&[u8]>| -> Result<bool, Fault> {
            match bytes {
                None => Ok(false),
                Some([_, _, _, _, 0]) => Ok(false),
                Some([_, _, _, _, 1]) => Ok(true),
                Some(other) => Err(Fault::unreadable(
                    format!("the firmware's {name}"),
                    format!("{other:?} isn't a flag"),
                )),
            }
        };
        let enforcing = flag("SecureBoot", Some(secure_boot))?;
        let mode = match (
            flag("SetupMode", Some(setup_mode))?,
            flag("AuditMode", audit_mode)?,
            flag("DeployedMode", deployed_mode)?,
        ) {
            (true, true, _) => Mode::Audit,
            (true, false, _) => Mode::Setup,
            (false, _, true) => Mode::Deployed,
            (false, _, false) => Mode::User,
        };
        Ok(Some(SecureBoot { enforcing, mode }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::Fake;
    use crate::fixtures;

    fn path(name: &str) -> String {
        format!("/sys/firmware/efi/efivars/{name}-{GLOBAL}")
    }

    #[test]
    fn the_xps_enforces_in_deployed_mode() {
        let mut fake = Fake::new();
        for name in ["SecureBoot", "SetupMode", "AuditMode", "DeployedMode"] {
            let bytes = std::fs::read(format!("{}/efivar-{name}-{GLOBAL}", fixtures::XPS)).unwrap();
            fake.file(&path(name), &bytes);
        }
        assert_eq!(
            SecureBoot::observe(&mut fake).unwrap(),
            Some(SecureBoot {
                enforcing: true,
                mode: Mode::Deployed
            })
        );
    }

    #[test]
    fn the_surface_enforces_in_user_mode_with_no_later_variables() {
        let mut fake = Fake::new();
        for name in ["SecureBoot", "SetupMode"] {
            let bytes =
                std::fs::read(format!("{}/efivar-{name}-{GLOBAL}", fixtures::SURFACE)).unwrap();
            fake.file(&path(name), &bytes);
        }
        fake.no_file(&path("AuditMode"))
            .no_file(&path("DeployedMode"));
        assert_eq!(
            SecureBoot::observe(&mut fake).unwrap(),
            Some(SecureBoot {
                enforcing: true,
                mode: Mode::User
            })
        );
    }

    #[test]
    fn setup_mode_is_read_as_setup() {
        let off = [7, 0, 0, 0, 0];
        let on = [7, 0, 0, 0, 1];
        assert_eq!(
            SecureBoot::from_variables(Some(&off), Some(&on), Some(&off), Some(&off)).unwrap(),
            Some(SecureBoot {
                enforcing: false,
                mode: Mode::Setup
            })
        );
        assert_eq!(
            SecureBoot::from_variables(Some(&off), Some(&on), Some(&on), None).unwrap(),
            Some(SecureBoot {
                enforcing: false,
                mode: Mode::Audit
            })
        );
    }

    #[test]
    fn a_machine_without_the_variables_has_no_secure_boot() {
        let mut fake = Fake::new();
        for name in ["SecureBoot", "SetupMode", "AuditMode", "DeployedMode"] {
            fake.no_file(&path(name));
        }
        assert_eq!(SecureBoot::observe(&mut fake).unwrap(), None);
    }

    #[test]
    fn a_variable_that_isnt_a_flag_is_refused() {
        let on = [7, 0, 0, 0, 1];
        let Err(Fault::Unreadable { what, .. }) =
            SecureBoot::from_variables(Some(&[7, 0, 0, 0, 2]), Some(&on), None, None)
        else {
            panic!("refused")
        };
        assert_eq!(what, "the firmware's SecureBoot");
    }

    #[test]
    fn db_holds_the_certificate_sbctl_made_for_the_machine() {
        let db =
            std::fs::read(format!("{}/efivar-db-{IMAGE_SECURITY}", fixtures::SURFACE)).unwrap();
        let pem = std::fs::read_to_string(format!("{}/sbctl-db.pem", fixtures::SURFACE)).unwrap();
        let (_, der) = rsa::pkcs8::der::pem::decode_vec(pem.as_bytes()).unwrap();
        assert_eq!(certificates(&db).unwrap(), [der]);
        // The XPS 14 holds one of its own, and not the Surface's.
        let xps = std::fs::read(format!("{}/efivar-db-{IMAGE_SECURITY}", fixtures::XPS)).unwrap();
        let theirs = certificates(&xps).unwrap();
        assert_eq!(theirs.len(), 1);
        assert_ne!(theirs, certificates(&db).unwrap());
    }

    #[test]
    fn a_signature_list_that_overruns_is_refused() {
        let mut db =
            std::fs::read(format!("{}/efivar-db-{IMAGE_SECURITY}", fixtures::SURFACE)).unwrap();
        db.truncate(db.len() - 1);
        assert!(certificates(&db).is_err());
    }
}
