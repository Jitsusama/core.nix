//! Where each of the hour's findings goes: to its owner while a session
//! answers as it, or else to the orchestrator, saying whose it was. A
//! finding sent to an owner that has ended is written to the mesh and read
//! by no one: the relay's count of 502s went so on 27 November, and two of
//! Joel's turns that stalled on them were found only by hand.

/// The address a finding for `owner` goes to, with its words: `owner`
/// itself while `there` says a session answers as it, or else `to`, told
/// whose it was and asked to pass it on. The ledger still owes it to
/// `owner`, so it closes as its figure comes back in, whoever fixes it.
#[must_use]
pub fn told_to(
    owner: &str,
    text: String,
    to: &str,
    there: impl Fn(&str) -> bool,
) -> (String, String) {
    if there(owner) {
        return (owner.to_owned(), text);
    }
    (
        to.to_owned(),
        format!(
            "For {owner}, which no session answers as now: {text} Please pass it to whoever \
             owns this now, and have cadence's owner map name them, so the next one reaches \
             them."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finding_whose_owner_has_ended_goes_to_the_orchestrator_saying_whose() {
        let there = |address: &str| address == "LIVE01";
        let said = "cadence hour: the relay is past its ceiling.".to_owned();
        assert_eq!(
            told_to("LIVE01", said.clone(), "ORCH01", there),
            ("LIVE01".to_owned(), said.clone()),
            "a finding for an owner a session answers as went elsewhere, or was reworded"
        );
        let (to, text) = told_to("GONE01", said.clone(), "ORCH01", there);
        assert_eq!(
            to, "ORCH01",
            "a finding for an owner no session answers as wasn't sent to the orchestrator"
        );
        assert!(
            text.starts_with("For GONE01, ") && text.contains(&said),
            "the orchestrator wasn't told whose finding it was, or not the finding: {text}"
        );
    }
}
