//! What a device is called, said one way by the command line's views, the
//! notices and the launcher's rows: the name eero gives it, its nickname or
//! its hostname; else its maker's, as "Intel device"; else its hardware
//! address. eero falls back to a device's maker for the name it shows when
//! nothing else names it, which the read leaves out (`read::devices`), so
//! the maker is said as a maker here rather than as though it were a name.

use std::borrow::Cow;

use crate::model::{Device, DeviceOff};

/// How the command line says a device known only by its maker; the rows
/// say it in the launcher's words (`eero:device`'s `made`).
pub const MADE: &str = "{maker} device";

/// What a company adds to its name that says nothing of which it is, as
/// eero lists makers: "Intel Corporate", "Sony Corporation", "ecobee inc".
const FORMS: [&str; 11] = [
    "co",
    "co.",
    "corp",
    "corp.",
    "corporate",
    "corporation",
    "gmbh",
    "inc",
    "inc.",
    "llc",
    "ltd",
];

/// What a device on now is called, a maker said as `made` says one.
pub fn device<'a>(device: &'a Device, made: &str) -> Cow<'a, str> {
    called(
        [&device.name, &device.nickname, &device.hostname],
        [&device.make, &device.manufacturer],
        &device.mac,
        made,
    )
}

/// What a device not on now is called, as `device` says one on now.
pub fn off<'a>(device: &'a DeviceOff, made: &str) -> Cow<'a, str> {
    called(
        [&device.name, &device.nickname, &device.hostname],
        [&device.make, &device.manufacturer],
        &device.mac,
        made,
    )
}

fn called<'a>(
    names: [&'a Option<String>; 3],
    makers: [&Option<String>; 2],
    mac: &'a str,
    made: &str,
) -> Cow<'a, str> {
    let given = names
        .into_iter()
        .flatten()
        .find(|name| !name.trim().is_empty());
    if let Some(name) = given {
        return Cow::Borrowed(name);
    }
    makers
        .into_iter()
        .flatten()
        .find_map(|maker| plain_maker(maker))
        .map_or(Cow::Borrowed(mac), |maker| {
            Cow::Owned(made.replace("{maker}", &maker))
        })
}

/// A maker's name without the forms of company it ends in: "Samsung
/// Electronics Co.,Ltd" as "Samsung Electronics"; none for one that is
/// nothing but those.
fn plain_maker(maker: &str) -> Option<String> {
    let before_comma = maker.split(',').next().unwrap_or("");
    let mut words: Vec<&str> = before_comma.split_whitespace().collect();
    while words.len() > 1
        && words
            .last()
            .is_some_and(|word| FORMS.contains(&word.to_ascii_lowercase().as_str()))
    {
        words.pop();
    }
    let plain = words.join(" ");
    (!plain.is_empty()).then_some(plain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::tests::model;

    /// A device of the fixture's with nothing naming or making it.
    fn unnamed() -> Device {
        Device {
            mac: "02:00:00:00:00:99".to_owned(),
            name: None,
            nickname: None,
            hostname: None,
            make: None,
            manufacturer: None,
            ..model().devices[0].clone()
        }
    }

    #[test]
    fn a_device_is_called_by_its_name_then_nickname_then_hostname() {
        let mut one = Device {
            name: Some("Kitchen speaker".to_owned()),
            nickname: Some("speaker".to_owned()),
            hostname: Some("speaker-01".to_owned()),
            ..unnamed()
        };
        assert_eq!(device(&one, MADE), "Kitchen speaker");
        one.name = None;
        assert_eq!(device(&one, MADE), "speaker");
        one.nickname = Some(" ".to_owned());
        assert_eq!(device(&one, MADE), "speaker-01");
    }

    #[test]
    fn a_device_nothing_names_is_called_by_its_maker() {
        let one = Device {
            manufacturer: Some("Intel Corporate".to_owned()),
            ..unnamed()
        };

        assert_eq!(device(&one, MADE), "Intel device");
    }

    #[test]
    fn the_make_eero_works_out_is_said_before_the_registered_maker() {
        let one = Device {
            make: Some("Samsung".to_owned()),
            manufacturer: Some("Samsung Electronics Co.,Ltd".to_owned()),
            ..unnamed()
        };

        assert_eq!(device(&one, "{maker} thing"), "Samsung thing");
    }

    #[test]
    fn a_maker_is_said_without_the_form_of_company_it_is() {
        let said: Vec<Option<String>> = [
            "Samsung Electronics Co.,Ltd",
            "Sony Corporation",
            "Apple, Inc.",
            "ecobee inc",
            "AzureWave Technology Inc.",
            "Inc.",
        ]
        .into_iter()
        .map(plain_maker)
        .collect();

        assert_eq!(
            said,
            [
                Some("Samsung Electronics".to_owned()),
                Some("Sony".to_owned()),
                Some("Apple".to_owned()),
                Some("ecobee".to_owned()),
                Some("AzureWave Technology".to_owned()),
                Some("Inc.".to_owned()),
            ]
        );
    }

    #[test]
    fn a_device_nothing_names_or_makes_is_called_by_its_address() {
        assert_eq!(device(&unnamed(), MADE), "02:00:00:00:00:99");
    }

    #[test]
    fn a_device_not_on_now_is_called_the_same_way() {
        let one = DeviceOff {
            mac: "02:00:00:00:00:98".to_owned(),
            name: None,
            nickname: None,
            hostname: None,
            make: None,
            manufacturer: Some("Intel Corporate".to_owned()),
            ..model().devices_off[0].clone()
        };

        assert_eq!(off(&one, MADE), "Intel device");
    }
}
