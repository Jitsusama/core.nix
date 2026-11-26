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
    assert!(m.past().is_some(), "an unbacked claim isn't past its target of 0");
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
fn a_quote_a_date_or_a_clock_beside_his_name_backs_a_claim() {
    let backed = [
        "Joel wants this: \"spread load now please\".",
        "Per Joel on 22 Nov, security never blocks the work.",
        "Joel ruled it on Nov 22.",
        "Joel ruled it Sun 22.",
        "Joel ruled it in 2026-11-22's thread.",
        "Joel said at 15:49 that security never blocks the work.",
        "Joel's rule from 10:05 stands.",
        "Joel ruled it. He said so on 22 Nov.",
        "Joel ruled it.\n\"Never block the work\", he said.",
    ];
    for text in backed {
        assert!(claims(text), "the case claims nothing, so tests nothing: {text}");
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
    ];
    for text in unbacked_ones {
        assert!(unbacked(text).is_some(), "an unbacked claim went uncounted: {text}");
    }
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
}

#[test]
fn cadences_own_asks_and_answers_to_them_are_left_out() {
    let ask = sent(
        "0000000d",
        1200,
        CADENCE,
        "cadence hour: your message 0000000a says Joel ruled or wants something.",
    );
    let mut answer = sent(
        "0000000e",
        600,
        "T6E7X6",
        "The claim is dropped: Joel's rule was my own caution.",
    );
    answer.re = Some(ask.id.clone());
    let m = unsourced(&[ask, answer], NOW);
    assert_eq!(m.behind, Vec::<String>::new(), "cadence's ask or its answer was counted");
    assert_eq!(m.owner, KEEPER, "with none unsourced, the keeper answers for it");
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

#[test]
fn with_no_messages_folder_the_measure_is_left_unjudged() {
    let home = std::env::temp_dir().join(format!("ruled-{}", std::process::id()));
    let m = read(&home, NOW);
    assert_eq!(m.target, None, "an unread mesh was judged as none unsourced");
    let messages = home.join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder couldn't be made");
    let m = read(&home, NOW);
    assert_eq!(m.target, Some(0.0), "a mesh with no messages went unjudged");
    fs::remove_dir_all(&home).expect("the test's folder couldn't be removed");
}
