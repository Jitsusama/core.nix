//! The boot loader's entries, as `bootctl list --json=short` gives them,
//! and the few lines of `bootctl status` that nothing else says, such as
//! whether the running system booted from a measured UKI.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{Fault, Machine, Tool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    /// The NixOS generation it boots, when it's one of NixOS's.
    pub generation: Option<u32>,
    /// What systemd-boot boots when nobody chooses.
    pub default: bool,
    /// What the running system booted from.
    pub booted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A unified kernel image, which lanzaboote writes and signs.
    Uki,
    /// A loader entry file naming a kernel and an initrd apart.
    Conf,
    /// One systemd-boot makes itself, such as rebooting to the firmware.
    Automatic,
    Other,
}

pub fn list(machine: &mut dyn Machine) -> Result<Vec<Entry>, Fault> {
    parse_list(
        &machine
            .run(Tool::Bootctl, &["list", "--json=short"])?
            .ok()?,
    )
}

pub fn parse_list(json: &str) -> Result<Vec<Entry>, Fault> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Listed {
        id: String,
        #[serde(rename = "type")]
        kind: String,
        title: String,
        is_default: bool,
        is_selected: bool,
    }
    let listed: Vec<Listed> =
        serde_json::from_str(json).map_err(|why| Fault::unreadable("bootctl's entries", why))?;
    Ok(listed
        .into_iter()
        .map(|entry| Entry {
            generation: generation(&entry.id),
            kind: match entry.kind.as_str() {
                "type2" => Kind::Uki,
                "type1" => Kind::Conf,
                "auto" => Kind::Automatic,
                _ => Kind::Other,
            },
            id: entry.id,
            title: entry.title,
            default: entry.is_default,
            booted: entry.is_selected,
        })
        .collect())
}

/// The generation in an id NixOS gives an entry: `nixos-generation-15.conf`
/// from its own boot loader, `nixos-generation-15-<hash>.efi` from
/// lanzaboote.
fn generation(id: &str) -> Option<u32> {
    let rest = id.strip_prefix("nixos-generation-")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let after = &rest[digits.len()..];
    if after == ".conf" || after.starts_with('-') {
        digits.parse().ok()
    } else {
        None
    }
}

/// `bootctl status`'s fields, each `Name: value` line by its name, the
/// first of each kept. It says what it can and exits 1 when it can't read
/// the ESP, as for anyone but root, so what it printed is read either way.
pub fn status(machine: &mut dyn Machine) -> Result<Fields, Fault> {
    let ran = machine.run(Tool::Bootctl, &["status"])?;
    let text = String::from_utf8(ran.stdout)
        .map_err(|_| Fault::unreadable("bootctl's status", "not UTF-8"))?;
    Ok(Fields::parse(&text))
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fields(BTreeMap<String, String>);

impl Fields {
    pub fn parse(text: &str) -> Fields {
        let mut fields = BTreeMap::new();
        for line in text.lines() {
            let Some((name, value)) = line.split_once(": ") else {
                continue;
            };
            let name = name.trim();
            let named = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-');
            if named {
                fields
                    .entry(name.to_string())
                    .or_insert_with(|| value.trim().to_string());
            }
        }
        Fields(fields)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Whether the running system booted from a UKI that measured itself
    /// into the TPM, which a policy on PCR 4 needs.
    pub fn measured_uki(&self) -> Option<bool> {
        match self.get("Measured UKI")? {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::Fake;

    #[test]
    fn the_surface_boots_generation_15_from_a_uki() {
        let entries = parse_list(surface!("bootctl-list.json")).unwrap();
        let booted: Vec<&Entry> = entries.iter().filter(|entry| entry.booted).collect();
        assert_eq!(booted.len(), 1);
        assert_eq!(booted[0].generation, Some(15));
        assert_eq!(booted[0].kind, Kind::Uki);
        assert!(booted[0].default);
    }

    #[test]
    fn the_surfaces_older_entries_are_numbered_too() {
        let entries = parse_list(surface!("bootctl-list.json")).unwrap();
        let ukis: Vec<u32> = entries
            .iter()
            .filter(|entry| entry.kind == Kind::Uki)
            .filter_map(|entry| entry.generation)
            .collect();
        assert_eq!(ukis, [15, 14, 13, 12]);
        let confs = entries
            .iter()
            .filter(|entry| entry.kind == Kind::Conf)
            .count();
        assert_eq!(confs, 14);
        let firmware = entries
            .iter()
            .find(|entry| entry.kind == Kind::Automatic)
            .unwrap();
        assert_eq!(firmware.generation, None);
    }

    #[test]
    fn only_nixos_ids_have_generations() {
        assert_eq!(generation("nixos-generation-7.conf"), Some(7));
        assert_eq!(generation("nixos-generation-12-2jraginct.efi"), Some(12));
        assert_eq!(generation("nixos-generation-12x.conf"), None);
        assert_eq!(generation("auto-reboot-to-firmware-setup"), None);
    }

    #[test]
    fn both_machines_booted_from_a_measured_uki() {
        for text in [xps!("bootctl-status.txt"), surface!("bootctl-status.txt")] {
            assert_eq!(Fields::parse(text).measured_uki(), Some(true));
        }
    }

    #[test]
    fn status_is_read_though_bootctl_couldnt_reach_the_esp() {
        let mut fake = Fake::new();
        fake.answers(
            Tool::Bootctl,
            &["status"],
            1,
            xps!("bootctl-status.txt"),
            xps!("bootctl-status.err"),
        );
        let fields = status(&mut fake).unwrap();
        assert_eq!(fields.get("Secure Boot"), Some("enabled (deployed)"));
        assert_eq!(fields.get("Product"), Some("systemd-boot 261.3"));
    }
}
