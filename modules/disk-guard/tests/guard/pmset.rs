use disk_guard::moments::Power;
use disk_guard::pmset::{low_power, power};

#[test]
fn a_laptop_on_mains_is_on_mains_with_its_charge() {
    let text = "Now drawing from 'AC Power'\n \
                -InternalBattery-0 (id=4653155)\t87%; charging; 0:42 remaining present: true\n";
    assert_eq!(
        power(text),
        Some(Power {
            mains: true,
            battery: Some(87)
        })
    );
}

#[test]
fn a_laptop_on_its_battery_is_not_on_mains() {
    let text = "Now drawing from 'Battery Power'\n \
                -InternalBattery-0 (id=4653155)\t64%; discharging; 3:10 remaining present: true\n";
    assert_eq!(
        power(text),
        Some(Power {
            mains: false,
            battery: Some(64)
        })
    );
}

#[test]
fn a_desktop_has_no_battery_and_a_ups_is_not_mains() {
    assert_eq!(
        power("Now drawing from 'AC Power'\n"),
        Some(Power {
            mains: true,
            battery: None
        })
    );
    let ups = "Now drawing from 'UPS Power'\n -UPS (id=1)\t40%; discharging; present: true\n";
    assert_eq!(
        power(ups),
        Some(Power {
            mains: false,
            battery: None
        })
    );
}

#[test]
fn output_saying_nothing_of_power_says_nothing() {
    assert_eq!(power(""), None);
    assert_eq!(power("pmset: unknown option\n"), None);
}

/// The settings `pmset -g` prints, Low Power Mode among them as `on`.
fn settings(low_power_mode: &str) -> String {
    format!(
        "System-wide power settings:\n\
         Currently in use:\n \
         standby              1\n \
         lowpowermode         {low_power_mode}\n \
         powermode            0\n \
         sleep                1 (sleep prevented by coreaudiod)\n"
    )
}

#[test]
fn low_power_mode_is_saving_power() {
    assert!(low_power(&settings("1")));
    assert!(!low_power(&settings("0")));
}

#[test]
fn settings_without_low_power_mode_save_nothing() {
    assert!(!low_power("Currently in use:\n standby 1\n"));
    assert!(!low_power(""));
    // A setting whose name only begins with it is another.
    assert!(!low_power("Currently in use:\n lowpowermodeforever 1\n"));
}
