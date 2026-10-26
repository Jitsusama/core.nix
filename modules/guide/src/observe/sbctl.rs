//! The machine's own Secure Boot keys, as sbctl keeps them in
//! `/var/lib/sbctl`, and whether each boot file is signed with them.

use serde::Deserialize;

use crate::{Fault, Machine, Tool};

/// What `sbctl status --json` says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Status {
    /// Whether sbctl holds keys of its own.
    pub installed: bool,
    /// The owner GUID its keys were made under, when it holds any.
    #[serde(default)]
    pub guid: Option<String>,
    pub setup_mode: bool,
    pub secure_boot: bool,
    /// Vendors' keys the firmware's db holds beside the machine's, such as
    /// Microsoft's. A machine of core's holds none.
    #[serde(default, deserialize_with = "nothing_is_none")]
    pub vendors: Vec<String>,
}

/// sbctl writes `null` for a list it has nothing in.
fn nothing_is_none<'de, D: serde::Deserializer<'de>>(into: D) -> Result<Vec<String>, D::Error> {
    Ok(Option::<Vec<String>>::deserialize(into)?.unwrap_or_default())
}

impl Status {
    pub fn observe(machine: &mut dyn Machine) -> Result<Status, Fault> {
        Status::parse(&machine.run(Tool::Sbctl, &["status", "--json"])?.ok()?)
    }

    pub fn parse(json: &str) -> Result<Status, Fault> {
        serde_json::from_str(json).map_err(|why| Fault::unreadable("sbctl's status", why))
    }
}

/// One boot file and whether it's signed with the machine's db key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    pub file: String,
    pub signed: bool,
}

/// What `sbctl verify --json` says of every file on the boot partition it checks.
pub fn verify(machine: &mut dyn Machine) -> Result<Vec<Signed>, Fault> {
    parse_verified(&machine.run(Tool::Sbctl, &["verify", "--json"])?.ok()?)
}

pub fn parse_verified(json: &str) -> Result<Vec<Signed>, Fault> {
    #[derive(Deserialize)]
    struct File {
        file_name: String,
        is_signed: u8,
    }
    let files: Vec<File> =
        serde_json::from_str(json).map_err(|why| Fault::unreadable("sbctl's verify", why))?;
    files
        .into_iter()
        .map(|file| match file.is_signed {
            0 | 1 => Ok(Signed {
                signed: file.is_signed == 1,
                file: file.file_name,
            }),
            other => Err(Fault::unreadable(
                "sbctl's verify",
                format!("{} is signed {other}", file.file_name),
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stand_in::StandIn;

    #[test]
    fn the_surface_holds_its_own_keys_and_enforces() {
        let mut stand_in = StandIn::new();
        stand_in.prints(
            Tool::Sbctl,
            &["status", "--json"],
            surface!("sbctl-status.json"),
        );
        assert_eq!(
            Status::observe(&mut stand_in).unwrap(),
            Status {
                installed: true,
                guid: Some("22f0175f-c166-49d2-96ab-c61575cddac5".into()),
                setup_mode: false,
                secure_boot: true,
                vendors: vec![],
            }
        );
    }

    #[test]
    fn the_xps_holds_its_own_keys_and_enforces() {
        let status = Status::parse(xps!("sbctl-status.json")).unwrap();
        assert_eq!(
            status.guid.as_deref(),
            Some("9bc745a8-c143-45ee-8216-3e6d0f6bb7d2")
        );
        assert!(status.installed && status.secure_boot && !status.setup_mode);
    }

    #[test]
    fn a_machine_without_keys_has_no_guid() {
        let status = Status::parse(
            r#"{"installed":false,"setup_mode":true,"secure_boot":false,"vendors":null}"#,
        )
        .unwrap();
        assert_eq!(status.guid, None);
        assert_eq!(status.vendors, Vec::<String>::new());
    }

    #[test]
    fn the_surface_signs_every_boot_file_but_lanzabootes_kernels() {
        let files = parse_verified(surface!("sbctl-verify.json")).unwrap();
        assert_eq!(files.len(), 8);
        let unsigned: Vec<&str> = files
            .iter()
            .filter(|file| !file.signed)
            .map(|file| file.file.as_str())
            .collect();
        assert_eq!(unsigned.len(), 2);
        assert!(
            unsigned
                .iter()
                .all(|file| file.starts_with("/boot/EFI/nixos/kernel-")),
            "{unsigned:?}"
        );
    }

    #[test]
    fn sbctl_refusing_is_a_fault_not_a_status() {
        let mut stand_in = StandIn::new();
        stand_in.refuses(
            Tool::Sbctl,
            &["status", "--json"],
            1,
            "sbctl requires root to run\n",
        );
        assert!(matches!(
            Status::observe(&mut stand_in),
            Err(Fault::Refused { said, .. }) if said == "sbctl requires root to run"
        ));
    }
}
