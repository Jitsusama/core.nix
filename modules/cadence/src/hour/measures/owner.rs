//! The address a measure's line goes to, from the owner field a record
//! keeps by hand: the orchestrator's asks and Joel's misses name the one
//! owning it first, then those helping, in whatever shape was typed.

use super::cells::short_id;

/// The first address in an owner field such as `PN2VZC (H6EDLC)`, which
/// names the one owning it before those helping. Owners listed with a
/// comma, slash, semicolon or bracket give the first, and a quest named by
/// its id gives its short id, which is the address its cell answers to.
pub fn first_owner(owner: &str) -> String {
    let first = owner
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '/' | ';' | '('))
        .find(|name| !name.is_empty())
        .unwrap_or_default();
    if first.starts_with("QEST-") {
        short_id(first).to_owned()
    } else {
        first.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::first_owner;

    #[test]
    fn the_owner_is_the_first_name_before_those_helping() {
        assert_eq!(first_owner("PN2VZC (H6EDLC)"), "PN2VZC");
        assert_eq!(first_owner("16V6F4"), "16V6F4");
        assert_eq!(first_owner(""), "");
    }

    #[test]
    fn a_list_of_owners_gives_the_first_without_its_mark() {
        // 19 November's asks name owners as "Y2YBTS, 0IM95G",
        // "XUBK2E/H2VYOB" and "5FWIUS; WCJQSG", which the mesh can't route.
        assert_eq!(first_owner("Y2YBTS, 0IM95G"), "Y2YBTS");
        assert_eq!(first_owner("XUBK2E/H2VYOB"), "XUBK2E");
        assert_eq!(first_owner("5FWIUS; WCJQSG"), "5FWIUS");
        assert_eq!(first_owner("0FX11G,OSPHT4"), "0FX11G");
        assert_eq!(first_owner("PN2VZC(H6EDLC)"), "PN2VZC");
        assert_eq!(
            first_owner(" , Y2YBTS"),
            "Y2YBTS",
            "a leading mark names no one"
        );
    }

    #[test]
    fn a_quest_named_by_its_id_gives_its_cells_address() {
        // 31 of 155 asks named their owner as a quest's id, and the hour
        // sent its slips to "QEST-20261115-VRZKID", which no cell answers.
        assert_eq!(first_owner("QEST-20261115-VRZKID"), "VRZKID");
        assert_eq!(first_owner("QEST-20261116-6LU6ZM (PN2VZC)"), "6LU6ZM");
        assert_eq!(
            first_owner("cadence-day"),
            "cadence-day",
            "only a quest's id is cut"
        );
    }
}
