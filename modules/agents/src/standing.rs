//! How a pi session stands, and what that lets Joel do with it, said once
//! for every place the launcher shows a session (Joel, 2026-10-28: "we
//! need to not have different versions of these sort of elements. they
//! need to be treated the same everywhere").

use serde_json::Value;

use crate::said::Said;

/// How a session can stand, in the one order every list of sessions
/// keeps: what only Joel can move first, then what's under way, then
/// what waits for work, then what he put down, then what gave its memory
/// back until it's wanted (the harness's park-workflow), and last one
/// with no presence at all. One idle is listening, never waiting, which
/// is only ever said of something waiting on Joel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stands {
    Needs,
    Working,
    InLine,
    Listening,
    Paused,
    Parked,
    Off,
}

/// Every way a running session stands, in their order.
pub const RUNNING: [Stands; 6] = [
    Stands::Needs,
    Stands::Working,
    Stands::InLine,
    Stands::Listening,
    Stands::Paused,
    Stands::Parked,
];

impl Stands {
    /// Its id, as the launcher and the vocabulary name it.
    pub fn id(self) -> &'static str {
        match self {
            Self::Needs => "needs",
            Self::Working => "working",
            Self::InLine => "inLine",
            Self::Listening => "listening",
            Self::Paused => "paused",
            Self::Parked => "parked",
            Self::Off => "off",
        }
    }

    /// Whether it only waits, so an orchestration may fold it away.
    pub fn waits(self) -> bool {
        self >= Self::Listening
    }
}

/// How a session stands, from its presence, whether its want is Joel's
/// (`asks`) and whether it waits in the queue (`queued`). A want not his
/// reads as what it does; one parked stays parked, its want For You's,
/// since Wake is its only act.
pub fn state(presence: Option<&Value>, asks: bool, queued: bool) -> Stands {
    let Some(presence) = presence.filter(|presence| !presence.is_null()) else {
        return Stands::Off;
    };
    let said = presence.get("state").and_then(Value::as_str).unwrap_or("");
    if said == "parked" {
        Stands::Parked
    } else if asks {
        Stands::Needs
    } else if said == "paused" {
        Stands::Paused
    } else if queued {
        Stands::InLine
    } else if truthy(presence.get("busy")) || said == "busy" {
        Stands::Working
    } else {
        Stands::Listening
    }
}

/// Whether a value is set, as the harness's JavaScript reads one.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(set)) => *set,
        Some(Value::Number(number)) => number.as_f64().is_some_and(|n| n != 0.0),
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
    }
}

/// The acts a running session offers, by their vocabulary's ids, in the
/// order every kind keeps (docs/design.md, the order of actions): what
/// leaves it as it is before what changes it, and what ends it last.
/// Every session on the mesh reads its steer inbox, so each can be talked
/// to, paused, resumed and stopped; one with no window offers no Go To,
/// and only one under way can be stopped. One parked has no process to
/// pause or stop and no window known to go to: it can be woken, or talked
/// to, which wakes it too. Only one here can be closed, last, since the
/// harness ends a session on a close Joel steers it from this machine and
/// drops one a node relays (harness #1023).
pub fn acts(stands: Stands, windowed: bool, here: bool) -> Vec<&'static str> {
    if stands == Stands::Parked {
        return vec!["wake", "talk"];
    }
    let mut ids = Vec::with_capacity(5);
    if windowed {
        ids.push("switch");
    }
    ids.push("talk");
    ids.push(if stands == Stands::Paused {
        "resume"
    } else {
        "pause"
    });
    if matches!(stands, Stands::Working | Stands::InLine) {
        ids.push("stop");
    }
    if here {
        ids.push("close");
    }
    ids
}

/// The act Return does, one of `acts`: the one its state makes harmless
/// (VKCF3Z, 2026-11-06). Go To one at work or waiting on Joel, Talk to
/// one listening, Resume one paused and Wake one parked; one with no
/// window to go to is talked to. Stop and Close end it, so they are never
/// Return's.
pub fn returns(stands: Stands, windowed: bool) -> &'static str {
    match stands {
        Stands::Parked => "wake",
        Stands::Paused => "resume",
        Stands::Listening => "talk",
        _ if windowed => "switch",
        _ => "talk",
    }
}

/// The theme's role its glyph is drawn in: a program's, as a live one
/// is, and muted for one paused, parked or closed, which never looks live
/// (#2052; VKCF3Z, 2026-11-07). A live one with a tier 2 figure is nearing
/// a limit, in the warning's role, as a budget near its allowance is.
pub fn hue(stands: Stands, warns: bool) -> &'static str {
    match stands {
        Stands::Paused | Stands::Parked | Stands::Off => "muted",
        _ if warns => "warning",
        _ => "program",
    }
}

