//! What completion offers for a view's word: the views themselves, and the
//! home network's nodes, devices and profiles by the names each view finds
//! them by, each with a few words saying which it is.
//!
//! A name is offered only when it finds the thing it names, so where two
//! devices share one, each is offered by its hardware address instead,
//! which is what the view would ask for. A name holding a character the
//! terminal kit keeps from the terminal is offered as the kit draws it,
//! which the views find it by too.

use super::{
    VIEWS, device_name, find_device, find_node, find_profile, location, node_name, off_device,
    off_name, on_now_answers_to, profile_name, shown,
};
use terminal_kit::layout;

use crate::model::HomeNetwork;

/// A name to offer and what it is.
pub type Named = (String, String);

/// The views, by the word that names each.
pub fn views() -> Vec<Named> {
    VIEWS
        .iter()
        .map(|(name, what)| ((*name).to_owned(), (*what).to_owned()))
        .collect()
}

/// Each node by its location, or its id where another shares it.
pub fn nodes(model: &HomeNetwork) -> Vec<Named> {
    model
        .nodes
        .iter()
        .map(|node| {
            // Asked for as it's offered, drawn, so a word offered finds
            // what it names.
            let name = shown(node_name(node));
            let found = find_node(model, &name).is_ok_and(|found| found.id == node.id);
            let what = [node.model.clone(), node.gateway.then(|| "gateway".into())]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ");
            (if found { name } else { node.id.clone() }, what)
        })
        .collect()
}

/// Each device on now by its name, or its hardware address where another
/// shares the name, then each eero knows that isn't on now the same way,
/// since `eero device` says where and when that one was last on.
pub fn devices(model: &HomeNetwork) -> Vec<Named> {
    let on = model.devices.iter().map(|device| {
        let name = shown(&device_name(device));
        let found = find_device(model, &name, None).is_ok_and(|found| found.mac == device.mac);
        let on = location(model, device.node.as_deref());
        if found {
            (name, format!("on {on}"))
        } else {
            (device.mac.clone(), format!("{name} on {on}"))
        }
    });
    let off = model.devices_off.iter().map(|device| {
        let name = shown(&off_name(device));
        let found = !on_now_answers_to(model, &name)
            && off_device(model, &name).is_some_and(|found| found.mac == device.mac);
        let last = match device.node.as_deref() {
            Some(node) => format!("not on now, last on {}", location(model, Some(node))),
            None => "not on now".to_owned(),
        };
        if found {
            (name, last)
        } else {
            (device.mac.clone(), format!("{name}, {last}"))
        }
    });
    on.chain(off).collect()
}

/// Each profile by its name, or its id where another shares it.
pub fn profiles(model: &HomeNetwork) -> Vec<Named> {
    model
        .profiles
        .iter()
        .map(|profile| {
            let name = shown(profile_name(profile));
            let found = find_profile(model, &name).is_ok_and(|found| found.id == profile.id);
            (
                if found { name } else { profile.id.clone() },
                super::counted(profile.devices.len(), "device"),
            )
        })
        .collect()
}

