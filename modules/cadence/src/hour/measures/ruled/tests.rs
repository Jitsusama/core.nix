//! The measure of claims of Joel's rule: which are backed by a word of
//! his, and which are asked of their senders.

use std::fs;

use super::super::slipped::at;
use super::*;

const NOW: i64 = 1_795_600_000;

fn sent(short: &str, ago: i64, from: &str, text: &str) -> Message {
    Message {
        v: 1,
        id: format!("{}-{from}-1-{short}", (NOW - ago) * 1000),
        at: at(NOW - ago),
        from: from.to_owned(),
        to: vec!["#quest/wcjqsg".to_owned()],
        intent: "fyi".to_owned(),
        text: text.to_owned(),
        pid: 1,
        re: None,
        supersedes: None,
        ends_hold: false,
    }
}

/// Cadence's ask `ago` seconds back to `to` about its message `claim`.
fn asked(short: &str, ago: i64, claim: &str, to: &str) -> Message {
    Message {
        to: vec![format!("@{to}")],
        ..sent(short, ago, CADENCE, &ask(claim, "Joel wants it."))
    }
}

/// On 26 November two cells told the mesh they had broken Joel's rule,
/// with no word of his behind it, each in a message holding code and
/// times of its own, while a third gave his words and when.
#[test]
fn a_claim_of_his_rule_with_nothing_behind_it_is_counted_and_its_sender_named() {
    let all = [
        sent(
            "0000000b",
            600,
            "5UTLKM",
            "5UTLKM, the raw record. I put down nothing; my last such call was Thu 00:43.\n\
             **I ran 17 of these myself.** That breaks Joel's rule, and my 05:52 message \
             undercounted it as 6. Checked by jq over every log since Wed 25 Nov.",
        ),
        sent(
            "0000000a",
            3000,
            "T6E7X6",
            "T6E7X6, 10:08: held and stopped. I have no grant, and I broke Joel's rule. \
             (1) I reached the box through `tec run //delivery:lab -- ssh the box now`, from \
             my tree at main b1cd33e5.",
        ),
        sent(
            "0000000c",
            900,
            "WCJQSG",
            "Joel wants loops built. Joel, 11:30 Thu 26 Nov, verbatim: \"we need to make \
             sure we have various self improvement loops built in\"",
        ),
    ];
    let m = unsourced(&all, NOW);
    assert!((m.now - 2.0).abs() < 0.5, "two unbacked, got {}", m.now);
    assert!(
        m.past().is_some(),
        "an unbacked claim isn't past its target of 0"
    );
    assert_eq!(
        m.behind,
        [
            "[0000000a] I have no grant, and I broke Joel's rule. · T6E7X6",
            "[0000000b] That breaks Joel's rule, and my 05:52 message undercounted ... · 5UTLKM",
        ],
        "the unbacked claims weren't named oldest first by id, sentence and sender"
    );
    assert_eq!(m.owner, "T6E7X6", "the oldest unbacked claim's sender");
}

#[test]
fn a_date_or_a_clock_given_as_when_he_said_it_backs_a_claim() {
    let backed = [
        "Per Joel on 22 Nov, security never blocks the work.",
        "Joel ruled it on Nov 22.",
        "Joel ruled it Sun 22.",
        "Joel ruled it in 2026-11-22's thread.",
        "Joel said at 15:49 that security never blocks the work.",
        "Joel said on 26th Nov that security never blocks the work.",
        "Joel's rule from 10:05 stands.",
        "Joel's rule of 22 Nov stands.",
        "On 22 Nov, Joel ruled it.",
        "Joel ruled it. He said so on 22 Nov.",
        "Joel ruled it. His words of 22 Nov: never block the work.",
        "Joel ruled it. Sam agreed, and on 25 Nov Joel said so.",
        "On 22 Nov he said Joel wants it.",
        "Joel wants it, his words of 10:05.",
        "Joel at 10:05 said he wants it.",
        "Joel wants it. Joel (22 Nov 15:49): security never blocks the work.",
    ];
    for text in backed {
        assert!(
            claims(text),
            "the case claims nothing, so tests nothing: {text}"
        );
        assert_eq!(unbacked(text), None, "a backed claim was counted: {text}");
    }
}

