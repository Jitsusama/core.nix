//! The tests of echo.rs, its parent: each rule that cuts the others'
//! echo from Joel's lines, held to calls made up word by word.

use super::*;
use crate::transcript::tests::{kept, line};

#[test]
fn joels_short_answer_to_a_longer_line_of_theirs_is_his_own() {
    // On the meetings, which hold no echo, each line of Joel's the
    // filter left out was a short answer like these, its few words
    // all in the longer line of theirs it answered.
    let theirs = "Okay, that's fine, yes, but let's check the budget.";
    for answer in ["Okay.", "Yes. Yes.", "Okay. That's fine."] {
        let lines = [line(Side::Theirs, 4, theirs), line(Side::Mine, 5, answer)];
        assert_eq!(
            kept(&lines),
            [answer],
            "Joel's answer {answer:?} was taken for their echo"
        );
    }
}

#[test]
fn joels_long_line_taking_up_their_words_is_his_own() {
    // Four of their five words are his, but under half of his.
    let mine = "Thursday morning works for me, but not for my team at all.";
    let lines = [
        line(Side::Theirs, 4, "Thursday morning works for everyone?"),
        line(Side::Mine, 5, mine),
    ];
    assert_eq!(
        kept(&lines),
        [mine],
        "Joel's long line was taken for their echo"
    );
}

#[test]
fn an_echo_of_a_short_line_of_theirs_is_still_left_out() {
    let lines = [
        line(Side::Theirs, 4, "Sounds good."),
        line(Side::Mine, 4, "Sounds good"),
    ];
    assert!(
        kept(&lines).is_empty(),
        "the echo of their short line was kept as Joel's"
    );
}

#[test]
fn four_words_of_a_longer_line_of_theirs_are_its_echo() {
    // Joel's microphone hears their long line as two takes where it
    // cuts at its own pauses, and each is the echo of part of it.
    let theirs = "Thursday morning works for everyone, then lunch at noon.";
    let lines = [
        line(Side::Theirs, 4, theirs),
        line(Side::Mine, 4, "Thursday morning works for"),
        line(Side::Mine, 6, "everyone then lunch at noon"),
    ];
    assert!(
        kept(&lines).is_empty(),
        "a part of their long line was kept as Joel's: {:?}",
        kept(&lines)
    );
}

/// `text`'s words, the first heard `from` milliseconds into the call
/// and each after 300 ms on.
fn timed(text: &str, from: usize) -> Vec<Word> {
    text.split_whitespace()
        .enumerate()
        .map(|(i, word)| Word {
            text: word.to_owned(),
            at: (from + 300 * i) * RATE / 1000,
            level: 0.0,
        })
        .collect()
}

/// `words` as heard at `level` dB.
fn at_level(mut words: Vec<Word>, level: f32) -> Vec<Word> {
    for word in &mut words {
        word.level = level;
    }
    words
}

/// A line of `side`'s holding `words`, from its first word to 300 ms
/// after its last.
fn worded(side: Side, words: &[Word]) -> Line {
    let joined: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
    Line {
        side,
        start: words[0].at,
        end: words[words.len() - 1].at + RATE * 3 / 10,
        text: joined.join(" "),
        voice: None,
        name: None,
    }
}

/// A line of each side's for each of its words as they were heard,
/// theirs first, and the words by line.
fn lines_heard(
    theirs: &[Vec<Word>],
    mine: &[Vec<Word>],
) -> (Vec<Line>, HashMap<usize, Vec<Word>>) {
    let mut lines = Vec::new();
    let mut words = HashMap::new();
    for (side, said) in [(Side::Theirs, theirs), (Side::Mine, mine)] {
        for heard in said {
            words.insert(lines.len(), heard.clone());
            lines.push(worded(side, heard));
        }
    }
    (lines, words)
}

/// `lines` with their echo cut by `words`, none of their sound known,
/// and what's left of each untidied.
fn cut_untidied(lines: &[Line], words: &HashMap<usize, Vec<Word>>) -> (Vec<Line>, Vec<Cut>) {
    cut_echoes(lines, words, &Track::default(), |left| Some(left.to_owned()))
}

