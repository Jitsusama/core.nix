//! How many of Joel's pains are still open, from the pain-points hearer's
//! hourly summary. Joel asked on 11 November that we learn from every pain
//! point he has, and the card measured only what our own ledgers held, so a
//! pain he said aloud and nobody took up never reached it.

use serde_json::Value;

use super::{Class, Measure, count};

/// The oldest the hearer's summary may be, in seconds: it writes every five
/// minutes, so three missed writes mean it has stopped.
const STALE_AFTER: i64 = 15 * 60;

/// The most characters of the worst pain's words the card quotes.
const QUOTED: usize = 120;

/// `words` cut at a word boundary to at most `QUOTED` characters.
fn quoted(words: &str) -> String {
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if words.chars().count() <= QUOTED {
        return words;
    }
    let cut: String = words.chars().take(QUOTED).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(kept, _)| kept);
    format!("{cut} ...")
}

/// The measure from `summary`, the hearer's `hour.json` (none when it
/// can't be read), at `now` in seconds, steered by `owner`: unjudged when
/// the summary is missing, torn, stale or names its own troubles, since a
/// count from a hearer that isn't hearing says nothing.
#[must_use]
pub fn pains(summary: Option<&str>, now: i64, owner: &str) -> Measure {
    let mut measure = Measure {
        key: "pains-open".to_owned(),
        class: Class::Functional,
        says: "pains Joel said that are still open".to_owned(),
        now: 0.0,
        target: Some(0.0),
        owner: owner.to_owned(),
        behind: Vec::new(),
    };
    let read: Option<Value> = summary.and_then(|text| serde_json::from_str(text).ok());
    let Some(read) = read else {
        measure.target = None;
        measure
            .says
            .push_str(" (unjudged: the pain-points summary couldn't be read)");
        return measure;
    };
    let open = read.get("open").and_then(Value::as_u64).unwrap_or_default();
    measure.now = count(usize::try_from(open).unwrap_or(usize::MAX));
    let at = read.get("at").and_then(Value::as_i64).map(|ms| ms / 1000);
    let troubles = read
        .get("troubles")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let why = match at {
        None => Some("the pain-points summary isn't dated".to_owned()),
        Some(at) if now - at > STALE_AFTER => Some(format!(
            "the pain-points summary is {} min old",
            (now - at) / 60
        )),
        _ if troubles > 0 => Some(format!("the pain-points hearer names {troubles} troubles")),
        _ => None,
    };
    if let Some(why) = why {
        measure.target = None;
        measure.says.push_str(&format!(" (unjudged: {why})"));
    }
    if let Some(worst) = read.get("worst").and_then(Value::as_str) {
        let hits = read.get("worst_hits").and_then(Value::as_u64).unwrap_or(1);
        measure.behind.push(format!(
            "the worst, said {hits} times: \"{}\"",
            quoted(worst)
        ));
    }
    measure
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_944_760;

    fn summary(at: i64, troubles: &str) -> String {
        format!(
            r#"{{"at":{},"heard":0,"routed":0,"open":20,"worst":"i am not able to show launcher when screen sharing over tuple. this sucks. we need to make it work so I can show people my UI","worst_hits":2,"stalled":0,"troubles":[{troubles}]}}"#,
            at * 1000
        )
    }

    #[test]
    fn the_open_pains_count_with_the_worst_quoted() {
        let m = pains(Some(&summary(NOW - 60, "")), NOW, "WCJQSG");
        assert_eq!(
            (m.now, m.target),
            (20.0, Some(0.0)),
            "the open pains weren't judged: {m:?}"
        );
        assert_eq!(
            m.behind,
            [
                "the worst, said 2 times: \"i am not able to show launcher when screen sharing \
              over tuple. this sucks. we need to make it work so I can show people ...\""
            ],
            "the worst pain wasn't quoted whole words, cut: {m:?}"
        );
    }

    #[test]
    fn a_hearer_missing_torn_stale_or_in_trouble_leaves_it_unjudged() {
        let cases = [
            (None, "couldn't be read"),
            (Some("{\"at\": 17949".to_owned()), "couldn't be read"),
            (Some(summary(NOW - 20 * 60, "")), "20 min old"),
            (
                Some(summary(NOW - 60, "\"slack unread\"")),
                "names 1 troubles",
            ),
        ];
        for (text, why) in cases {
            let m = pains(text.as_deref(), NOW, "WCJQSG");
            assert!(
                m.target.is_none() && m.says.contains(why),
                "a hearer whose summary {why} was judged: {m:?}"
            );
        }
    }
}