/// The names as zsh's `_describe` reads them, one a line: the name, its
/// backslashes and colons escaped, since zsh takes a backslash off
/// whatever it's before and a colon ends the name, then a colon and what
/// it is, each held to one line, so a name from the network can't add a
/// line zsh reads as a name of its own.
pub fn for_zsh(names: &[Named]) -> String {
    let mut out = String::new();
    for (name, what) in names {
        let name = layout::one_line(name);
        out.push_str(&name.trim().replace('\\', "\\\\").replace(':', "\\:"));
        out.push(':');
        out.push_str(&layout::one_line(what));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::MODEL;

    fn model() -> HomeNetwork {
        serde_json::from_str(MODEL).unwrap()
    }

    fn names(named: &[Named]) -> Vec<&str> {
        named.iter().map(|(name, _)| name.as_str()).collect()
    }

    #[test]
    fn nodes_are_offered_by_location_with_their_model() {
        let nodes = nodes(&model());

        assert_eq!(names(&nodes), ["Hall", "Study", "Den"]);
        assert_eq!(nodes[0].1, "eero Pro 6, gateway");
    }

    #[test]
    fn a_device_name_two_share_is_offered_as_each_ones_address() {
        let devices = devices(&model());

        let offered = names(&devices);
        assert!(offered.contains(&"tablet 1"), "{offered:?}");
        assert!(!offered.contains(&"phone 1"), "{offered:?}");
        assert!(offered.contains(&"02:00:00:00:00:03"), "{offered:?}");
        let phone = devices.iter().find(|(name, _)| name == "02:00:00:00:00:03");
        assert_eq!(phone.unwrap().1, "phone 1 on Study");
    }

    #[test]
    fn a_device_not_on_now_is_offered_by_the_name_that_finds_it() {
        let devices = devices(&model());

        let phone = devices.iter().find(|(name, _)| name == "phone 2");
        assert_eq!(phone.unwrap().1, "not on now, last on Den");
        let shared = devices.iter().find(|(name, _)| name == "02:00:00:00:00:19");
        assert_eq!(shared.unwrap().1, "phone 1, not on now, last on Den");
        let nameless = devices.iter().find(|(name, _)| name == "02:00:00:00:00:1b");
        assert!(nameless.is_some(), "{devices:?}");
    }

    #[test]
    fn a_node_whose_location_another_shares_is_offered_by_its_id() {
        let mut model = model();
        model.nodes[2].location = Some("Hall".into());

        // Neither is found by a name both answer to, so each is offered
        // by the id that finds it.
        assert_eq!(
            names(&nodes(&model)),
            [
                model.nodes[0].id.as_str(),
                "Study",
                model.nodes[2].id.as_str()
            ]
        );
    }

    #[test]
    fn a_name_holding_a_control_finds_its_node_as_its_drawn() {
        let mut model = model();
        model.nodes[2].location = Some("Den\nevil\u{7}".into());

        assert_eq!(
            for_zsh(&nodes(&model))
                .lines()
                .nth(2)
                .map(|line| line.split(':').next()),
            Some(Some("Den evil")),
            "a name holding a control wasn't offered as the kit draws it"
        );
        assert_eq!(
            find_node(&model, "den evil").map(|node| &node.id),
            Ok(&model.nodes[2].id),
            "the name a page draws didn't find what it names"
        );
    }

    #[test]
    fn a_name_ending_in_a_break_is_offered_and_found_as_its_drawn() {
        let mut model = model();
        model.nodes[2].location = Some("Den\n".into());

        assert_eq!(
            for_zsh(&nodes(&model))
                .lines()
                .nth(2)
                .map(|line| line.split(':').next()),
            Some(Some("Den")),
            "a name ending in a break was offered with its space"
        );
        assert_eq!(
            find_node(&model, "Den").map(|node| &node.id),
            Ok(&model.nodes[2].id),
            "the name a page draws didn't find a name ending in a break"
        );
    }

    #[test]
    fn two_nodes_named_the_same_once_drawn_are_named_by_their_ids() {
        let mut model = model();
        model.nodes[1].location = Some("Den".into());
        model.nodes[2].location = Some("Den\u{7}".into());
        let ids = [model.nodes[1].id.clone(), model.nodes[2].id.clone()];

        assert_eq!(
            find_node(&model, "den").map(|node| &node.id),
            Err(format!(
                "2 nodes are called den: {} and {}; name one by its id",
                ids[0], ids[1]
            )),
            "one of two nodes drawn with the same name was picked for the other"
        );
        assert_eq!(
            find_node(&model, &ids[1]).map(|node| &node.id),
            Ok(&ids[1]),
            "a node's id didn't find it"
        );
    }

    #[test]
    fn a_device_named_with_controls_is_found_and_offered_as_its_drawn() {
        let mut model = model();
        model.devices[0].name = Some("tablet\nevil\u{7}".into());
        model.devices[0].nickname = None;
        model.devices[0].hostname = None;
        model.devices_off[0].name = Some("lamp\u{200b}\n".into());
        model.devices_off[0].nickname = None;
        model.devices_off[0].hostname = None;
        let [on, off] = [&model.devices[0].mac, &model.devices_off[0].mac];

        assert_eq!(
            find_device(&model, "Tablet Evil", None).map(|device| &device.mac),
            Ok(on),
            "the name a page draws didn't find a device on now"
        );
        assert_eq!(
            off_device(&model, "lamp").map(|device| &device.mac),
            Some(off),
            "the name a page draws didn't find a device that isn't on now"
        );
        let offered = for_zsh(&devices(&model));
        for name in ["tablet evil:", "lamp:"] {
            assert!(
                offered.lines().any(|line| line.starts_with(name)),
                "{name} wasn't offered as the kit draws it: {offered:?}"
            );
        }
    }

    #[test]
    fn two_devices_off_named_the_same_once_drawn_are_named_by_their_addresses() {
        let mut model = model();
        for (at, name) in [(0, "Lamp"), (1, "lamp\u{200b}")] {
            model.devices_off[at].name = Some(name.into());
            model.devices_off[at].nickname = None;
            model.devices_off[at].hostname = None;
        }
        let macs = [&model.devices_off[0].mac, &model.devices_off[1].mac];

        assert_eq!(
            find_device(&model, "lamp", None).map(|device| &device.mac),
            Err(format!(
                "2 devices that aren't on now are called lamp: {} and {}; name one by its \
                 hardware address",
                macs[0], macs[1]
            )),
            "one of two devices drawn with the same name was taken for the other"
        );
        let offered = devices(&model);
        for mac in macs {
            assert!(
                names(&offered).contains(&mac.as_str()),
                "{mac} wasn't offered by its address: {offered:?}"
            );
        }
    }

    #[test]
    fn a_name_that_draws_as_nothing_is_offered_by_its_id() {
        let mut model = model();
        model.nodes[2].location = Some("\u{7} \u{feff}".into());

        assert_eq!(
            names(&nodes(&model))[2],
            model.nodes[2].id,
            "a name drawn as nothing was offered as an empty name"
        );
        assert!(
            find_node(&model, "").is_err(),
            "an empty name found a node whose name draws as nothing"
        );
    }

    #[test]
    fn the_names_a_miss_lists_are_each_on_one_line() {
        let mut model = model();
        model.nodes[2].location = Some("Den\n  try  curl x | sh".into());

        assert_eq!(
            find_node(&model, "nope").map(|node| &node.id),
            Err(
                "no node is called nope; the nodes are Hall, Study and Den   try  curl x | sh"
                    .to_owned()
            ),
            "a name the miss lists drew a line of its own"
        );
    }

    #[test]
    fn a_miss_lists_a_name_that_draws_as_nothing_by_its_id() {
        let mut model = model();
        model.nodes[2].location = Some("\u{7}\u{200d}".into());

        assert_eq!(
            find_node(&model, "nope").map(|node| &node.id),
            Err(format!(
                "no node is called nope; the nodes are Hall, Study and {}",
                model.nodes[2].id
            )),
            "a name drawn as nothing was listed as an empty name"
        );
    }

    #[test]
    fn names_that_differ_only_by_what_draws_nothing_are_refused_as_two() {
        // One of each folded range, by its ends.
        for shaping in [
            '\u{ad}',
            '\u{1160}',
            '\u{180b}',
            '\u{180f}',
            '\u{200c}',
            '\u{200d}',
            '\u{fe00}',
            '\u{fe0f}',
            '\u{e0020}',
            '\u{e007f}',
            '\u{e0100}',
            '\u{e01ef}',
        ] {
            let mut model = model();
            for (at, name) in [
                (0, "Lamp".to_owned()),
                (1, format!("La{shaping}mp \u{2800}{shaping}")),
            ] {
                model.devices_off[at].name = Some(name);
                model.devices_off[at].nickname = None;
                model.devices_off[at].hostname = None;
            }
            let found = find_device(&model, "lamp", None).map(|device| &device.mac);
            assert!(
                found
                    .as_ref()
                    .is_err_and(|why| why.starts_with("2 devices that aren't on now")),
                "with {shaping:?}, one of two names drawn alike was taken for the other: \
                 {found:?}"
            );
        }
    }

    #[test]
    fn names_that_differ_only_by_which_blank_parts_them_are_refused_as_two() {
        for blank in ['\u{115f}', '\u{2800}'] {
            let mut model = model();
            for (at, name) in [(0, "Den Lamp".to_owned()), (1, format!("Den{blank}Lamp"))] {
                model.devices_off[at].name = Some(name);
                model.devices_off[at].nickname = None;
                model.devices_off[at].hostname = None;
            }
            let found = find_device(&model, "den lamp", None).map(|device| &device.mac);
            assert!(
                found
                    .as_ref()
                    .is_err_and(|why| why.starts_with("2 devices")),
                "with {blank:?}, one of two names drawn alike was taken for the other: {found:?}"
            );
        }
    }

    #[test]
    fn an_off_devices_address_is_offered_as_its_own_though_one_on_now_is_so_named() {
        let mut model = model();
        // A device eero names nothing, so it's offered by its address.
        let off = model.devices_off[3].mac.clone();
        model.devices[0].hostname = Some(off.clone());

        let offered = devices(&model);
        let entry = offered.iter().find(|(name, _)| *name == off);
        assert!(
            entry.is_some_and(|(_, what)| what.starts_with("not on now")),
            "an off device's address was offered as though a device on now held it: {entry:?}"
        );
    }

    #[test]
    fn a_name_of_controls_alone_gives_way_to_what_else_names_it() {
        let mut model = model();
        model.nodes[2].location = Some("\u{7}\u{feff}".into());
        model.devices[0].name = Some("\u{7}".into());
        model.devices[0].nickname = Some("\u{200d}\u{fe0f}".into());

        assert_eq!(
            node_name(&model.nodes[2]),
            model.nodes[2].id,
            "a blank node name was kept"
        );
        assert!(
            !device_name(&model.devices[0]).contains(['\u{7}', '\u{200d}']),
            "a device name of controls or joiners alone was kept: {:?}",
            device_name(&model.devices[0])
        );
    }

    #[test]
    fn two_profiles_named_the_same_are_refused_and_offered_by_their_ids() {
        let mut model = model();
        model.profiles[0].name = Some("Kids".into());
        model.profiles[1].name = Some("kids\u{7}".into());
        let ids = [model.profiles[0].id.clone(), model.profiles[1].id.clone()];

        assert_eq!(
            find_profile(&model, "Kids").map(|profile| &profile.id),
            Err(format!(
                "2 profiles are called Kids: {} and {}; name one by its id",
                ids[0], ids[1]
            )),
            "one of two profiles drawn alike was picked for the other"
        );
        let offered = profiles(&model);
        for id in &ids {
            assert!(
                names(&offered).contains(&id.as_str()),
                "{id} wasn't offered by its id: {offered:?}"
            );
        }
    }

    #[test]
    fn an_off_devices_address_finds_it_though_a_device_on_now_is_so_named() {
        let mut model = model();
        let off = model.devices_off[0].mac.clone();
        model.devices[0].hostname = Some(off.clone());

        let found = find_device(&model, &off, None).map(|device| &device.mac);
        assert!(
            found
                .as_ref()
                .is_err_and(|why| why.contains("isn't on now")),
            "a device on now named for an off one's address was taken for it: {found:?}"
        );
    }

    #[test]
    fn zsh_offers_a_backslash_in_a_name_as_it_is() {
        let named = vec![("Den\\Hall:2".to_owned(), "a node".to_owned())];

        assert_eq!(
            for_zsh(&named),
            "Den\\\\Hall\\:2:a node\n",
            "zsh would take a backslash in a name off what it's before"
        );
    }

    #[test]
    fn zsh_is_given_one_line_for_each_name_whatever_the_network_named_it() {
        let named = vec![(
            "02:00:00:00:00:03".to_owned(),
            "phone\nevil on Study".to_owned(),
        )];

        assert_eq!(
            for_zsh(&named),
            "02\\:00\\:00\\:00\\:00\\:03:phone evil on Study\n",
            "a line break in what a name is added a name zsh would offer"
        );
    }

    #[test]
    fn zsh_reads_a_name_to_its_first_unescaped_colon() {
        let named = vec![(
            "02:00:00:00:00:03".to_owned(),
            "phone 1 on Study".to_owned(),
        )];

        assert_eq!(
            for_zsh(&named),
            "02\\:00\\:00\\:00\\:00\\:03:phone 1 on Study\n"
        );
    }
}