/// Joel's lines as written once their echo is cut from them, each
/// side's words given as they were heard.
fn cut_from(theirs: &[Vec<Word>], mine: &[Vec<Word>]) -> Vec<String> {
    let (lines, words) = lines_heard(theirs, mine);
    let (cut, _) = cut_untidied(&lines, &words);
    kept(&cut)
}

#[test]
fn their_words_heard_just_after_them_are_cut_from_joels_line() {
    // As Joel's microphone heard a meeting, 50 ms after the far end
    // said it, with his own words before it in one take.
    let theirs = timed(
        "Please please leave me a space at the bottom, I'm little.",
        3_000,
    );
    let mut mine = timed("Uh yeah, okay. Sorry.", 1_800);
    mine.extend(timed(
        "Please please leave me a space at the bottom.",
        3_050,
    ));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        ["Uh yeah, okay. Sorry."],
        "their echo wasn't cut from Joel's line, or his own words went with it"
    );
}

#[test]
fn joels_short_answer_heard_with_their_echo_is_his_own() {
    // His answer is in their line too, but said before they said it.
    let theirs = timed("Yes, we could use a seagull, yes.", 2_000);
    let mut mine = timed("Yes yes.", 1_000);
    mine.extend(timed("we could use a seagull, yes.", 2_350));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        ["Yes yes."],
        "Joel's answer went with their echo"
    );
}

#[test]
fn joel_saying_their_words_back_after_them_is_his_own() {
    // They say "a seagull" from 2 s; Joel says it back from 3.5 s.
    let theirs = timed("Could it be a seagull?", 1_100);
    let mine = timed("A seagull, right.", 3_500);
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        ["A seagull, right."],
        "Joel's own words were cut as their echo"
    );
}

#[test]
fn joels_words_the_far_ends_microphone_picked_up_are_his_own() {
    // On AMI's meetings the others' headsets pick up Joel's voice as he
    // says it, far quieter than their own: this is his, not their echo.
    let theirs = [
        at_level(timed("So what should the remote do first then?", 0), -20.0),
        at_level(timed("control all your household", 5_040), -45.0),
    ];
    let mine = "I think it should control all your household devices.";
    assert_eq!(
        cut_from(&theirs, &[at_level(timed(mine, 3_800), -20.0)]),
        [mine],
        "Joel's words their microphone picked up were cut as their echo"
    );
}

#[test]
fn their_echo_quieter_in_joels_microphone_than_they_said_it_is_cut() {
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let own = "Fine by me, I'll book the room for us.";
    let mut mine = at_level(timed(own, 1_000), -20.0);
    mine.extend(at_level(
        timed("Lunch at noon on Thursday then.", 5_050),
        -40.0,
    ));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        [own],
        "their echo, quieter in his microphone than in theirs, wasn't cut"
    );
}

#[test]
fn their_quieter_words_echoed_are_cut_however_little_joel_says() {
    // His side mostly their echo, as TS3003a's is, so his usual level
    // is the echo's: their quieter words, echoed with his microphone's
    // hiss, are no quieter against it than they said them against theirs.
    let theirs = [
        at_level(timed("We could meet next week if that suits", 0), -20.0),
        at_level(timed("Lunch at noon on Thursday then.", 5_000), -30.0),
    ];
    let own = "Sure, I'll put it in the calendar for us all.";
    let mut mine = at_level(timed(own, 2_000), -40.0);
    mine.extend(at_level(
        timed("Lunch at noon on Thursday then.", 5_050),
        -45.0,
    ));
    let echo = at_level(timed("We could meet next week if that suits", 50), -40.0);
    assert_eq!(
        cut_from(&theirs, &[echo, mine]),
        [own],
        "their echo, quieter in his microphone than they said it, wasn't cut"
    );
}

