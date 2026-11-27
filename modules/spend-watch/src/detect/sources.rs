//! What the sources say of themselves: what the provider's own meter
//! told the person, and a feed's source that can't give its figures.

use super::{digest, finding};
use crate::findings::{Action, Finding, Severity};
use crate::picture::Picture;

/// What a sentence says without its figures, so a meter that says the same
/// thing with a newer total says it once: on 8 November four findings stood
/// at once, each "7d AI spend has exceeded the $4,000.00 budget" with a
/// total a few dollars on from the last.
fn shape(text: &str) -> String {
    digest(&text.replace(|c: char| c.is_ascii_digit(), ""))
}

/// What a provider said to the person in the last quarter hour, each thing
/// once, as it said it last: a warning when it was one of its own, a note
/// otherwise. Its text is drawn as plain words, never markup.
pub fn said(picture: &Picture) -> Vec<Finding> {
    // The picture holds them oldest first, so the last of each shape is
    // the one kept.
    let mut last: Vec<(String, &(String, String))> = Vec::new();
    for said in &picture.said {
        let shape = shape(&said.1);
        last.retain(|(kept, _)| *kept != shape);
        last.push((shape, said));
    }
    last.into_iter()
        .map(|(shape, (severity, text))| {
            let plain: String = text.chars().filter(|c| !matches!(c, '<' | '>')).collect();
            let severity = match severity.as_str() {
                "critical" | "error" | "alert" | "warning" => Severity::Warning,
                _ => Severity::Note,
            };
            let mut f = finding(
                format!("said:{shape}"),
                "said",
                severity,
                // The meter's, never the cap's: its budget isn't what
                // refuses.
                format!("the provider's meter says: {plain}"),
                "Said by the provider's own meter.".into(),
            );
            f.note = "said by the provider's meter".into();
            f.surprise = 0.5;
            f
        })
        .collect()
}

/// How long a source stands unreadable before it's a finding: past the
/// minute or two a machine waking or booting spends before its network.
const OUTAGE: f64 = 15.0 * 60.0;

/// A feed's source that can't give its figures, once it's stood so a
/// while. One that fails is a note, since other sources carry the figures
/// meanwhile; one that wants a sign-in is a warning, since only the
/// person can fix it, and signing in follows the sign-in door. Optional
/// sources aren't findings.
pub fn health(picture: &Picture) -> Vec<Finding> {
    let mut out = Vec::new();
    for source in &picture.sources {
        let (severity, title) = match source.state.as_str() {
            "signed-out" => (
                Severity::Warning,
                format!("the {} wants a sign-in", source.name),
            ),
            "error" => (Severity::Note, format!("the {} can't be read", source.name)),
            "stalled" => (
                Severity::Note,
                format!("the {} has gone quiet", source.name),
            ),
            _ => continue,
        };
        if picture.now - source.since < OUTAGE {
            continue;
        }
        let mut f = finding(
            format!("health:{}:{}", source.feed, source.name),
            "health",
            severity,
            title,
            if source.message.is_empty() {
                "Other sources carry the figures meanwhile.".into()
            } else {
                format!(
                    "{}. Other sources carry the figures meanwhile.",
                    source.message
                )
            },
        );
        f.action = source
            .sign_in
            .as_ref()
            .and_then(|id| {
                picture
                    .actions
                    .iter()
                    .find(|a| &a.id == id && a.feed == source.feed)
            })
            .map(|a| Action {
                name: a.name.clone(),
                run: a.run.clone(),
            });
        f.badge = format!("since {}", picture.zone.clock(source.since, picture.now));
        // What failed, without the error underneath it, which the detail
        // keeps; or, when the feed says no more than the title does, what
        // it means for the figures.
        source
            .message
            .split(": ")
            .next()
            .filter(|said| !said.is_empty() && !said.eq_ignore_ascii_case(&f.title))
            .unwrap_or("other sources fill in")
            .clone_into(&mut f.note);
        f.surprise = 0.5;
        out.push(f);
    }
    out
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