/// What Close's ask says stays, so ending a session never reads as losing
/// its work (VKCF3Z, 2026-11-06: "Close 8IDU4E? Its worktree and 1 to-do
/// stay."): its worktree, or its folder where it works outside one, and
/// its quest's to-dos, by their count.
pub fn leaves(in_tree: bool, needs: usize, said: &Said) -> String {
    let place = said.word(if in_tree { "tree" } else { "folder" });
    if needs == 0 {
        return said.fill("stays", &[("kept", place)]);
    }
    let count = needs.to_string();
    let items = said.fill(
        if needs == 1 { "toDo" } else { "toDos" },
        &[("count", &count)],
    );
    let kept = format!("{place} {} {items}", said.word("and"));
    said.fill("stay", &[("kept", &kept)])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_session_stands_as_its_presence_says() {
        let cases = [
            (
                "one wanting Joel needs him",
                json!({"busy": true}),
                true,
                false,
                "needs",
            ),
            (
                "a want that isn't his reads as what it does",
                json!({"busy": true}),
                false,
                false,
                "working",
            ),
            (
                "one at work is working",
                json!({"busy": true}),
                false,
                false,
                "working",
            ),
            (
                "one waiting its turn is in line",
                json!({"busy": true}),
                false,
                true,
                "inLine",
            ),
            (
                "one idle is listening, never waiting",
                json!({"busy": false}),
                false,
                false,
                "listening",
            ),
            (
                "one the harness says is waiting is still only listening",
                json!({"busy": false, "state": "waiting"}),
                false,
                false,
                "listening",
            ),
            (
                "one paused is paused",
                json!({"state": "paused"}),
                false,
                true,
                "paused",
            ),
            (
                "one paused that wants Joel needs him",
                json!({"state": "paused"}),
                true,
                false,
                "needs",
            ),
            (
                "one parked is parked, never listening",
                json!({"state": "parked"}),
                false,
                false,
                "parked",
            ),
            (
                "one parked that wants Joel is still parked",
                json!({"state": "parked"}),
                true,
                false,
                "parked",
            ),
            ("none at all is off", Value::Null, false, false, "off"),
        ];
        for (what, presence, asks, queued, want) in cases {
            assert_eq!(state(Some(&presence), asks, queued).id(), want, "{what}");
        }
        assert_eq!(
            state(None, true, true),
            Stands::Off,
            "one with no presence is off"
        );
    }

    #[test]
    fn every_list_keeps_one_order_of_states() {
        let mut order = [
            Stands::Paused,
            Stands::Parked,
            Stands::Listening,
            Stands::Off,
            Stands::InLine,
            Stands::Needs,
            Stands::Working,
        ];
        order.sort();
        let said: Vec<_> = order.iter().map(|stands| stands.id()).collect();
        assert_eq!(
            said.join(" "),
            "needs working inLine listening paused parked off"
        );
    }

    #[test]
    fn a_sessions_acts_follow_how_it_stands() {
        let cases = [
            (Stands::Working, true, true, "switch talk pause stop close"),
            (Stands::Working, true, false, "switch talk pause stop"),
            (Stands::InLine, false, true, "talk pause stop close"),
            (Stands::Listening, true, true, "switch talk pause close"),
            (Stands::Needs, true, false, "switch talk pause"),
            (Stands::Paused, true, true, "switch talk resume close"),
            (Stands::Parked, false, true, "wake talk"),
        ];
        for (stands, windowed, here, want) in cases {
            let got = acts(stands, windowed, here).join(" ");
            assert_eq!(got, want, "the acts of one {}, here {here}", stands.id());
        }
    }

    #[test]
    fn return_does_the_act_its_state_makes_harmless() {
        let cases = [
            (Stands::Working, true, "switch"),
            (Stands::InLine, true, "switch"),
            (Stands::Needs, true, "switch"),
            (Stands::Working, false, "talk"),
            (Stands::Listening, true, "talk"),
            (Stands::Paused, true, "resume"),
            (Stands::Paused, false, "resume"),
            (Stands::Parked, false, "wake"),
        ];
        for (stands, windowed, want) in cases {
            let got = returns(stands, windowed);
            assert_eq!(
                got,
                want,
                "Return on one {}, windowed {windowed}",
                stands.id()
            );
            let offered = acts(stands, windowed, true);
            assert!(
                offered.contains(&got),
                "Return on one {} isn't among its acts",
                stands.id()
            );
        }
    }

    #[test]
    fn one_not_live_never_looks_live() {
        let hues: Vec<_> = RUNNING
            .iter()
            .chain([Stands::Off].iter())
            .map(|stands| format!("{}:{}", stands.id(), hue(*stands, false)))
            .collect();
        let want = "needs:program working:program inLine:program listening:program \
                    paused:muted parked:muted off:muted";
        assert_eq!(hues.join(" "), want);
        let warned: Vec<_> = [
            Stands::Working,
            Stands::Listening,
            Stands::Paused,
            Stands::Off,
        ]
        .iter()
        .map(|stands| format!("{}:{}", stands.id(), hue(*stands, true)))
        .collect();
        let want = "working:warning listening:warning paused:muted off:muted";
        assert_eq!(
            warned.join(" "),
            want,
            "a tier 2 figure warns only on one live"
        );
    }

    #[test]
    fn close_says_what_stays() {
        let said =
            Said::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read");
        let cases = [
            (true, 0, "its worktree stays."),
            (true, 1, "its worktree and 1 to-do stay."),
            (false, 2, "its folder and 2 to-dos stay."),
            (false, 0, "its folder stays."),
        ];
        for (in_tree, needs, want) in cases {
            let got = leaves(in_tree, needs, &said);
            assert_eq!(
                got, want,
                "what Close leaves, in a tree {in_tree}, with {needs}"
            );
        }
    }
}
