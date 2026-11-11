//! What a budget's drawer and a window's say, served ready on their rows
//! (collection.rs), from how each stands (standing.rs), so the launcher
//! only names each fact by its id and draws it. By the drawer's rules
//! (VKCF3Z, 2026-11-07): no state in an eyebrow, nothing the row's side or
//! the title already says, what moves most first and identifiers last;
//! and a quiet window's, drawn while the screen is shared, holds no dollar
//! figure, as its row holds none.

use crate::collection::{Drawn, Fact, Figure};
use crate::standing::{Allowance, Budget};

const DOT: &str = " \u{b7} ";

/// What a window not read yet says beside its name.
const UNREAD: &str = "not read yet";

/// How near a window is, as the launcher's meter draws it: in the
/// warning's ink only from where it warns, as Context's is from 80%. None
/// for one not read yet.
fn stroke(allowance: &Allowance) -> Option<Figure> {
    allowance.nearest.map(|near| Figure {
        shape: "stroke",
        values: vec![near.clamp(0.0, 1.0)],
        warn: near >= allowance.warn,
    })
}

/// Whether a window counts dollars that go while the screen is shared.
fn hidden(allowance: &Allowance, quiet: bool) -> bool {
    quiet && allowance.unit != "share"
}

/// A window's facts beyond its name, in the order they move: what its feed
/// says, which is the one thing to act on, while it says anything; its
/// standing, where `standing` asks for it; when it resets, where `resets`
/// does; when an override of it ends and how it last changed; whose
/// reading it is and when; and where it warns. A share's row already says
/// how much, so it has no Standing and no meter: one beside Resets would
/// only say the row's share again (VKCF3Z, 2026-11-08). Dollars go while
/// the screen is shared, their meter with them.
fn window_facts(allowance: &Allowance, quiet: bool, standing: bool, resets: bool) -> Vec<Fact> {
    let told = &allowance.said;
    let hides = hidden(allowance, quiet);
    let stands = standing && allowance.unit != "share" && !hides;
    let meter = if stands { stroke(allowance) } else { None };
    [
        ("state", &told.state, true),
        ("standing", &allowance.standing, stands),
        ("resets", &told.resets, resets),
        ("override", &told.ends, true),
        ("changed", &told.changed, !hides),
        ("read", &told.read, true),
        ("warns", &told.warns, !hides),
    ]
    .into_iter()
    .filter(|(_, value, shows)| *shows && !value.is_empty())
    .map(|(id, value, _)| Fact {
        figure: if id == "standing" {
            meter.clone()
        } else {
            None
        },
        ..Fact::of(id, value.as_str(), false)
    })
    .collect()
}

/// A budget's drawer: whose and what kind of money it is over its name,
/// "your own \u{b7} plan", in Label's capitals as our own words are; then
/// each window by its feed's name, its figure and when it resets with how
/// near it is, since its standing in full is the row's side; a budget of
/// one window says that window's other facts too, since it has no row of
/// its own to step into; then what it spent and what a plan's turns were
/// worth at list, how its feeds read, and the providers it always draws
/// for, each as it names itself, by its id where it doesn't (VKCF3Z,
/// 2026-11-09). No budget's row is quiet, so none leaves dollars out.
pub fn budget(budget: &Budget) -> Drawn {
    let windows = budget.allowances.iter().map(|one| {
        let value = [&one.said.brief, &one.standing]
            .into_iter()
            .find(|said| !said.is_empty())
            .map_or(UNREAD, String::as_str);
        Fact {
            label: one.label.clone(),
            figure: stroke(one),
            ..Fact::of("", value, false)
        }
    });
    // One window's reset is said once: beside its name in its brief, else
    // among its facts.
    let only = match budget.allowances.as_slice() {
        [one] => window_facts(one, false, false, one.said.brief.is_empty()),
        _ => Vec::new(),
    };
    let told = &budget.said;
    let providers = if told.providers.is_empty() {
        budget.providers.join(DOT)
    } else {
        told.providers.clone()
    };
    let more = [
        ("spent", &told.spent),
        ("worth", &told.worth),
        ("reading", &told.reading),
        ("providers", &providers),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_empty())
    .map(|(id, value)| Fact::of(id, value.as_str(), false));
    Drawn {
        eyebrow: if told.what.is_empty() {
            budget.words.clone()
        } else {
            told.what.clone()
        },
        title: budget.label.clone(),
        names: false,
        facts: windows.chain(only).chain(more).collect(),
    }
}