#[test]
fn their_echo_misheard_far_below_what_they_said_is_cut() {
    // Each side's model hears the echo its own way, so his words for
    // it may spell none of theirs; far quieter than they said them,
    // as echo comes back 15 to 35 dB down, they're still theirs.
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let own = "Fine by me, I'll book the room for us.";
    let mut mine = at_level(timed(own, 1_000), -20.0);
    mine.extend(at_level(timed("Bunch had moon in Tuesday", 5_050), -42.0));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        [own],
        "their echo, misheard and far quieter than they said it, wasn't cut"
    );
}

#[test]
fn their_echo_misheard_far_below_them_is_cut_however_little_joel_says_alone() {
    // Where Joel says little, too few of his words are heard alone to
    // know his voice, while his side is mostly their echo: that's then
    // told by how far below theirs it is alone.
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let mut mine = at_level(timed("Fine.", 1_000), -20.0);
    mine.extend(at_level(timed("Bunch had moon in Tuesday", 5_050), -42.0));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        ["Fine."],
        "their echo, misheard far below them where Joel says little, wasn't cut"
    );
}

#[test]
fn joel_talking_over_them_near_their_level_is_his_own() {
    // His own voice in his microphone is near theirs in theirs, so
    // words of his said over them stay his, however they're spelled.
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let own = "Hang on, Friday suits me better.";
    assert_eq!(
        cut_from(&[theirs], &[at_level(timed(own, 5_100), -26.0)]),
        [own],
        "Joel talking over them was cut as their echo"
    );
}

#[test]
fn joel_talking_over_them_through_a_quiet_microphone_is_his_own() {
    // Each side's level is its own recorder's, so a quiet microphone
    // or a loud call app puts all of his voice far below theirs: his
    // words over them are as loud as his words alone, and so his. His
    // two lines have none of theirs between, so they're written as one.
    let theirs = at_level(timed("We could meet next week if that suits", 0), -20.0);
    let over = "Hang on, wait.";
    let alone = "Fine by me, I'll book the room for us.";
    let mine = [
        at_level(timed(over, 600), -40.0),
        at_level(timed(alone, 10_000), -40.0),
    ];
    assert_eq!(
        cut_from(&[theirs], &mine),
        [format!("{over} {alone}")],
        "Joel talking over them through a quiet microphone was cut as their echo"
    );
}

#[test]
fn joels_answer_in_the_take_holding_their_echo_is_his_own() {
    // His take may run on from the echo of their last words into his
    // answer: what follows the echo at his own voice's level is his.
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let own = "Fine by me, I'll book the room for us.";
    let answer = "Sure, that works.";
    let mut mine = at_level(timed(own, 1_000), -35.0);
    mine.extend(at_level(timed("Bunch had moon in Tuesday", 5_050), -52.0));
    mine.extend(at_level(timed(answer, 6_600), -35.0));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        [format!("{own} {answer}")],
        "Joel's answer after their echo in one take was cut as their echo"
    );
}

/// Joel's line saying `own` alone at `voice` dB, then `heard` within
/// their "Lunch at noon on Thursday then." at -20 dB, as written once
/// their echo is cut.
fn heard_under_their_lunch(voice: f32, heard: &[Vec<Word>]) -> Vec<String> {
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let mut mine = at_level(
        timed("Fine by me, I'll book the room for us.", 1_000),
        voice,
    );
    mine.extend(heard.iter().flatten().cloned());
    cut_from(&[theirs], &[mine])
}

#[test]
fn a_word_is_drowned_from_fifteen_db_below_theirs() {
    let own = "Fine by me, I'll book the room for us.";
    for (level, written) in [
        (-35.0, own.to_owned()),
        (-34.5, format!("{own} Bunch had moon")),
    ] {
        let heard = at_level(timed("Bunch had moon", 5_050), level);
        assert_eq!(
            heard_under_their_lunch(-20.0, &[heard]),
            [written],
            "a word {level} dB to their -20 was judged wrongly"
        );
    }
}