#[test]
fn words_that_only_look_like_a_date_or_a_quote_back_nothing() {
    let unbacked_ones = [
        "Joel wants it declined for 3 days.",
        "Joel's rule says `no` here.",
        "Joel's rule is `never reach the lab box at all`.",
        "Joel wants \"two words\" kept.",
        "At 15:49 the box fell over, and later on it was Joel's rule anyway.",
        "Joel ruled it. The box fell. It came back. It was 22 Nov by then.",
        "It was 22 Nov. The box fell. Joel ruled it.",
        "Joel wants it done by 26 Nov.",
        "I broke Joel's rule at 10:08.",
        "That breaks Joel's rule, my 05:52 message said.",
        "Joel's rule stands, Joelle said at 11:30.",
        "Joel wants this: \"spread load now please\".",
        "I broke Joel's rule, \"never reach the lab box\".",
        "Joel ruled it.\n\"Never block the work\", he said.",
        "T6E7X6, 10:08: Joel wants the lab shut.",
        // A time leading his name or he is when the rule was broken or he
        // was seen, unless a word of his saying follows.
        "At 10:08, Joel's rule was broken by me.",
        "On 26 Nov Joel's rule was broken twice.",
        "I broke Joel's rule. At 10:08 he saw it.",
        // A time after him with no word of his saying is a cell's own.
        "I broke Joel's rule. I told him at 10:08.",
        "Joel wants the lab shut; I'll tell him at 14:00.",
        "I broke Joel's rule. I told Joel at 10:08.",
        "Joel wants the lab shut; I'll ask Joel at 14:00.",
        "Joel wants it. I pinged Joel on 26 Nov.",
        "Joel wants the lab shut; I told Joel 10:08.",
        "Joel wants it; I pinged Joel, 10:08.",
        // He, his or him in another sentence may be anyone, unless it
        // names Joel or opens the sentence just after the claim.
        "Joel wants the lab shut. Sam agreed; on 25 Nov he said so.",
        "Joel wants it. Sam quoted his words of 25 Nov.",
        "He said so on 22 Nov. Joel ruled it.",
        // What he said, told with no date, is a claim of his words too.
        "Joel said the launcher hangs.",
    ];
    for text in unbacked_ones {
        assert!(
            unbacked(text).is_some(),
            "an unbacked claim went uncounted: {text}"
        );
    }
}

/// Relays cite his words by the message that carried them, which backs
/// a claim only when his quote or its date travels with the id.
#[test]
fn a_relayed_ruling_is_backed_by_the_words_it_carries_not_by_its_id_alone() {
    let carried = "Per Joel, loops come first (WCJQSG 88ee4e51: Joel, 11:30 Thu 26 Nov, \
                   verbatim: \"we need to make sure we have various self improvement loops \
                   built in\").";
    assert_eq!(
        unbacked(carried),
        None,
        "a relay carrying his dated words was counted"
    );
    assert!(
        unbacked("Per Joel, loops come first (WCJQSG 88ee4e51).").is_some(),
        "a relay naming only the message id went uncounted"
    );
}

/// A message with a backed claim and an unbacked one further on is
/// counted for the unbacked one, and a stray quote mark hides no claim
/// after it. A claim next to a backed one shares its backing, as a cite
/// may follow the claim it backs.
#[test]
fn every_claim_in_a_message_is_judged_past_a_backed_one_or_a_stray_quote() {
    assert_eq!(
        unbacked("Per Joel on 22 Nov, security never blocks. The lab is open. Joel wants it shut."),
        Some("Joel wants it shut.".to_owned()),
        "a backed claim hid an unbacked one after it"
    );
    assert_eq!(
        unbacked("I typed \" by mistake. Joel wants the lab shut."),
        Some("Joel wants the lab shut.".to_owned()),
        "a stray quote mark hid the claim after it"
    );
    assert_eq!(
        unbacked("I typed \" by mistake. Joel wants \"dark\" mode."),
        Some("Joel wants \"...\" mode.".to_owned()),
        "a stray quote mark paired with a later quote and hid the claim between"
    );
}

#[test]
fn his_rule_counts_only_in_a_sentence_naming_joel() {
    assert!(
        unbacked("Christian set it up. That is his rule for the lab.").is_none(),
        "another person's rule was counted as Joel's"
    );
    assert!(
        unbacked("Joel set it up, and that is his rule now.").is_some(),
        "his rule in a sentence naming Joel went uncounted"
    );
    assert!(
        unbacked("Joel's pain, by what he said, is the launcher.").is_none(),
        "what he said, told as a pain, was counted as a claim of a rule"
    );
}

