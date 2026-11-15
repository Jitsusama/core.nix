//! The cases for what a screen's part answers, asked by its name from
//! the top: the focused screen's part, then each other lit screen's,
//! behind a key no row is.

use super::*;

/// The rows behind the answer to part `id`, in the order served.
fn answered<'a>(made: &'a [Value], id: &str) -> Vec<&'a Value> {
    let within = key(&format!("answer:{id}"));
    made.iter()
        .filter(|row| row["parent"] == within.as_str())
        .collect()
}

/// The keys of the parts the answer to `id` stands for, in its order.
fn parts<'a>(made: &'a [Value], id: &str) -> Vec<&'a str> {
    answered(made, id)
        .into_iter()
        .filter_map(|row| row["held"]["part"].as_str())
        .collect()
}

#[test]
fn a_part_asked_by_name_answers_the_focused_screens_first_then_each_other_lit_ones() {
    let none = BTreeMap::new();
    let made = every(&desk(), &none, Some("HDMI-A-1"), &Light::default());
    assert_eq!(
        parts(&made, "scale"),
        [key("setting:HDMI-A-1:scale"), key("setting:eDP-1:scale")],
        "the focused screen's scale doesn't lead, or a screen niri lays nothing out on answers"
    );
    let made = every(&desk(), &none, None, &Light::default());
    assert_eq!(
        parts(&made, "scale"),
        [key("setting:eDP-1:scale"), key("setting:HDMI-A-1:scale")],
        "with no screen focused, the built-in one's doesn't lead"
    );
    for row in answered(&made, "scale") {
        let screen = key(&format!(
            "display:{}",
            row["held"]["output"].as_str().unwrap_or("")
        ));
        let named = made.iter().find(|one| one["key"] == screen.as_str());
        let part = made.iter().find(|one| one["key"] == row["held"]["part"]);
        assert_eq!(
            Some(&row["name"]),
            named.map(|one| &one["name"]),
            "an answer's row isn't named for its screen as the screen's own row is: {row}"
        );
        assert_eq!(
            Some(&row["side"]),
            part.map(|one| &one["side"]),
            "an answer's row doesn't say what its part says: {row}"
        );
        assert_eq!(
            row["acts"],
            json!([{"key": "verb:copy", "copy": row["side"][0]["said"]}]),
            "an answer's row doesn't copy what it says"
        );
    }
}

#[test]
fn a_part_only_one_screen_has_answers_that_ones_alone() {
    let desk = BTreeMap::from([("HDMI-A-1".to_owned(), monitor())]);
    let made = every(&super::desk(), &desk, Some("eDP-1"), &Light::default());
    assert_eq!(
        parts(&made, "white"),
        [key("fact:HDMI-A-1:white")],
        "a screen whose EDID says no white point answers for it"
    );
    assert!(
        answered(&made, "warmth").is_empty(),
        "the Warmth, Screens' own, is answered as a screen's part"
    );
}