#[test]
fn a_word_is_drowned_only_from_twelve_db_below_his_voice() {
    let own = "Fine by me, I'll book the room for us.";
    for (level, written) in [
        (-42.0, own.to_owned()),
        (-41.0, format!("{own} Bunch had moon")),
    ] {
        let heard = at_level(timed("Bunch had moon", 5_050), level);
        assert_eq!(
            heard_under_their_lunch(-30.0, &[heard]),
            [written],
            "a word {level} dB to his voice's -30 was judged wrongly"
        );
    }
}

#[test]
fn the_echo_of_their_last_words_reaches_under_a_second_past_his_last_cut() {
    // Their last word is within a second of his "Right.", and the echo
    // cut before it ends 950 ms or 1,050 ms before it: only the first
    // is still their echo, the second his answer.
    let own = "Fine by me, I'll book the room for us.";
    for (from_ms, written) in [(7_200, own.to_owned()), (7_300, format!("{own} Right."))] {
        let echo = at_level(timed("Bunch had moon in Tuesday", 5_050), -42.0);
        let right = at_level(timed("Right.", from_ms), -42.0);
        assert_eq!(
            heard_under_their_lunch(-20.0, &[echo, right]),
            [written],
            "a word {from_ms} ms in, past the echo ending at 6250 ms, was judged wrongly"
        );
    }
}

#[test]
fn a_word_far_below_theirs_is_their_echo_only_while_they_go_on_speaking() {
    // Their words run 5,000 to 6,500 ms. At 5,300 ms more of theirs
    // follows within a second, so it's their echo; at 6,500 ms, on
    // their last word, none follows and none of his was cut before,
    // so it may be his answer, however far below them.
    let own = "Fine by me, I'll book the room for us.";
    for (from_ms, written) in [(5_300, own.to_owned()), (6_500, format!("{own} Right."))] {
        let right = at_level(timed("Right.", from_ms), -42.0);
        assert_eq!(
            heard_under_their_lunch(-20.0, &[right]),
            [written],
            "a word {from_ms} ms in, against their last word at 6500 ms, was judged wrongly"
        );
    }
}

#[test]
fn a_word_far_below_theirs_is_held_to_his_voice_once_five_of_his_are_heard_alone() {
    // Words 16 dB below theirs but near his voice's -40 dB are cut while
    // four of his are heard alone, and kept as his from the fifth.
    let misheard = "Bunch had moon in Tuesday";
    for (own, written) in [
        ("Fine by me, sure.", "Fine by me, sure.".to_owned()),
        (
            "Fine by me, sure then.",
            format!("Fine by me, sure then. {misheard}"),
        ),
    ] {
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let mut mine = at_level(timed(own, 1_000), -40.0);
        mine.extend(at_level(timed(misheard, 5_050), -36.0));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            [written],
            "their echo after Joel's {own:?} alone was judged wrongly"
        );
    }
}

#[test]
fn his_voice_is_the_middle_level_of_his_words_heard_alone() {
    // His words alone at -20, -30 and -44 dB: his voice is the -30
    // between, not the loudest, the quietest or their mean, -31.8.
    let own = "Fine by me, I'll book the room for us.";
    let levels = [
        -20.0, -44.0, -20.0, -44.0, -30.0, -44.0, -20.0, -44.0, -20.0,
    ];
    for (level, written) in [
        (-42.0, own.to_owned()),
        (-41.0, format!("{own} Bunch had moon")),
    ] {
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let mut mine = timed(own, 1_000);
        for (word, alone) in mine.iter_mut().zip(levels) {
            word.level = alone;
        }
        mine.extend(at_level(timed("Bunch had moon", 5_050), level));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            [written],
            "a word {level} dB to his voice's -30 among louder and quieter was judged wrongly"
        );
    }
}