#[test]
fn a_claim_inside_quotes_is_words_about_a_claim_and_isnt_counted() {
    let about = [
        "The measure counts messages like \"Joel wants\" and nothing more.",
        "It asks of `joel's rule` claims.",
    ];
    for text in about {
        assert_eq!(unbacked(text), None, "a quoted claim was counted: {text}");
    }
}

#[test]
fn a_claim_with_a_word_inside_joelish_isnt_one() {
    assert_eq!(
        unbacked("Bjoel's rule is not his."),
        None,
        "a word only ending in joel was read as his name"
    );
    assert_eq!(
        unbacked("It goes on Joel's wish list."),
        None,
        "the wish list, a list of his, was read as a wish he said"
    );
    for text in [
        "Joel's calendar has the review at 3.",
        "Per Joelle, the lab is shut.",
        "Joel's willingness is not the question.",
        "Joel's call at 16:35 dropped its video.",
        "This measures Joel's calls under load.",
    ] {
        assert_eq!(
            unbacked(text),
            None,
            "a word only starting a claim was read as one: {text}"
        );
    }
    assert!(
        unbacked("Those were Joel's rules.").is_some(),
        "a claim in the plural went uncounted"
    );
    assert!(
        unbacked("Those were Joel's wishes.").is_some(),
        "a claim in the plural in es went uncounted"
    );
}

/// A denial of his rule is the correction the measure wants, not a claim,
/// but a denial set off by a stop denies nothing after it.
#[test]
fn a_denial_of_his_rule_claims_nothing() {
    for text in [
        "That was my pivot, not Joel's rule.",
        "This isn't Joel's rule.",
        "It was never his rule, Joel set nothing.",
    ] {
        assert_eq!(
            unbacked(text),
            None,
            "a denial was counted as a claim: {text}"
        );
    }
    assert!(
        unbacked("No, Joel wants the lab shut.").is_some(),
        "a claim after a no set off by a stop went uncounted"
    );
}

/// The card names each claim on a line of its own for the orchestrator to
/// pass on, so a message passing on the card's lines claims nothing; its
/// own words around them are still judged.
#[test]
fn the_cards_lines_passed_on_claim_nothing() {
    let relay = format!(
        "From the hour's card:\n✕ {SAYS}: 1, at most 0\n  [0000000a] I broke Joel's rule. · \
         T6E7X6\n- [0000000b] Joel wants it. · 5UTLKM\n✓ pains Joel said that nobody has \
         taken up: 0, at most 0\n· pains Joel said that nobody has taken up: 2, unjudged\n\
         Drag: pains Joel said that nobody has taken up, 3 against at most 0, to 1JQPPC by \
         10:00.\nLast hour's drag, pains Joel said that nobody has taken up, is back in.\n\
         Out of spec three days running:\n  - pains Joel said that nobody has taken up: 3 \
         against at most 0, out since 2026-11-24, 1JQPPC"
    );
    assert_eq!(
        unbacked(&relay),
        None,
        "a relay of the card was counted as a claim"
    );
    assert!(
        unbacked(&format!("{relay}\nAnd Joel wants it shut.")).is_some(),
        "a claim of the relayer's own beside the card's lines went uncounted"
    );
    for shaped in [
        "- [NOTE] Joel's rule: nobody reaches the lab · T6E7X6",
        "[x] Joel wants it shut · DONE",
        "✓ Joel wants it shut.",
        "Drag: Joel wants it shut.",
    ] {
        assert!(
            unbacked(shaped).is_some(),
            "a line only shaped a little like the card's hid its claim: {shaped}"
        );
    }
}

/// The ask's text is written by [`ask`] and read back by [`about`], so
/// rewording one can't leave the other reading no ask at all; and it
/// claims nothing itself, so a cell passing it on isn't asked in turn.
#[test]
fn an_asks_text_names_the_claim_it_asks_about_and_claims_nothing() {
    let asked = ask("0000000a", "I broke Joel's rule.");
    assert_eq!(
        about(&asked),
        Some("0000000a"),
        "the ask's own text didn't read back as about its claim"
    );
    assert_eq!(
        unbacked(&format!("Passing this on: {asked}")),
        None,
        "the ask's own words, passed on, were counted as a claim"
    );
}

