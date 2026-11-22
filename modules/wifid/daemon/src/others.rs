//! What becomes of a card's other networks as wifid gives it its networks:
//! whether they're turned on beside the one it's on, or kept off while a
//! move, Joel's pick or his leaving a network holds the card to its own.

use crate::supplicant::holds_a_network;

/// What becomes of a card's other networks as it's given its networks.
#[derive(Debug, PartialEq, Eq)]
pub enum Others {
    /// Left as they are, off while a move or Joel's pick keeps them so.
    Kept,
    /// Left as they are, and marked to come on once the card's next try
    /// is taken off, as Joel left it between tries.
    KeptTillTakenOff,
    /// Turned on.
    On,
}

/// Whether a card's other networks stay off as it's given its networks:
/// only while it holds the one Joel picked in the launcher. A card on a
/// network it chose itself has every network on, so a network newly saved
/// or ranked above it can still win its next look.
fn keeps_the_others_off(picked: bool, step: &str) -> bool {
    picked && holds_a_network(step)
}

/// What becomes of a card's other networks as it's given its networks: a
/// card in a move (`alone`) keeps them off until the move ends, and one
/// Joel left between tries (`leaving`) keeps them as they are until its
/// next try is taken off, as turned on its next try could go to any
/// network it hears. Otherwise they're off only while it holds the one
/// he picked (`keeps_the_others_off`).
pub fn others_given(alone: bool, leaving: bool, picked: bool, step: &str) -> Others {
    if alone {
        Others::Kept
    } else if leaving {
        Others::KeptTillTakenOff
    } else if keeps_the_others_off(picked, step) {
        Others::Kept
    } else {
        Others::On
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_card_on_the_network_joel_picked_keeps_the_others_off() {
        assert!(
            keeps_the_others_off(true, "completed"),
            "a card on the network Joel picked keeps the others off"
        );
        assert!(
            !keeps_the_others_off(false, "completed"),
            "a card on a network it chose itself has a newly saved one on"
        );
        assert!(
            !keeps_the_others_off(true, "disconnected"),
            "a card come off the network it was given has every network on"
        );
    }

    #[test]
    fn a_card_in_a_move_or_left_between_tries_keeps_its_networks_as_they_are() {
        assert_eq!(
            others_given(true, false, false, "disconnected"),
            Others::Kept,
            "a card in a move keeps them off till the move ends"
        );
        assert_eq!(
            others_given(true, false, true, "disconnected"),
            Others::Kept,
            "a move outranks a pick come off its network"
        );
        assert_eq!(
            others_given(false, true, false, "disconnected"),
            Others::KeptTillTakenOff,
            "off every network and in no pick, a card Joel left isn't given the others"
        );
        assert_eq!(
            others_given(true, true, false, "disconnected"),
            Others::Kept,
            "one in a move is already marked to have them on as it ends"
        );
        assert_eq!(
            others_given(false, false, true, "completed"),
            Others::Kept,
            "one on the network Joel picked keeps them off"
        );
        assert_eq!(
            others_given(false, false, false, "disconnected"),
            Others::On,
            "any other is given every network"
        );
    }
}
