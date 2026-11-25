//! What a USB device says of itself in sysfs, read alike for its sound
//! cards, cameras and keyboards, and a Bluetooth device's release.

/// A USB release number as its device gives it: 0x1310 is "13.10", and
/// 0x0021 "0.21".
pub(crate) fn release(bcd: &str) -> String {
    let bcd = bcd.trim();
    match (bcd.get(..2).map(str::parse::<u8>), bcd.get(2..)) {
        (Some(Ok(major)), Some(minor)) if bcd.len() == 4 => format!("{major}.{minor}"),
        _ => bcd.to_owned(),
    }
}

/// A USB speed in Mbit/s, as it's said.
pub(crate) fn speed(megabits: &str) -> String {
    match megabits.trim().parse::<u32>() {
        Ok(fast) if fast >= 1000 && fast % 1000 == 0 => format!("{} Gbit/s", fast / 1000),
        Ok(speed) => format!("{speed} Mbit/s"),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_usb_release_reads_as_its_device_writes_it() {
        assert_eq!(release("1310\n"), "13.10");
        assert_eq!(release("0203"), "2.03");
        assert_eq!(release("0021"), "0.21");
        assert_eq!(release("21"), "21");
    }

    #[test]
    fn a_usb_speed_is_said_in_its_own_unit() {
        assert_eq!(speed("5000\n"), "5 Gbit/s");
        assert_eq!(speed("480"), "480 Mbit/s");
        assert_eq!(speed("12"), "12 Mbit/s");
        assert_eq!(speed(""), "");
    }
}
