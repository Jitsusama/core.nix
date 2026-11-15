//! The shape each orchestration is drawn by, as the node serves it on its
//! team's record (mycelium's `glyph`), named as the theme's mark for it, so
//! an orchestration's row wears the shape the map draws its network in,
//! in place of the agent's mark (VKCF3Z, 2026-11-14). The node chooses the
//! shapes, so every view of the realm agrees without keeping any here.

use std::collections::HashMap;

use serde_json::Value;

/// The theme's mark for a shape the node serves, or none for one the theme
/// has no mark for, whose row keeps the agent's mark. The down triangle's is
/// the triangle turned a half turn, since the icon font draws none at the
/// others' size.
#[must_use]
pub fn mark(shape: &str) -> Option<&'static str> {
    match shape {
        "square" => Some("shapeSquare"),
        "triangle" => Some("shapeTriangle"),
        "plus" => Some("shapePlus"),
        "hexagon" => Some("shapeHexagon"),
        "circle" => Some("shapeCircle"),
        "down-triangle" => Some("shapeDownTriangle"),
        "star" => Some("shapeStar"),
        _ => None,
    }
}

/// Each team's mark, by pi's id for the session leading it, from the
/// node's `team` records; a team no session leads, or drawn by no shape
/// the theme marks, has none.
pub fn by_lead<'a>(teams: impl Iterator<Item = &'a Value>) -> HashMap<&'a str, &'static str> {
    teams
        .filter_map(|team| Some((team["lead"].as_str()?, mark(team["glyph"].as_str()?)?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn each_team_led_and_drawn_by_a_marked_shape_is_known_by_its_lead() {
        let teams = [
            json!({"key": "team:HARBOR", "lead": "s-harbor", "glyph": "square"}),
            json!({"key": "team:KELP01", "lead": "s-kelp", "glyph": "star"}),
            json!({"key": "team:REEF01", "glyph": "plus"}),
            json!({"key": "team:TIDE01", "lead": "s-tide"}),
            json!({"key": "team:GULL01", "lead": "s-gull", "glyph": "down-triangle"}),
            json!({"key": "team:CRAB01", "lead": "s-crab", "glyph": "spiral"}),
        ];
        let marks = by_lead(teams.iter());
        assert_eq!(
            marks,
            HashMap::from([
                ("s-harbor", "shapeSquare"),
                ("s-kelp", "shapeStar"),
                ("s-gull", "shapeDownTriangle")
            ]),
            "none for a team no one leads, one drawn by no shape, or one the theme can't mark"
        );
    }

    #[test]
    fn every_shape_the_map_draws_has_its_own_mark() {
        let shapes = [
            "square",
            "triangle",
            "plus",
            "hexagon",
            "circle",
            "down-triangle",
            "star",
        ];
        let marks: Vec<_> = shapes.iter().filter_map(|shape| mark(shape)).collect();
        assert_eq!(
            marks,
            [
                "shapeSquare",
                "shapeTriangle",
                "shapePlus",
                "shapeHexagon",
                "shapeCircle",
                "shapeDownTriangle",
                "shapeStar"
            ],
            "one mark each, in the map's order"
        );
        assert_eq!(mark("spiral"), None, "a shape the theme has no mark for");
    }
}