/// A window's drawer: where it is and what it is over its name, "in Your
/// AI proxy token \u{b7} window", set as it's written since it names
/// something (Drawer.qml); then its facts, its standing among them. Quiet,
/// it leaves its dollars out.
pub fn window(budget: &Budget, allowance: &Allowance, quiet: bool) -> Drawn {
    Drawn {
        eyebrow: if budget.label.is_empty() {
            String::new()
        } else {
            format!("in {}{DOT}window", budget.label)
        },
        title: allowance.label.clone(),
        names: true,
        facts: window_facts(allowance, quiet, true, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standing::{AllowanceSaid, BudgetSaid};

    /// A window as budgets.json says it, its facts in `said`.
    fn allowance(
        name: &str,
        unit: &'static str,
        near: Option<f64>,
        said: AllowanceSaid,
    ) -> Allowance {
        Allowance {
            name: name.to_lowercase().replace(' ', "-"),
            label: name.into(),
            unit,
            used: None,
            allowance: None,
            spent: None,
            window_hours: 24,
            warn: if unit == "share" { 0.8 } else { 0.9 },
            resets: None,
            state: None,
            from: None,
            at: None,
            standing: String::new(),
            nearest: near,
            said,
        }
    }

    fn of(name: &str, label: &str, allowances: Vec<Allowance>, said: BudgetSaid) -> Budget {
        Budget {
            name: name.into(),
            label: label.into(),
            kind: None,
            whose: None,
            default: false,
            undeclared: false,
            providers: vec!["claude-code".into()],
            spent: std::collections::BTreeMap::new(),
            value: None,
            allowances,
            standing: String::new(),
            nearest: None,
            health: None,
            words: String::new(),
            said,
        }
    }

    /// A plan of two share windows, one past where it warns and one not
    /// read yet.
    fn plan() -> Budget {
        let mut five = allowance(
            "Five Hours",
            "share",
            Some(0.85),
            AllowanceSaid {
                brief: "85% \u{b7} resets 16:20".into(),
                warns: "at 80%".into(),
                resets: "16:20".into(),
                read: "Claude at 14:20".into(),
                ..AllowanceSaid::default()
            },
        );
        five.standing = "85% of 5 h".into();
        let said = AllowanceSaid {
            warns: "at 80%".into(),
            ..AllowanceSaid::default()
        };
        let week = allowance("Week", "share", None, said);
        let told = BudgetSaid {
            what: "your own \u{b7} plan".into(),
            worth: "$84 today".into(),
            reading: "reading fine".into(),
            providers: "Claude Code".into(),
            ..BudgetSaid::default()
        };
        of("claude", "Claude Pro", vec![five, week], told)
    }

    /// The employer's money: a dollar window past where it warns, raised
    /// and overridden for a while, and one its feed refuses.
    fn proxy() -> Budget {
        let read = || "the AI proxy at 21:35".to_owned();
        let mut weekly = allowance(
            "Week",
            "dollars",
            Some(0.925),
            AllowanceSaid {
                warns: "at $36,000".into(),
                ends: "Wed 2 Dec \u{b7} a guess".into(),
                changed: "raised $30,000 to $40,000 \u{b7} Thursday 11:31".into(),
                read: read(),
                ..AllowanceSaid::default()
            },
        );
        weekly.standing = "$37,000 of $40,000".into();
        let mut premium = allowance(
            "Premium Day",
            "dollars",
            Some(0.0),
            AllowanceSaid {
                warns: "at $150".into(),
                resets: "05:00 tomorrow".into(),
                read: read(),
                state: "refusing requests".into(),
                ..AllowanceSaid::default()
            },
        );
        premium.standing = "$0 of $300".into();
        premium.warn = 0.5;
        let told = BudgetSaid {
            what: "the employer's \u{b7} money".into(),
            spent: "$3,075 today".into(),
            reading: "signed out".into(),
            ..BudgetSaid::default()
        };
        of("proxy", "Your AI proxy token", vec![weekly, premium], told)
    }

    /// A drawer's facts as one line: each name, its id where it has none,
    /// and value, its meter after, one past where it warns marked so.
    fn facts(drawn: &Drawn) -> String {
        let said: Vec<String> = drawn
            .facts
            .iter()
            .map(|fact| {
                let name = if fact.label.is_empty() {
                    &fact.id
                } else {
                    &fact.label
                };
                let meter = fact.figure.as_ref().map_or_else(String::new, |figure| {
                    format!(":{}{}", figure.shape, if figure.warn { "!" } else { "" })
                });
                format!("{name}={}{meter}", fact.value)
            })
            .collect();
        said.join("; ")
    }

    // A share window's row says how much, so its drawer has neither a
    // Standing nor a meter that would say it again; a dollar window says
    // its standing with its meter, and what its feed says comes first.
    #[test]
    fn a_window_s_drawer_says_what_its_row_doesn_t() {
        let plan = plan();
        let five = window(&plan, &plan.allowances[0], false);
        assert_eq!(five.eyebrow, "in Claude Pro \u{b7} window");
        assert!(five.names, "a window's eyebrow names its budget");
        assert_eq!(five.title, "Five Hours");
        assert_eq!(
            facts(&five),
            "resets=16:20; read=Claude at 14:20; warns=at 80%"
        );
        assert_eq!(
            facts(&window(&plan, &plan.allowances[1], false)),
            "warns=at 80%"
        );
        let proxy = proxy();
        assert_eq!(
            facts(&window(&proxy, &proxy.allowances[0], false)),
            "standing=$37,000 of $40,000:stroke!; override=Wed 2 Dec \u{b7} a guess; \
             changed=raised $30,000 to $40,000 \u{b7} Thursday 11:31; \
             read=the AI proxy at 21:35; warns=at $36,000"
        );
        assert_eq!(
            facts(&window(&proxy, &proxy.allowances[1], false)),
            "state=refusing requests; standing=$0 of $300:stroke; resets=05:00 tomorrow; \
             read=the AI proxy at 21:35; warns=at $150"
        );
    }

    // While the screen is shared a window's drawer holds no dollar figure,
    // and a dollar window's meter goes with its Standing.
    #[test]
    fn a_quiet_window_s_drawer_holds_no_dollars() {
        let plan = plan();
        let five = window(&plan, &plan.allowances[0], true);
        assert_eq!(
            facts(&five),
            "resets=16:20; read=Claude at 14:20; warns=at 80%"
        );
        let proxy = proxy();
        assert_eq!(
            facts(&window(&proxy, &proxy.allowances[0], true)),
            "override=Wed 2 Dec \u{b7} a guess; read=the AI proxy at 21:35"
        );
    }

    // A budget's drawer lists each window by its own name, its figure and
    // meter, then what the budget spent and how its feeds read; a budget
    // of one window says its facts too, its reset among them.
    #[test]
    fn a_budget_s_drawer_lists_its_windows() {
        let drawn = budget(&plan());
        assert_eq!(drawn.eyebrow, "your own \u{b7} plan");
        assert!(!drawn.names, "a budget's eyebrow is our own words");
        assert_eq!(drawn.title, "Claude Pro");
        assert_eq!(
            facts(&drawn),
            "Five Hours=85% \u{b7} resets 16:20:stroke!; Week=not read yet; \
             worth=$84 today; reading=reading fine; providers=Claude Code"
        );
        let mut proxy = proxy();
        proxy.providers.clear();
        assert_eq!(
            facts(&budget(&proxy)),
            "Week=$37,000 of $40,000:stroke!; Premium Day=$0 of $300:stroke; \
             spent=$3,075 today; reading=signed out"
        );
        proxy.allowances.remove(0);
        proxy.said.reading.clear();
        assert_eq!(
            facts(&budget(&proxy)),
            "Premium Day=$0 of $300:stroke; state=refusing requests; \
             resets=05:00 tomorrow; read=the AI proxy at 21:35; warns=at $150; \
             spent=$3,075 today"
        );
        // Its providers by their ids, where it doesn't name them.
        let mut unnamed = plan();
        unnamed.said.providers.clear();
        let said = facts(&budget(&unnamed));
        assert!(said.ends_with("providers=claude-code"), "{said}");
    }
}
