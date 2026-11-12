//! What completion offers for a view's word: the views themselves, and the
//! home network's nodes, devices and profiles by the names each view finds
//! them by, each with a few words saying which it is.
//!
//! A name is offered only when it finds the thing it names, so where two
//! devices share one, each is offered by its hardware address instead,
//! which is what the view would ask for.

use super::{
    VIEWS, device_name, find_device, find_node, find_profile, location, node_name, off_device,
    off_name, on_now_answers_to, profile_name,
};
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
/// colons escaped, since a colon ends it, then a colon and what it is.
pub fn for_zsh(names: &[Named]) -> String {
    let mut out = String::new();
    for (name, what) in names {
        out.push_str(&name.replace(':', "\\:"));
        out.push(':');
        out.push_str(what);
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

        assert_eq!(
            names(&nodes(&model)),
            ["Hall", "Study", model.nodes[2].id.as_str()]
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
