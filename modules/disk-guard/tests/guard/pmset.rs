use disk_guard::moments::Power;
use disk_guard::pmset::power;

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