#[test]
fn his_voice_between_two_middle_levels_is_the_quieter() {
    // Six words alone, three at -20 and three at -40 dB: judged by the
    // louder middle, or by the two middles' mean, -30, his word at -42
    // would be cut as 12 dB below his voice; by the quieter it's his, as
    // a doubtful word should be.
    let own = "Fine by me, sure then, thanks.";
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let mut mine = timed(own, 1_000);
    let levels = [-20.0, -40.0, -20.0, -40.0, -20.0, -40.0];
    for (word, alone) in mine.iter_mut().zip(levels) {
        word.level = alone;
    }
    mine.extend(at_level(timed("Bunch had moon", 5_050), -42.0));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        [format!("{own} Bunch had moon")],
        "a word near the quieter of his voice's two middle levels was cut"
    );
}

#[test]
fn a_word_only_joels_microphone_heard_doesnt_break_their_echo() {
    // Echo comes in quiet over his room, so his side may hear a word
    // in it theirs didn't: their "noon" must still be cut after it.
    let theirs = timed("Lunch at noon", 5_000);
    let mut mine = timed("Fine by me.", 1_000);
    mine.extend(timed("Lunch at the noon", 5_050));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        ["Fine by me. the"],
        "a word of theirs after one only Joel's side heard was written as his"
    );
}

#[test]
fn one_word_of_joels_heard_just_after_theirs_is_his_own() {
    // A lone "Okay" is as likely his own as their echo.
    let theirs = timed("Okay so the next item", 2_000);
    let mut mine = timed("Right, I see.", 1_100);
    mine.extend(timed("Okay", 2_050));
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        ["Right, I see. Okay"],
        "Joel's lone word was cut as their echo"
    );
}

#[test]
fn joels_words_said_well_before_theirs_are_his_own() {
    // Echo is heard from 300 ms before their word, by each side's frame,
    // to 400 ms after; Joel's "so it goes" comes 400 ms before theirs.
    let theirs = timed("So it goes.", 5_400);
    let mine = "Yeah, so it goes with these things every time.";
    assert_eq!(
        cut_from(&[theirs], &[timed(mine, 4_700)]),
        [mine],
        "Joel's words said before theirs were cut as their echo"
    );
}

#[test]
fn a_line_of_joels_all_their_echo_is_left_out() {
    let theirs = timed("Flying around everywhere.", 8_000);
    let mine = timed("Flying around everywhere.", 8_080);
    assert!(
        cut_from(&[theirs], &[mine]).is_empty(),
        "a line of Joel's that's all their echo was written"
    );
}

#[test]
fn what_was_cut_says_where_it_was_and_how_late() {
    let theirs = timed("Lunch at noon", 5_000);
    let mine = timed("Fine. Lunch at noon", 4_700);
    let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
    let words = HashMap::from([(0, theirs), (1, mine)]);
    let (cut, cuts) = cut_untidied(&lines, &words);
    assert_eq!(cut[1].text, "Fine.");
    assert_eq!(
        cuts,
        [Cut {
            line: 1,
            at: 5_000 * RATE / 1000,
            after: 0,
            text: "Lunch at noon".to_owned(),
            rule: Rule::Spelling,
        }]
    );
}

#[test]
fn what_was_cut_for_its_level_says_how_far_below_theirs_it_was() {
    // A word cut for its level echoes none of theirs in particular,
    // so how late it came says nothing; how far below them it was
    // does, where the margin is judged on a live call. Their words
    // spelled alike are cut for that, as before. It's from the loudest
    // of theirs around the first word cut, -20 dB beside -25, to that
    // word, -42 dB, before quieter ones at -45.
    let mut theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    theirs[0].level = -25.0;
    let own = at_level(
        timed("Fine by me, I'll book the room for us.", 1_000),
        -20.0,
    );
    for (heard, rule) in [
        ("Bunch had moon in Tuesday", Rule::Level(22.0)),
        ("Lunch at noon on Thursday then.", Rule::Spelling),
    ] {
        let mut mine = own.clone();
        let mut run = at_level(timed(heard, 5_050), -45.0);
        run[0].level = -42.0;
        mine.extend(run);
        let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
        let words = HashMap::from([(0, theirs.clone()), (1, mine)]);
        let (_, cuts) = cut_untidied(&lines, &words);
        let told: Vec<(&str, Rule)> = cuts
            .iter()
            .map(|cut| (cut.text.as_str(), cut.rule))
            .collect();
        assert_eq!(
            told,
            [(heard, rule)],
            "the cut of {heard:?} didn't say how far below theirs it was by level alone"
        );
    }
}

