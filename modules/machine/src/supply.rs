//! Each power supply the kernel knows (`supply:<name>`): the battery, the
//! charger and each USB-C port, with every figure its folder in
//! `/sys/class/power_supply` has, a USB-C port's roles from its folder in
//! `/sys/class/typec`, and what `UPower` works out of the battery, its time
//! to empty or full, as main.rs last asked it.

use serde_json::{Value, json};

use crate::{Root, rounded};

const SUPPLIES: &str = "/sys/class/power_supply";

/// The choice a kernel list marks, as `[sink]` in `source [sink]`, and
/// every choice it lists.
pub fn chosen(text: Option<String>) -> (Option<String>, Vec<String>) {
    let Some(text) = text else {
        return (None, vec![]);
    };
    let mut chosen = None;
    let all = text
        .split_whitespace()
        .map(|word| {
            let plain = word.trim_start_matches('[').trim_end_matches(']');
            if word.starts_with('[') {
                chosen = Some(plain.to_owned());
            }
            plain.to_owned()
        })
        .collect();
    (chosen, all)
}

/// A figure the kernel gives in millionths, as volts, amps, watts, amp
/// hours and watt hours, in whole units.
fn millionths(root: &Root, folder: &str, file: &str) -> Option<f64> {
    let value: i64 = root.said(&format!("{folder}/{file}"))?.parse().ok()?;
    i32::try_from(value / 1000)
        .ok()
        .map(|thousandths| f64::from(thousandths) / 1000.0)
}

/// Whether Joel's account may write a file, as the charge limit's.
fn writable(root: &Root, folder: &str, file: &str) -> Option<bool> {
    let path = root.path(&format!("{folder}/{file}"));
    if !path.exists() {
        return None;
    }
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
    // SAFETY: access reads the path it's given, a string ending in nul.
    Some(unsafe { libc::access(path.as_ptr(), libc::W_OK) } == 0)
}

/// A USB-C port's roles, by its supply's number (`USBC000:001` is the
/// kernel's typec port0), and what's plugged into it.
fn port(root: &Root, name: &str) -> Value {
    let Some(n) = name
        .rsplit(':')
        .next()
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| *n > 0)
    else {
        return Value::Null;
    };
    let folder = format!("/sys/class/typec/port{}", n - 1);
    if !root.path(&folder).exists() {
        return Value::Null;
    }
    let at = |file: &str| root.said(&format!("{folder}/{file}"));
    let partner = format!("{folder}-partner");
    let plugged = root.path(&partner).exists().then(|| {
        let at = |file: &str| root.said(&format!("{partner}/{file}"));
        json!({
            "power_delivery": at("supports_usb_power_delivery").map(|yes| yes == "yes"),
            "power_delivery_revision": at("usb_power_delivery_revision").filter(|r| r != "0.0"),
            "accessory": at("accessory_mode").filter(|mode| mode != "none"),
        })
    });
    json!({
        "port": n - 1,
        "power_role": chosen(at("power_role")).0,
        "data_role": chosen(at("data_role")).0,
        "operation_mode": at("power_operation_mode"),
        "power_delivery_revision": at("usb_power_delivery_revision").filter(|r| r != "0.0"),
        "typec_revision": at("usb_typec_revision"),
        "partner": plugged,
    })
}

/// One supply's record; `upower` what `UPower` last said of it, or null.
pub fn record(root: &Root, name: &str, upower: &Value) -> Value {
    let folder = format!("{SUPPLIES}/{name}");
    let at = |file: &str| root.said(&format!("{folder}/{file}"));
    let whole = |file: &str| root.number(&format!("{folder}/{file}"));
    let units = |file: &str| millionths(root, &folder, file).map(|v| rounded(v, 3));
    let volts = millionths(root, &folder, "voltage_now");
    let amps = millionths(root, &folder, "current_now");
    let watts = millionths(root, &folder, "power_now")
        .or_else(|| Some((volts? * amps?).abs()))
        .map(|w| rounded(w, 3));
    // Health against design, from charge where the battery counts it so,
    // else from energy.
    let health = ["charge", "energy"].iter().find_map(|unit| {
        let full = whole(&format!("{unit}_full"))?;
        let design = whole(&format!("{unit}_full_design")).filter(|d| *d > 0)?;
        Some(rounded(crate::float(full) / crate::float(design), 4))
    });
    let made = match (
        whole("manufacture_year"),
        whole("manufacture_month"),
        whole("manufacture_day"),
    ) {
        (Some(year), Some(month), Some(day)) => Some(format!("{year:04}-{month:02}-{day:02}")),
        _ => None,
    };
    let (charge_type, charge_types) = chosen(at("charge_types"));
    let (usb_type, usb_types) = chosen(at("usb_type"));
    json!({
        "key": format!("supply:{name}"),
        "kind": "supply",
        "name": name,
        "type": at("type"),
        "online": whole("online").map(|o| o == 1),
        "present": whole("present").map(|p| p == 1),
        "status": at("status"),
        "scope": at("scope"),
        "capacity_pct": whole("capacity"),
        "capacity_level": at("capacity_level"),
        "charge_ah": units("charge_now"),
        "charge_full_ah": units("charge_full"),
        "charge_design_ah": units("charge_full_design"),
        "energy_wh": units("energy_now"),
        "energy_full_wh": units("energy_full"),
        "energy_design_wh": units("energy_full_design"),
        "health_ratio": health,
        "volts": volts.map(|v| rounded(v, 3)),
        "amps": amps.map(|a| rounded(a, 3)),
        "watts": watts,
        "min_design_volts": units("voltage_min_design"),
        "max_volts": units("voltage_max"),
        "min_volts": units("voltage_min"),
        "max_amps": units("current_max"),
        "cycles": whole("cycle_count"),
        "health": at("health"),
        "technology": at("technology"),
        "charge_type": charge_type.or_else(|| at("charge_type")),
        "charge_types": charge_types,
        "limit_start_pct": whole("charge_control_start_threshold"),
        "limit_stop_pct": whole("charge_control_end_threshold"),
        "limit_settable": writable(root, &folder, "charge_control_end_threshold"),
        "alarm": whole("alarm"),
        "manufacturer": at("manufacturer"),
        "model": at("model_name"),
        "serial": at("serial_number"),
        "made": made,
        "temperature_c": whole("temp").and_then(|t| u32::try_from(t).ok()).map(|t| rounded(f64::from(t) / 10.0, 1)),
        "usb_type": usb_type,
        "usb_types": usb_types,
        "typec": port(root, name),
        "upower": upower,
    })
}

/// The supplies the kernel lists, by name.
pub fn supplies(root: &Root) -> Vec<String> {
    root.names(SUPPLIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_marks_its_choice() {
        assert_eq!(
            chosen(Some("source [sink]".into())),
            (Some("sink".into()), vec!["source".into(), "sink".into()])
        );
        assert_eq!(
            chosen(Some("C [PD] PD_PPS".into())).0.as_deref(),
            Some("PD")
        );
        assert_eq!(chosen(None), (None, vec![]));
    }
}
