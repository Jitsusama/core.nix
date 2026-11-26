//! A monitor's USB-C cable, from the Type-C class and UCSI.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Facts, fact};

/// The SVID of the DisplayPort alternate mode.
const DISPLAYPORT_SVID: &str = "ff01";

/// The bracketed choice in a sysfs list, as "[usb2] usb3" gives "usb2".
fn chosen(list: &str) -> Option<&str> {
    let (_, rest) = list.split_once('[')?;
    Some(rest.split_once(']')?.0)
}

/// A USB mode as Type-C names it, said plainly.
fn usb_mode(mode: &str) -> String {
    match mode {
        "usb2" => "USB 2".to_owned(),
        "usb3" => "USB 3".to_owned(),
        "usb4" => "USB4".to_owned(),
        other => other.to_owned(),
    }
}

/// Volts from millivolts or microvolts, amps from milliamps or microamps,
/// to the nearest tenth, a whole number without its point.
fn tenths(thousandths: u64) -> String {
    let tenths = (thousandths + 50) / 100;
    if tenths.is_multiple_of(10) {
        format!("{}", tenths / 10)
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

/// What a source offers, from its fixed supplies' voltages and currents in
/// millivolts and milliamps: "5, 9 and 20 V at 3 A" when they share a
/// current, and otherwise each with its own.
fn offered_power(supplies: &[(u64, u64)]) -> String {
    let Some(&(_, current)) = supplies.first() else {
        return String::new();
    };
    let volts: Vec<String> = supplies.iter().map(|(mv, _)| tenths(*mv)).collect();
    if supplies.iter().all(|(_, each)| *each == current) {
        let listed = match volts.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
            None => String::new(),
        };
        return format!("{listed} V at {} A", tenths(current));
    }
    supplies
        .iter()
        .map(|(mv, ma)| format!("{} V at {} A", tenths(*mv), tenths(*ma)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The fixed supplies a partner's PD source capabilities list, in
/// millivolts and milliamps, in their order.
fn fixed_supplies(partner: &Path) -> Vec<(u64, u64)> {
    let Ok(entries) = fs::read_dir(partner) else {
        return Vec::new();
    };
    let Some(source) = entries
        .flatten()
        .map(|entry| entry.path().join("source-capabilities"))
        .find(|path| path.is_dir())
    else {
        return Vec::new();
    };
    let mut supplies: Vec<(u32, u64, u64)> = fs::read_dir(source)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (index, "fixed_supply") = name.split_once(':')? else {
                return None;
            };
            let read = |file: &str| {
                let text = fs::read_to_string(entry.path().join(file)).ok()?;
                text.trim()
                    .trim_end_matches(|c: char| c.is_ascii_alphabetic())
                    .parse()
                    .ok()
            };
            Some((
                index.parse().ok()?,
                read("voltage")?,
                read("maximum_current")?,
            ))
        })
        .collect();
    supplies.sort_unstable();
    supplies.into_iter().map(|(_, mv, ma)| (mv, ma)).collect()
}

/// What the USB-C cable to the one monitor carries, when exactly one
/// partner has DisplayPort's alternate mode active: how its data runs
/// beside the picture, the power it charges the machine with, and what it
/// offers. With two such partners nothing says which display is whose, so
/// none of this is said.
pub(crate) fn usb_c(sys: &Path) -> Facts {
    let class = sys.join("class/typec");
    let Ok(entries) = fs::read_dir(&class) else {
        return Facts::new();
    };
    let showing: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|partner| {
            partner
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-partner"))
                && fs::read_dir(partner)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|mode| {
                        let read = |file: &str| {
                            fs::read_to_string(mode.path().join(file)).unwrap_or_default()
                        };
                        read("svid").trim() == DISPLAYPORT_SVID && read("active").trim() == "yes"
                    })
        })
        .collect();
    let [partner] = &showing[..] else {
        return Facts::new();
    };
    let mut facts = Facts::new();
    let mode = fs::read_to_string(partner.join("usb_mode")).unwrap_or_default();
    fact(
        &mut facts,
        "USB-C",
        chosen(&mode).map_or("DisplayPort".to_owned(), |mode| {
            format!("DisplayPort and {}", usb_mode(mode))
        }),
    );
    // UCSI names a port's supply for its device and the connector's
    // number, one more than the port's: port1 of USBC000:00 charges
    // through ucsi-source-psy-USBC000:002.
    let port = partner
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix("-partner"))
        .unwrap_or_default();
    let index = port
        .strip_prefix("port")
        .and_then(|n| n.parse::<u32>().ok());
    let device = fs::read_link(class.join(port).join("device"))
        .ok()
        .and_then(|link| {
            link.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
    if let (Some(index), Some(device)) = (index, device) {
        let supply = sys
            .join("class/power_supply")
            .join(format!("ucsi-source-psy-{device}{}", index + 1));
        let read = |file: &str| {
            fs::read_to_string(supply.join(file))
                .ok()
                .and_then(|text| text.trim().parse::<u64>().ok())
        };
        if let (Some(1), Some(volts), Some(amps)) =
            (read("online"), read("voltage_now"), read("current_max"))
        {
            let watts = (volts / 1000 * (amps / 1000) + 500_000) / 1_000_000;
            fact(
                &mut facts,
                "charging",
                format!(
                    "{watts} W ({} V, {} A)",
                    tenths(volts / 1000),
                    tenths(amps / 1000)
                ),
            );
        }
    }
    fact(
        &mut facts,
        "power offered",
        offered_power(&fixed_supplies(partner)),
    );
    facts
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn power_is_said_in_volts_amps_and_watts_plainly() {
        assert_eq!(tenths(20_000), "20");
        assert_eq!(tenths(1500), "1.5");
        assert_eq!(chosen("[usb2] usb3"), Some("usb2"));
        assert_eq!(chosen("usb2 usb3"), None);
        assert_eq!(
            offered_power(&[(5000, 3000), (9000, 3000), (20_000, 3000)]),
            "5, 9 and 20 V at 3 A"
        );
        assert_eq!(offered_power(&[(5000, 3000)]), "5 V at 3 A");
        assert_eq!(
            offered_power(&[(5000, 3000), (20_000, 2250)]),
            "5 V at 3 A, 20 V at 2.3 A"
        );
        assert_eq!(offered_power(&[]), "");
    }

    /// A monitor's partner on a USB-C port, as UCSI shows one, charging the
    /// machine through the port's supply when it's port1.
    pub(crate) fn partner(root: &Path, port: &str) {
        let partner = format!("class/typec/{port}-partner");
        put(root, &format!("{partner}/{port}-partner.0/svid"), "ff01\n");
        put(root, &format!("{partner}/{port}-partner.0/active"), "yes\n");
        put(root, &format!("{partner}/usb_mode"), "[usb2] usb3\n");
        for (index, volts) in [(1, 5000), (2, 9000), (3, 12_000), (4, 15_000), (5, 20_000)] {
            let supply = format!("{partner}/pd3/source-capabilities/{index}:fixed_supply");
            put(root, &format!("{supply}/voltage"), &format!("{volts}mV\n"));
            put(root, &format!("{supply}/maximum_current"), "5000mA\n");
        }
        fs::create_dir_all(root.join("devices/platform/USBC000:00")).unwrap();
        fs::create_dir_all(root.join(format!("class/typec/{port}"))).unwrap();
        std::os::unix::fs::symlink(
            "../../../devices/platform/USBC000:00",
            root.join(format!("class/typec/{port}/device")),
        )
        .unwrap();
        let supply = "class/power_supply/ucsi-source-psy-USBC000:002";
        put(root, &format!("{supply}/online"), "1\n");
        put(root, &format!("{supply}/voltage_now"), "20000000\n");
        put(root, &format!("{supply}/current_max"), "5000000\n");
    }

    #[test]
    fn the_monitor_s_usb_c_cable_says_its_data_its_charge_and_its_offer() {
        let sys = Sys::new("usb-c");
        partner(&sys.0, "port1");
        let facts = usb_c(&sys.0);
        let said: Vec<(&str, &str)> = facts
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert_eq!(
            said,
            vec![
                ("USB-C", "DisplayPort and USB 2"),
                ("charging", "100 W (20 V, 5 A)"),
                ("power offered", "5, 9, 12, 15 and 20 V at 5 A"),
            ]
        );
    }

    #[test]
    fn two_monitors_cables_say_nothing_since_neither_is_known_to_be_whose() {
        let sys = Sys::new("usb-c-two");
        partner(&sys.0, "port0");
        partner(&sys.0, "port1");
        assert_eq!(
            usb_c(&sys.0),
            [] as [(std::string::String, std::string::String); 0]
        );
    }
}
