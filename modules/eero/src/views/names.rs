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
    off_name, on_now_answers_to, profile_name,
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
            let name = node_name(node);
            let found = find_node(model, name).is_ok_and(|found| found.id == node.id);
            let what = [node.model.clone(), node.gateway.then(|| "gateway".into())]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ");
            (if found { name } else { &node.id }.to_owned(), what)
        })
        .collect()
}

/// Each device on now by its name, or its hardware address where another
/// shares the name, then each eero knows that isn't on now the same way,
/// since `eero device` says where and when that one was last on.
pub fn devices(model: &HomeNetwork) -> Vec<Named> {
    let on = model.devices.iter().map(|device| {
        let name = device_name(device);
        let found = find_device(model, &name, None).is_ok_and(|found| found.mac == device.mac);
        let on = location(model, device.node.as_deref());
        if found {
            (name.into_owned(), format!("on {on}"))
        } else {
            (device.mac.clone(), format!("{name} on {on}"))
        }
    });
    let off = model.devices_off.iter().map(|device| {
        let name = off_name(device);
        let found = !on_now_answers_to(model, &name)
            && off_device(model, &name).is_some_and(|found| found.mac == device.mac);
        let last = match device.node.as_deref() {
            Some(node) => format!("not on now, last on {}", location(model, Some(node))),
            None => "not on now".to_owned(),
        };
        if found {
            (name.into_owned(), last)
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
            let name = profile_name(profile);
            let found = find_profile(model, name).is_ok_and(|found| found.id == profile.id);
            (
                if found { name } else { &profile.id }.to_owned(),
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
