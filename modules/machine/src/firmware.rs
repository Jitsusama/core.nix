//! What fwupd knows of the machine's firmware (`firmware:<id>`): each
//! device it names, its versions and what it can do, the update it
//! offers, or in fwupd's own words why it offers none; and the machine's
//! own (`firmware:host`), its security level as fwupd rates it. main.rs
//! asks fwupd over the system bus and hands its answers here as plain
//! JSON, so a record is a function of what fwupd said.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// fwupd's device flags by their bits, as its own names say them
/// (libfwupd/fwupd-device.rs, fwupd 2.1.6).
const FLAGS: [(u32, &str); 38] = [
    (0, "internal"),
    (1, "updatable"),
    (3, "require-ac"),
    (4, "locked"),
    (5, "supported"),
    (6, "needs-bootloader"),
    (8, "needs-reboot"),
    (9, "reported"),
    (10, "notified"),
    (13, "is-bootloader"),
    (14, "wait-for-replug"),
    (17, "needs-shutdown"),
    (18, "another-write-required"),
    (20, "needs-activation"),
    (22, "historical"),
    (24, "will-disappear"),
    (25, "can-verify"),
    (26, "can-verify-image"),
    (27, "dual-image"),
    (28, "self-recovery"),
    (29, "usable-during-update"),
    (30, "version-check-required"),
    (31, "install-all-releases"),
    (37, "updatable-hidden"),
    (39, "has-multiple-branches"),
    (40, "backup-before-install"),
    (42, "wildcard-install"),
    (43, "only-version-upgrade"),
    (44, "unreachable"),
    (45, "affects-fde"),
    (46, "end-of-life"),
    (47, "signed-payload"),
    (48, "unsigned-payload"),
    (49, "emulated"),
    (50, "emulation-tag"),
    (51, "only-explicit-updates"),
    (52, "can-emulation-tag"),
    (53, "install-skip-version-check"),
];

/// What keeps fwupd from updating a device, by its bits.
const PROBLEMS: [(u32, &str); 15] = [
    (0, "system-power-too-low"),
    (1, "unreachable"),
    (2, "power-too-low"),
    (3, "update-pending"),
    (4, "require-ac-power"),
    (5, "lid-is-closed"),
    (6, "is-emulated"),
    (7, "missing-license"),
    (8, "system-inhibit"),
    (9, "update-in-progress"),
    (10, "in-use"),
    (11, "display-required"),
    (12, "lower-priority"),
    (13, "insecure-platform"),
    (14, "firmware-locked"),
];

/// How urgent fwupd says a release is, by its number.
const URGENCIES: [&str; 5] = ["unknown", "low", "medium", "high", "critical"];

