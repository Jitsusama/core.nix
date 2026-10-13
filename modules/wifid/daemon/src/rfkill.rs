//! The kernel's radio switches, through /dev/rfkill. Opened, it says every
//! switch there is, then each one added, changed or taken away, eight
//! bytes each; written, it turns every Wi-Fi radio off or on at once.

use std::collections::BTreeMap;

/// The device every radio's switch is read and set through.
pub const DEVICE: &str = "/dev/rfkill";

/// What the kernel says of one switch, as it first comes.
pub type Event = [u8; 8];

/// Wi-Fi's kind of switch, among Bluetooth's and the rest.
const WLAN: u8 = 1;

/// What happened to the switch an event is about.
const ADD: u8 = 0;
const DELETE: u8 = 1;
const CHANGE: u8 = 2;
const CHANGE_ALL: u8 = 3;

/// Every Wi-Fi radio's switch, by the kernel's index for it.
#[derive(Debug, Default)]
pub struct Switches {
    wifi: BTreeMap<u32, Switch>,
}

#[derive(Debug)]
struct Switch {
    /// Turned off from the machine, as wifid does.
    soft: bool,
    /// Turned off by a switch on the machine, which only a hand undoes.
    hard: bool,
}

impl Switches {
    /// Takes in what the kernel says of a switch.
    pub fn take_in(&mut self, event: Event) {
        let [a, b, c, d, kind, op, soft, hard] = event;
        if kind != WLAN {
            return;
        }
        let index = u32::from_ne_bytes([a, b, c, d]);
        let switch = Switch {
            soft: soft != 0,
            hard: hard != 0,
        };
        match op {
            ADD | CHANGE => {
                self.wifi.insert(index, switch);
            }
            DELETE => {
                self.wifi.remove(&index);
            }
            _ => {}
        }
    }

    /// Whether Wi-Fi can be used: none of its radios is switched off. A
    /// machine with no switch for Wi-Fi at all can't turn it off either.
    pub fn on(&self) -> bool {
        self.wifi
            .values()
            .all(|switch| !switch.soft && !switch.hard)
    }
}

/// What to write to turn every Wi-Fi radio on, or off.
pub fn turning(on: bool) -> Event {
    [0, 0, 0, 0, WLAN, CHANGE_ALL, !on as u8, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(index: u32, kind: u8, op: u8, soft: bool, hard: bool) -> Event {
        let mut event = [0; 8];
        event[..4].copy_from_slice(&index.to_ne_bytes());
        event[4..].copy_from_slice(&[kind, op, soft as u8, hard as u8]);
        event
    }

    #[test]
    fn wifi_is_on_while_no_radio_of_it_is_switched_off() {
        let mut switches = Switches::default();

        switches.take_in(event(0, WLAN, ADD, false, false));
        switches.take_in(event(1, WLAN, ADD, false, false));

        assert!(switches.on());
    }

    #[test]
    fn one_radio_switched_off_turns_wifi_off() {
        let mut switches = Switches::default();
        switches.take_in(event(0, WLAN, ADD, false, false));
        switches.take_in(event(1, WLAN, ADD, false, false));

        switches.take_in(event(1, WLAN, CHANGE, true, false));

        assert!(!switches.on());
    }

    #[test]
    fn a_hand_on_the_machine_s_switch_turns_it_off_too() {
        let mut switches = Switches::default();

        switches.take_in(event(0, WLAN, ADD, false, true));

        assert!(!switches.on());
    }

    #[test]
    fn a_radio_switched_off_and_taken_away_no_longer_counts() {
        let mut switches = Switches::default();
        switches.take_in(event(0, WLAN, ADD, false, false));
        switches.take_in(event(1, WLAN, ADD, true, false));

        switches.take_in(event(1, WLAN, DELETE, true, false));

        assert!(switches.on());
    }

    #[test]
    fn bluetooth_s_switch_isn_t_wifi_s() {
        let mut switches = Switches::default();
        switches.take_in(event(0, WLAN, ADD, false, false));

        switches.take_in(event(1, 2, ADD, true, false));

        assert!(switches.on());
    }

    #[test]
    fn turning_wifi_off_switches_every_radio_of_it_at_once() {
        let off = turning(false);
        let on = turning(true);

        assert_eq!(off, event(0, WLAN, CHANGE_ALL, true, false));
        assert_eq!(on, event(0, WLAN, CHANGE_ALL, false, false));
    }
}