#[test]
fn a_cut_says_what_it_was_cut_by() {
    let cut = |after_ms: isize, rule| Cut {
        line: 1,
        at: 5_000 * RATE / 1000,
        after: after_ms * isize::try_from(RATE).expect("a rate fits") / 1000,
        text: "Lunch at noon".to_owned(),
        rule,
    };
    assert_eq!(
        [
            cut(-200, Rule::Spelling).by(),
            cut(50, Rule::Spelling).by(),
            cut(50, Rule::Level(21.6)).by(),
            cut(50, Rule::Sound(20.4)).by(),
        ],
        [
            "200 ms before theirs",
            "50 ms after theirs",
            "22 dB below theirs",
            "20 dB under their sound",
        ],
        "a cut didn't say the rule it was cut by"
    );
}

#[test]
fn a_run_of_cuts_splits_where_the_rule_cutting_it_changes() {
    // "Lunch at noon" is cut for its spelling, then "in Tuesday" for
    // its level: each cut says what it was cut by.
    let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
    let mut mine = at_level(
        timed("Fine by me, I'll book the room for us.", 1_000),
        -20.0,
    );
    mine.extend(at_level(timed("Lunch at noon in Tuesday", 5_050), -42.0));
    let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
    let words = HashMap::from([(0, theirs), (1, mine)]);
    let (_, cuts) = cut_untidied(&lines, &words);
    let told: Vec<(&str, Rule)> = cuts
        .iter()
        .map(|cut| (cut.text.as_str(), cut.rule))
        .collect();
    assert_eq!(
        told,
        [("Lunch at noon", Rule::Spelling), ("in Tuesday", Rule::Level(22.0))],
        "a run cut by spelling, then by level, was told as cut by one rule"
    );
}

/// What's written of Joel's line when he says `headset` alone at -20
/// dB from the call's start, `laptop` alone at -40 dB from `laptop_ms`,
/// and "Hang on, wait." at -40 dB over their -20 dB line at `over_ms`.
fn over_them_after(laptop: &str, laptop_ms: usize, over_ms: usize) -> Vec<String> {
    let headset = "Morning all, I'm on my headset for now, so let me know if the \
                   sound drops out at any point.";
    let mut mine = at_level(timed(headset, 0), -20.0);
    mine.extend(at_level(timed(laptop, laptop_ms), -40.0));
    mine.extend(at_level(timed("Hang on, wait.", over_ms), -40.0));
    mine.sort_by_key(|word| word.at);
    let theirs = at_level(
        timed("We could meet next week if that suits", over_ms - 100),
        -20.0,
    );
    cut_from(&[theirs], &[mine])
}

#[test]
fn his_voice_follows_a_new_microphone_from_six_of_his_words_heard_alone_on_it() {
    // Most of what he says alone is on a headset, 20 dB louder than the
    // laptop he changes to, and his voice is what it is near his word,
    // not what it was for most of the call: of the eleven words alone
    // nearest his words over theirs, six on the laptop are the middle of
    // them, five aren't.
    for (laptop, kept) in [
        ("Can you hear me now?", false),
        ("Can you all hear me now?", true),
    ] {
        let written = over_them_after(laptop, 60_000, 70_100);
        let joels = written.first().expect("none of Joel's line was written");
        assert_eq!(
            joels.ends_with("Hang on, wait."),
            kept,
            "Joel over them after {laptop:?} on a quieter microphone was judged wrongly"
        );
    }
}

#[test]
fn his_voice_is_taken_from_his_words_alone_just_after_as_well_as_before() {
    // He talks over them on the laptop, says eight words alone on it
    // just after, and was on a headset long before: his voice is the
    // laptop's, nearest in time, though every earlier word is louder.
    let written = over_them_after("Sorry, I've moved to my laptop just now.", 34_000, 30_100);
    let joels = written.first().expect("none of Joel's line was written");
    assert!(
        joels.contains("Hang on, wait."),
        "Joel over them was judged by his headset long before, not his laptop just after"
    );
}