/// The ask tells its sender to answer re the ask or re the claim, and
/// either takes the claim off the count without being counted itself,
/// whatever the answer says. A reply re a claim never asked about, its
/// sender's own or backed, another cell's reply, or a reply to any other
/// message of cadence's, clears nothing and is judged as any other.
#[test]
fn a_senders_answer_re_the_ask_or_the_claim_clears_it_and_isnt_counted() {
    let first = sent("0000000a", 3000, "T6E7X6", "I broke Joel's rule.");
    let second = sent("0000000b", 2900, "5UTLKM", "That breaks Joel's rule.");
    let third = sent("0000000c", 2800, "IKQ63G", "Joel wants the lab shut.");
    let backed = sent(
        "00000017",
        2700,
        "IKQ63G",
        "Joel said on 22 Nov he wants it.",
    );
    let ask = asked("0000000d", 1200, "0000000a", "T6E7X6");
    let ask_second = asked("00000015", 1100, "0000000b", "5UTLKM");
    let mut own_unasked = sent("00000016", 150, "IKQ63G", "So Joel wants it shut.");
    own_unasked.re = Some(backed.id.clone());
    let mut by_ask = sent(
        "0000000e",
        600,
        "T6E7X6",
        "The claim is dropped: Joel's rule was my own caution.",
    );
    by_ask.re = Some("0000000d".to_owned());
    let mut by_claim = sent(
        "0000000f",
        500,
        "5UTLKM",
        "Dropped: Joel's rule was IKQ63G's.",
    );
    by_claim.re = Some(second.id.clone());
    let mut not_its_own = sent("00000010", 400, "T6E7X6", "Joel's rule binds you too.");
    not_its_own.re = Some(third.id.clone());
    let mut not_its_ask = sent("00000012", 300, "5UTLKM", "Joel wants T6E7X6 to stop.");
    not_its_ask.re = Some("0000000d".to_owned());
    let card = sent(
        "00000013",
        250,
        CADENCE,
        "cadence hour: 3 measures past target. Is your red line held?",
    );
    let mut by_card = sent("00000014", 200, "IKQ63G", "Held: Joel wants it held.");
    by_card.re = Some("00000013".to_owned());
    let mut before_ask = sent("00000018", 1500, "T6E7X6", "Correction: Joel wants it.");
    before_ask.re = Some("0000000a".to_owned());
    let mut again = sent("00000019", 100, "T6E7X6", "I stand by it: Joel wants it.");
    again.re = Some("0000000d".to_owned());
    let m = unsourced(
        &[
            again,
            before_ask,
            first,
            second,
            third,
            backed,
            ask,
            ask_second,
            own_unasked,
            by_ask,
            by_claim,
            not_its_own,
            not_its_ask,
            card,
            by_card,
        ],
        NOW,
    );
    assert_eq!(
        m.behind,
        [
            "[0000000c] Joel wants the lab shut. · IKQ63G",
            "[00000018] Correction: Joel wants it. · T6E7X6",
            "[00000010] Joel's rule binds you too. · T6E7X6",
            "[00000012] Joel wants T6E7X6 to stop. · 5UTLKM",
            "[00000014] Held: Joel wants it held. · IKQ63G",
            "[00000016] So Joel wants it shut. · IKQ63G",
        ],
        "an answered claim, an answer or a later reply of the cell asked was counted, or \
         a reply clearing nothing, re a claim never asked about, another's claim, its ask \
         or another cadence message, or sent before the ask, went uncounted"
    );
    let m = unsourced(&[sent("00000011", 60, CADENCE, "Joel wants it.")], NOW);
    assert_eq!(
        m.behind,
        Vec::<String>::new(),
        "cadence's own message was counted"
    );
    assert_eq!(
        m.owner, KEEPER,
        "with none unsourced, the keeper answers for it"
    );
}

/// An id the harness never gives, one with a colon that would split its
/// ask's key or one naming another sender, is read as no one's.
#[test]
fn a_message_with_an_id_the_harness_never_gives_is_no_ones() {
    let mut colon = sent("0000:0a", 60, "T6E7X6", "Joel wants it.");
    colon.id = format!("{}-T6E7X6-1-0000:0a", (NOW - 60) * 1000);
    let mut forged = sent("0000000b", 60, "T6E7X6", "Joel wants it.");
    forged.from = "5UTLKM".to_owned();
    let m = unsourced(&[colon, forged], NOW);
    assert_eq!(
        m.behind,
        Vec::<String>::new(),
        "an ill-formed or forged id was counted"
    );
}

