//! What a session's row says at its right, and the figures its drawer
//! says, by OpenTelemetry's `gen_ai` names as its record's `usage` keeps
//! them (the design's figures, 2026-11-06).

use serde_json::Value;

use crate::said::Said;

/// One thing a row says at its right, with how much it's needed, so
/// where room runs short they give way whole (Choice.qml).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Item {
    pub said: String,
    pub keep: u8,
}

/// What a row says at its right: how it stands, then what it spent
/// today, kept longest, and nothing more (VKCF3Z, 2026-11-06: "working ·
/// $4.20"). Its address and folder are its drawer's eyebrow. A tier 2
/// figure leads while it's true, "84% full · working · $4.20", and is
/// kept longest, since it's why the row asks for a look; beside it how
/// the session stands outlasts the money, which the drawer and Today say
/// too (VKCF3Z, 2026-11-07).
pub fn side(doing: &str, spent: &str, filled: &str) -> Vec<Item> {
    let (stands, costs) = if filled.is_empty() { (3, 4) } else { (4, 3) };
    [(filled, 5), (doing, stands), (spent, costs)]
        .into_iter()
        .filter(|(said, _)| !said.is_empty())
        .map(|(said, keep)| Item {
            said: said.to_owned(),
            keep,
        })
        .collect()
}

/// The share of its window past which a session's context is tier 2 on
/// its row: a compaction is near, and each turn sends most of a window.
const FULL_AT: f64 = 80.0;

/// A figure the usage says, or 0 where it says none.
fn number(value: Option<&Value>) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(0.0)
}

/// How full a session's context is, as a whole percent of its window,
/// from its record's `usage.context` ({ used, limit }), the harness's
/// count of what the next turn sends; 0 where it isn't said.
pub fn fullness(usage: Option<&Value>) -> f64 {
    let context = usage.and_then(|usage| usage.get("context"));
    let used = number(context.and_then(|context| context.get("used")));
    let limit = number(context.and_then(|context| context.get("limit")));
    if limit > 0.0 && used > 0.0 {
        (used / limit * 100.0).floor()
    } else {
        0.0
    }
}

/// What a row says of how full it is, only past `FULL_AT`; "" below it.
pub fn full(usage: Option<&Value>, said: &Said) -> String {
    let share = fullness(usage);
    if share >= FULL_AT {
        said.fill("full", &[("share", &share.to_string())])
    } else {
        String::new()
    }
}

/// A figure to one decimal, as JavaScript's `toFixed(1)` says it, with a
/// whole one's ".0" left off.
fn tenths(value: f64) -> String {
    let shown = format!("{value:.1}");
    if let Some(whole) = shown.strip_suffix(".0") {
        return whole.to_owned();
    }
    shown
}

/// A count of tokens as a figure reads it: whole under a thousand, then
/// thousands and millions, one decimal while that's under ten ("4.1k",
/// "142k", "1.2M", "1M").
pub fn tokens(count: f64) -> String {
    let scaled = |by: f64, unit: &str| {
        let value = count / by;
        let shown = if value < 10.0 {
            tenths(value)
        } else {
            value.round().to_string()
        };
        format!("{shown}{unit}")
    };
    if count < 1000.0 {
        count.round().to_string()
    } else if (count / 1000.0).round() < 1000.0 {
        scaled(1000.0, "k")
    } else {
        scaled(1_000_000.0, "M")
    }
}

/// How long a turn took, from milliseconds: tenths of a second under
/// ten, whole seconds under a minute, then minutes.
pub fn took(ms: f64) -> String {
    let seconds = ms / 1000.0;
    if seconds < 10.0 {
        format!("{} s", tenths(seconds))
    } else if seconds < 60.0 {
        format!("{} s", seconds.round())
    } else {
        format!("{} min", (seconds / 60.0).round())
    }
}

/// One of a drawer's figures: what names it, what it says and, for its
/// context, Meter's stroke of how full, warning from `FULL_AT` as its
/// row's glyph does (VKCF3Z, 2026-11-07).
#[derive(Clone, Debug, PartialEq)]
pub struct Fact {
    pub label: String,
    pub value: String,
    pub stroke: Option<Stroke>,
}