#[test]
fn his_voice_is_taken_from_no_more_than_the_eleven_of_his_words_alone_nearest() {
    // Nearest his words over theirs are five of his alone on the laptop,
    // then six on a headset 20 dB louder, then one more on the laptop,
    // furthest out. From the eleven nearest his voice is the headset's,
    // so his words over theirs are cut; from twelve, it would be the
    // quieter of two middles, the laptop's, and they'd be kept.
    let headset = "Morning all, I'm on my headset.";
    let laptop = "Can you hear me now?";
    let mut mine = at_level(timed(headset, 0), -20.0);
    mine.extend(at_level(timed(laptop, 20_000), -40.0));
    mine.extend(at_level(timed("Hang on, wait.", 22_000), -40.0));
    mine.extend(at_level(timed("Thanks.", 50_000), -40.0));
    let theirs = at_level(
        timed("We could meet next week if that suits", 21_900),
        -20.0,
    );
    assert_eq!(
        cut_from(&[theirs], &[mine]),
        [format!("{headset} {laptop} Thanks.")],
        "Joel over them was judged by more than the eleven of his words alone nearest"
    );
}

#[test]
fn their_echo_timed_just_before_their_word_is_cut_and_says_how_early() {
    // Each side's words are timed by its own encoder frames, so echo
    // can be timed before the word it echoes, as on AMI's meetings.
    let theirs = timed("Lunch at noon on Thursday then.", 5_000);
    let mut mine = timed("Fine by me.", 1_000);
    mine.extend(timed("Lunch at noon on Thursday then.", 4_800));
    let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
    let words = HashMap::from([(0, theirs), (1, mine)]);
    let (cut, cuts) = cut_untidied(&lines, &words);
    assert_eq!(
        cut[1].text, "Fine by me.",
        "their echo timed 200 ms before their words wasn't cut"
    );
    let early = isize::try_from(200 * RATE / 1000).unwrap();
    assert_eq!(
        cuts.iter().map(|cut| cut.after).collect::<Vec<_>>(),
        [-early],
        "the cut didn't say it was heard 200 ms before their word"
    );
}

#[test]
fn a_line_whose_opening_echo_is_cut_starts_at_joels_first_word() {
    // His microphone hears their "Lunch at noon" before he answers in
    // the same take, so what's left of it starts when he does.
    let theirs = timed("Lunch at noon", 5_000);
    let mut mine = timed("Lunch at noon", 5_050);
    mine.extend(timed("Fine by me.", 6_500));
    let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
    let words = HashMap::from([(0, theirs), (1, mine)]);
    let (cut, _) = cut_untidied(&lines, &words);
    assert_eq!(cut[1].text, "Fine by me.", "their echo wasn't cut");
    assert_eq!(
        cut[1].start,
        6_500 * RATE / 1000,
        "Joel's line starts where their echo did, so it's written before what he answers"
    );
}

#[test]
fn a_line_whose_words_are_unknown_is_cut_from_by_its_text_alone() {
    // Lines read back after a restart have no words; the whole-line
    // rule still leaves their echo out where the transcript's written.
    let lines = [
        line(Side::Theirs, 4, "Sounds good."),
        line(Side::Mine, 4, "Sounds good"),
    ];
    let (cut, cuts) = cut_untidied(&lines, &HashMap::new());
    assert_eq!(cut, lines);
    assert_eq!(
        cuts,
        Vec::<Cut>::new(),
        "words were cut from a line whose words are unknown"
    );
    assert!(
        kept(&cut).is_empty(),
        "the whole-line rule didn't leave the echo out"
    );
}

/// The cuts of Joel's words far under their sound, built on the calls
/// above.
#[path = "echo_sound_tests.rs"]
mod sound;