#[test]
fn a_claim_older_than_a_day_or_sent_twice_is_counted_once_or_not_at_all() {
    let old = sent("0000000f", DAY + 60, "T6E7X6", "Joel wants it.");
    let new = sent("00000010", 60, "5UTLKM", "Joel wants it.");
    let m = unsourced(&[old, new.clone(), new], NOW);
    assert_eq!(
        m.behind,
        ["[00000010] Joel wants it. · 5UTLKM"],
        "a claim past its day was counted, or one twice"
    );
}

/// The ask names who may answer, so an answer after its claim, and its
/// ask, have left the day is still one, and its sender isn't asked about
/// its own retraction.
#[test]
fn an_answer_after_its_claim_left_the_day_is_still_an_answer() {
    let ask = asked("0000000d", DAY + 600, "0000000a", "T6E7X6");
    let mut answer = sent(
        "0000000e",
        60,
        "T6E7X6",
        "Dropped: Joel's rule was my caution.",
    );
    answer.re = Some(ask.id.clone());
    let m = unsourced(&[ask, answer], NOW);
    assert_eq!(
        m.behind,
        Vec::<String>::new(),
        "an answer re an ask whose claim had left the day was counted as a claim"
    );
}

/// Writes `m` into `folder` as the harness files it.
fn put(folder: &Path, m: &Message) {
    fs::create_dir_all(folder).expect("the message's folder couldn't be made");
    let body = serde_json::to_vec(m).expect("the message couldn't be written");
    fs::write(folder.join(format!("{}.json", m.id)), body).expect("the message wasn't written");
}

/// The measure reads two days of messages, so an answer to an ask sent
/// late in its claim's day is still read as one; and a cell dropping its
/// claim after a first answer of "looking into it" isn't asked about its
/// own retraction.
#[test]
fn the_measure_reads_two_days_so_late_answers_and_later_retractions_are_left_alone() {
    let dir = tempfile::tempdir().expect("the test's folder couldn't be made");
    let messages = dir
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    let ask = asked("0000000d", DAY + 600, "0000000a", "T6E7X6");
    let mut answer = sent(
        "0000000e",
        60,
        "T6E7X6",
        "Dropped: Joel's rule was my caution.",
    );
    answer.re = Some(ask.id.clone());
    put(&messages, &ask);
    put(&messages, &answer);
    let m = read(dir.path(), NOW);
    assert_eq!(
        (m.behind.as_slice(), m.says.as_str()),
        ([].as_slice(), SAYS),
        "an answer to an ask over a day old was counted"
    );
    let later = asked("00000011", DAY + 600, "0000000b", "5UTLKM");
    let mut first = sent("00000012", DAY + 300, "5UTLKM", "Looking into it.");
    first.re = Some(later.id.clone());
    let mut dropped = sent(
        "00000013",
        60,
        "5UTLKM",
        "Dropped: Joel's rule was my own caution.",
    );
    dropped.re = Some(later.id.clone());
    for m in [&later, &first, &dropped] {
        put(&messages, m);
    }
    let m = read(dir.path(), NOW);
    assert_eq!(
        m.behind,
        Vec::<String>::new(),
        "a cell dropping its claim after a first answer was asked about its retraction"
    );
}

/// Cadence's asks are read back further than the claims, so an answer
/// sent more than a day after its ask, which has left the two days of
/// claims read, is still an answer, not a fresh claim to ask about.
#[test]
fn an_answer_sent_days_after_its_ask_is_still_an_answer() {
    let dir = tempfile::tempdir().expect("the test's folder couldn't be made");
    let messages = dir
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    let ask = asked("0000000d", 3 * DAY, "0000000a", "T6E7X6");
    let mut answer = sent(
        "0000000e",
        60,
        "T6E7X6",
        "Dropped: Joel's rule was my caution.",
    );
    answer.re = Some(short_id(&ask.id).to_owned());
    put(&messages, &ask);
    put(&messages, &answer);
    assert_eq!(
        read(dir.path(), NOW).behind,
        Vec::<String>::new(),
        "an answer to an ask past the two days of claims read was counted as a claim"
    );
}

/// A claim is given to the card and the ask by its first words and its
/// first characters, so neither a sentence with no space in it nor one
/// spaced by marks that only look like spaces is given whole.
#[test]
fn a_claim_is_given_short_however_it_is_spaced() {
    for text in [
        format!("Joel wants {}", "x".repeat(300)),
        format!("Joel wants{}", "\u{2800}word".repeat(100)),
        format!("Joel wants{}", "\u{3164}word".repeat(100)),
    ] {
        let m = unsourced(&[sent("0000000a", 60, "T6E7X6", &text)], NOW);
        let shown = m
            .behind
            .first()
            .unwrap_or_else(|| panic!("a claim spaced oddly wasn't counted: {text}"));
        assert!(
            shown.chars().count() <= CLAIM_LONGEST + 40,
            "a claim was given whole: {} characters",
            shown.chars().count()
        );
    }
}