/// How full a context is, as a share of one, and whether that warns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub value: f64,
    pub warn: bool,
}

/// The `gen_ai` usage figure of one kind, in a set of them.
fn used(spent: Option<&Value>, kind: &str) -> f64 {
    number(spent.and_then(|spent| spent.get(format!("gen_ai.usage.{kind}"))))
}

/// The drawer's figures for a session (tier 3): how full its context is
/// and its last turn first, the two that move as it works, then its
/// model and today's turns, tokens and cache, those only while the
/// record's day is `today`, since the harness starts them over each day.
/// No amount is among them, so sharing the screen hides none; what it
/// spent is spend-watch's, said beside them. `named` names each figure.
pub fn figures(
    usage: Option<&Value>,
    today: &str,
    said: &Said,
    named: &dyn Fn(&str) -> String,
) -> Vec<Fact> {
    let Some(usage) = usage.filter(|usage| usage.is_object()) else {
        return Vec::new();
    };
    let at = |path: &[&str]| path.iter().try_fold(usage, |value, key| value.get(*key));
    let model = at(&["gen_ai.response.model"])
        .or_else(|| at(&["gen_ai.request.model"]))
        .and_then(Value::as_str)
        .unwrap_or("");
    let (used_now, limit) = (
        number(at(&["context", "used"])),
        number(at(&["context", "limit"])),
    );
    let context = if limit > 0.0 && used_now > 0.0 {
        said.fill(
            "contextOf",
            &[("used", &tokens(used_now)), ("limit", &tokens(limit))],
        )
    } else {
        String::new()
    };
    let lasted = number(at(&["turns", "last", "duration_ms"]));
    let first = number(at(&["turns", "last", "first_token_ms"]));
    let mut turn = String::new();
    if lasted > 0.0 {
        let out = tokens(used(at(&["last"]), "output_tokens"));
        turn = said.fill("turnTook", &[("took", &took(lasted)), ("out", &out)]);
        if first > 0.0 {
            turn = format!(
                "{turn} · {}",
                said.fill("firstToken", &[("took", &took(first))])
            );
        }
    }
    let on_its_day = at(&["day"]).and_then(Value::as_str) == Some(today);
    let day = if on_its_day { at(&["today"]) } else { None };
    let sent = used(day, "input_tokens")
        + used(day, "cache_read.input_tokens")
        + used(day, "cache_creation.input_tokens");
    let turns = number(at(&["turns", "today"]));
    let (tokens_of, cache) = if sent > 0.0 {
        let out = tokens(used(day, "output_tokens"));
        let read = used(day, "cache_read.input_tokens") / sent * 100.0;
        (
            said.fill("tokensOf", &[("sent", &tokens(sent)), ("out", &out)]),
            said.fill("cacheRead", &[("share", &format!("{read:.1}"))]),
        )
    } else {
        (String::new(), String::new())
    };
    let known = [
        ("context", context),
        ("last-turn", turn),
        ("model", model.to_owned()),
        (
            "turns",
            if on_its_day && turns > 0.0 {
                turns.to_string()
            } else {
                String::new()
            },
        ),
        ("tokens", tokens_of),
        ("cache", cache),
    ];
    let stroke = Stroke {
        value: used_now / limit,
        warn: fullness(Some(usage)) >= FULL_AT,
    };
    known
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(id, value)| Fact {
            label: named(id),
            value,
            stroke: (id == "context").then_some(stroke),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn words() -> Said {
        Said::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    fn usage() -> Value {
        json!({
            "day": "2026-11-07",
            "gen_ai.request.model": "claude-opus-5-5",
            "last": {"gen_ai.usage.output_tokens": 4100},
            "today": {"gen_ai.usage.input_tokens": 786, "gen_ai.usage.output_tokens": 272_470,
                "gen_ai.usage.cache_read.input_tokens": 124_620_036,
                "gen_ai.usage.cache_creation.input_tokens": 1_194_960},
            "context": {"used": 142_000, "limit": 200_000},
            "turns": {"today": 359, "last": {"duration_ms": 38000, "first_token_ms": 900}}
        })
    }

    #[test]
    fn a_row_says_how_it_stands_and_what_it_spent() {
        let read = |items: Vec<Item>| {
            let said: Vec<_> = items
                .iter()
                .map(|item| format!("{}/{}", item.said, item.keep))
                .collect();
            said.join(" ")
        };
        let cases = [
            (
                "its state, then its spend, kept longest",
                "working",
                "$4.20",
                "",
                "working/3 $4.20/4",
            ),
            (
                "one that has spent nothing says its state alone",
                "listening",
                "",
                "",
                "listening/3",
            ),
            ("one off says only what it spent", "", "$1", "", "$1/4"),
            (
                "a tier 2 figure leads, kept longest, the state outlasting the money",
                "working",
                "$4.20",
                "84% full",
                "84% full/5 working/4 $4.20/3",
            ),
        ];
        for (what, doing, spent, filled, want) in cases {
            assert_eq!(read(side(doing, spent, filled)), want, "{what}");
        }
    }

    #[test]
    fn past_four_fifths_a_row_says_how_full() {
        let cases = [
            (json!(842), json!(1000), "84% full"),
            (json!(799), json!(1000), ""),
            (Value::Null, json!(200_000), ""),
            (json!(5), json!(0), ""),
        ];
        for (used, limit, want) in cases {
            let usage = json!({"context": {"used": used, "limit": limit}});
            assert_eq!(full(Some(&usage), &words()), want, "{used} of {limit}");
        }
        assert_eq!(
            full(None, &words()),
            "",
            "one with no usage said it was full"
        );
    }

    #[test]
    fn counts_and_times_read_as_figures_do() {
        let counts: Vec<_> = [
            2.0,
            999.0,
            4100.0,
            9999.0,
            142_400.0,
            999_600.0,
            1_000_000.0,
            1_240_000.0,
            124_620_036.0,
        ]
        .into_iter()
        .map(tokens)
        .collect();
        assert_eq!(counts.join(" "), "2 999 4.1k 10k 142k 1M 1M 1.2M 125M");
        let times: Vec<_> = [9284.0, 900.0, 38000.0, 125_000.0]
            .into_iter()
            .map(took)
            .collect();
        assert_eq!(times.join(", "), "9.3 s, 0.9 s, 38 s, 2 min");
    }

    #[test]
    fn the_drawers_figures_say_todays_only_on_its_day() {
        let read = |today: &str| {
            let facts = figures(Some(&usage()), today, &words(), &str::to_owned);
            let said: Vec<_> = facts
                .iter()
                .map(|fact| format!("{}={}", fact.label, fact.value))
                .collect();
            said.join("; ")
        };
        let lasting = "context=142k of 200k; last-turn=38 s · 4.1k out · 0.9 s first token; \
                       model=claude-opus-5-5";
        let on_its_day =
            format!("{lasting}; turns=359; tokens=126M in · 272k out; cache=99.0% read");
        assert_eq!(read("2026-11-07"), on_its_day, "the figures on their day");
        assert_eq!(read("2026-11-08"), lasting, "the figures a day on");
        assert!(
            figures(None, "2026-11-07", &words(), &str::to_owned).is_empty(),
            "one with no usage had figures"
        );
    }

    #[test]
    fn context_alone_carries_a_stroke_warning_from_four_fifths() {
        let strokes = |usage: &Value| -> Vec<_> {
            figures(Some(usage), "2026-11-07", &words(), &str::to_owned)
                .into_iter()
                .filter_map(|fact| {
                    let stroke = fact.stroke?;
                    Some(format!("{}:{}:{}", fact.label, stroke.value, stroke.warn))
                })
                .collect()
        };
        let mut fuller = usage();
        fuller["context"] = json!({"used": 168_000, "limit": 200_000});
        assert_eq!(strokes(&usage()), ["context:0.71:false"]);
        assert_eq!(strokes(&fuller), ["context:0.84:true"]);
    }
}