/// The names of the bits set in a mask, by a table of them; a bit the
/// table doesn't know left out.
fn named(mask: Option<u64>, table: &[(u32, &'static str)]) -> Vec<&'static str> {
    let Some(mask) = mask else {
        return vec![];
    };
    table
        .iter()
        .filter(|(bit, _)| mask & (1u64 << bit) != 0)
        .map(|(_, name)| *name)
        .collect()
}

/// What fwupd said of a release, as a record's update says it.
fn release(said: &Value) -> Value {
    let text = |field: &str| said[field].as_str().map(str::to_owned);
    json!({
        "version": text("Version"),
        "summary": text("Summary"),
        "description": text("Description"),
        "urgency": said["Urgency"].as_u64().and_then(|u| URGENCIES.get(usize::try_from(u).ok()?)).copied(),
        "size_b": said["Size"].as_u64(),
        "released_at_s": said["Created"].as_u64(),
        "vendor": text("Vendor"),
        "protocol": text("Protocol"),
        "uri": said["Locations"][0].as_str().map(str::to_owned).or_else(|| text("Uri")),
        "install_minutes": said["InstallDuration"].as_u64().map(|s| s.div_ceil(60)),
    })
}

/// What fwupd offers a device: its newest update, or why it offers
/// none, as fwupd said it.
pub type Offered = Result<Option<Value>, String>;

/// How an update fwupd kept in its history ended, by `FwupdUpdateState`'s
/// number, as its own `ToString` names it (libfwupd/fwupd-enums.rs).
const UPDATE_STATES: [&str; 6] = [
    "unknown",
    "pending",
    "success",
    "failed",
    "needs-reboot",
    "failed-transient",
];

/// The newest update fwupd's history keeps for each device, by its id:
/// how it ended, when, to which version, and fwupd's words for why it
/// failed. Firmware says from it when a device was last updated, and
/// which version waits for the restart.
pub fn history(entries: &[Value]) -> BTreeMap<String, Value> {
    let mut newest: BTreeMap<String, &Value> = BTreeMap::new();
    for entry in entries {
        let Some(id) = entry["DeviceId"].as_str() else {
            continue;
        };
        let when = |each: &Value| each["Modified"].as_u64().or(each["Created"].as_u64());
        if newest.get(id).is_none_or(|kept| when(entry) >= when(kept)) {
            newest.insert(id.to_owned(), entry);
        }
    }
    newest
        .into_iter()
        .map(|(id, entry)| {
            let said = json!({
                "state": entry["UpdateState"]
                    .as_u64()
                    .and_then(|s| UPDATE_STATES.get(usize::try_from(s).ok()?))
                    .copied(),
                "at_s": entry["Modified"].as_u64().or(entry["Created"].as_u64()),
                "version": entry["Releases"][0]["Version"].as_str(),
                "error": entry["UpdateError"].as_str(),
            });
            (id, said)
        })
        .collect()
}

/// Each device fwupd names, a device with no name (a GPIO pin it keeps
/// for its own checks) left out, with what's offered for it.
pub fn records(
    devices: &[Value],
    offered: &BTreeMap<String, Offered>,
    history: &BTreeMap<String, Value>,
) -> Vec<Value> {
    devices
        .iter()
        .filter_map(|device| {
            let id = device["DeviceId"].as_str()?;
            let name = device["Name"].as_str()?;
            let text = |field: &str| device[field].as_str().map(str::to_owned);
            let (update, reason) = match offered.get(id) {
                Some(Ok(Some(said))) => (release(said), None),
                Some(Ok(None)) | None => (Value::Null, None),
                Some(Err(why)) => (Value::Null, Some(why.clone())),
            };
            Some(json!({
                "key": format!("firmware:{id}"),
                "kind": "firmware",
                "id": id,
                "name": name,
                "vendor": text("Vendor"),
                "summary": text("Summary"),
                "version": text("Version"),
                "version_lowest": text("VersionLowest"),
                "version_bootloader": text("VersionBootloader"),
                "version_format": device["VersionFormat"].as_u64(),
                "branch": text("Branch"),
                "serial": text("Serial"),
                "plugin": text("Plugin"),
                "protocols": device["Protocols"].clone(),
                "parent": text("ParentDeviceId"),
                "flags": named(device["Flags"].as_u64(), &FLAGS),
                "problems": named(device["Problems"].as_u64(), &PROBLEMS),
                "update_error": text("UpdateError"),
                "flashes_left": device["FlashesLeft"].as_u64(),
                "battery_pct": device["BatteryLevel"].as_u64().filter(|level| *level <= 100),
                "created_at_s": device["Created"].as_u64(),
                "modified_at_s": device["Modified"].as_u64(),
                "update": update,
                "no_update": reason,
                "last_update": history.get(id).cloned(),
            }))
        })
        .collect()
}

/// The machine's own, as fwupd rates it: its security level (HSI), the
/// product and fwupd's version.
pub fn host(security: Option<&str>, product: Option<&str>, version: Option<&str>) -> Value {
    json!({
        "key": "firmware:host",
        "kind": "firmware",
        "security": security,
        "product": product,
        "fwupd": version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mask_says_its_names() {
        assert_eq!(
            named(Some(0b11 | (1 << 25)), &FLAGS),
            ["internal", "updatable", "can-verify"]
        );
        assert_eq!(named(Some(1 << 4), &PROBLEMS), ["require-ac-power"]);
        assert!(named(Some(1 << 2), &FLAGS).is_empty(), "an unnamed bit");
    }

    #[test]
    fn a_device_says_its_update_or_why_none() {
        let devices = [
            json!({"DeviceId": "a1", "Name": "System Firmware", "Version": "1.2.0", "Flags": 3}),
            json!({"DeviceId": "b2", "Name": "Touchpad", "Version": "0.9"}),
            json!({"DeviceId": "c3", "Flags": 1u64 << 52}),
        ];
        let offered = BTreeMap::from([
            (
                "a1".to_owned(),
                Ok(Some(
                    json!({"Version": "1.3.0", "Urgency": 3, "Size": 32_000_000}),
                )),
            ),
            ("b2".to_owned(), Err("No releases found".to_owned())),
        ]);
        let kept = history(&[
            json!({"DeviceId": "a1", "UpdateState": 2, "Modified": 100,
                "Releases": [{"Version": "1.1.0"}]}),
            json!({"DeviceId": "a1", "UpdateState": 4, "Modified": 200,
                "Releases": [{"Version": "1.2.0"}]}),
        ]);
        let found = records(&devices, &offered, &kept);
        assert_eq!(
            found[0]["last_update"]["state"], "needs-reboot",
            "the newest kept"
        );
        assert_eq!(found[0]["last_update"]["version"], "1.2.0");
        assert_eq!(found[1]["last_update"], Value::Null);
        assert_eq!(found.len(), 2, "a device with no name is left out");
        assert_eq!(found[0]["update"]["version"], "1.3.0");
        assert_eq!(found[0]["update"]["urgency"], "high");
        assert_eq!(found[0]["flags"], json!(["internal", "updatable"]));
        assert_eq!(found[1]["no_update"], "No releases found");
    }
}