/// A claim is aged by its id's stamp, which the harness gives, not by the
/// `at` its sender writes, so one set a day back still counts.
#[test]
fn a_claim_is_aged_by_its_ids_stamp_not_its_senders_time() {
    let mut backdated = sent("0000000a", 60, "T6E7X6", "Joel wants it.");
    backdated.at = at(NOW - DAY - 60);
    assert_eq!(
        unsourced(&[backdated], NOW).behind,
        ["[0000000a] Joel wants it. · T6E7X6"],
        "a claim whose sender set its time a day back slipped out of the count"
    );
}

/// Two claims of one sender's in the same second are told apart by their
/// milliseconds, so however the folders list them its oldest stays the
/// one it's asked about.
#[test]
fn claims_sent_in_one_second_keep_their_order_however_they_are_listed() {
    let mut early = sent("0000000b", 60, "T6E7X6", "Joel wants it.");
    early.id = format!("{}-T6E7X6-1-0000000b", (NOW - 60) * 1000 + 100);
    let mut late = sent("0000000a", 60, "T6E7X6", "Joel wants it shut.");
    late.id = format!("{}-T6E7X6-1-0000000a", (NOW - 60) * 1000 + 900);
    for listed in [[early.clone(), late.clone()], [late, early]] {
        assert_eq!(
            unsourced(&listed, NOW).behind[0],
            "[0000000b] Joel wants it. · T6E7X6",
            "a sender's oldest claim changed with the order its messages were listed in"
        );
    }
}

#[test]
fn with_no_messages_folder_or_one_that_cant_be_listed_the_measure_is_left_unjudged() {
    let dir = tempfile::tempdir().expect("the test's folder couldn't be made");
    let home = dir.path();
    let m = read(home, NOW);
    assert_eq!(
        m.target, None,
        "an unread mesh was judged as none unsourced"
    );
    let messages = home.join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder couldn't be made");
    let m = read(home, NOW);
    assert_eq!(m.target, Some(0.0), "a mesh with no messages went unjudged");
    let torn = format!("{}-T6E7X6-1-0000000a", (NOW - 60) * 1000);
    fs::write(messages.join(format!("{torn}.json")), "{")
        .expect("the torn message couldn't be written");
    let claim = sent("0000000b", 120, "5UTLKM", "Joel wants it.");
    let body = serde_json::to_vec(&claim).expect("the claim couldn't be written");
    fs::write(messages.join(format!("{}.json", claim.id)), body)
        .expect("the claim couldn't be written");
    let m = read(home, NOW);
    assert_eq!(
        (m.target, m.behind.as_slice()),
        (
            Some(0.0),
            ["[0000000b] Joel wants it. · 5UTLKM".to_owned()].as_slice()
        ),
        "one torn file hid the day's messages"
    );
    assert!(
        m.says.contains(&torn),
        "the torn file wasn't named: {}",
        m.says
    );
    shut_day_is_unread(&messages);
}

/// A day's folder that can't be listed leaves the measure unread, saying
/// why, rather than read as none and closing its asks.
#[cfg(unix)]
fn shut_day_is_unread(messages: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let home = messages
        .ancestors()
        .nth(4)
        .expect("the messages folder has a home");
    let day = messages.join("2026-11-25");
    put(&day, &sent("00000010", 300, "5UTLKM", "Joel wants it."));
    fs::set_permissions(&day, fs::Permissions::from_mode(0o000))
        .expect("the day's folder couldn't be shut");
    let shut = fs::read_dir(&day).is_err();
    let m = read(home, NOW);
    fs::set_permissions(&day, fs::Permissions::from_mode(0o755))
        .expect("the day's folder couldn't be opened again");
    // Run as root, a shut folder still lists, so there's nothing to see.
    if shut {
        assert!(
            m.target.is_none() && m.says.contains("can't be listed"),
            "a day's folder that can't be listed was read as none: {}",
            m.says
        );
    }
}

#[cfg(not(unix))]
fn shut_day_is_unread(_: &Path) {}
