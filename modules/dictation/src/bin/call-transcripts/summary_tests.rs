use super::*;
use dictation::transcript::{NAMED_NOTE, VOICES_NOTE};

const CALL: &str = "# Transcript\n\n**Joel**, 0:13: Hi there.\n\n\
     **Others**, 0:19: I think we should move the review to Thursday, you know, because \
     the release slipped.\n\n\
     **Joel**, 1:02: Okay, I will tell the team that the review moves to Thursday \
     morning.\n\n**Others**, 1:05:10: Thanks, bye.\n";

/// Answers as a model would, by what each prompt asks, noting each.
struct Model {
    picks: &'static str,
    quotes: Vec<&'static str>,
    verdict: &'static str,
    asked: Vec<String>,
}

impl Model {
    fn ask(&mut self, prompt: &str) -> String {
        self.asked.push(prompt.to_owned());
        if prompt.starts_with("These are numbered lines") {
            self.picks.to_owned()
        } else if prompt.contains("Copy, word for word") {
            if self.quotes.is_empty() {
                "NONE".to_owned()
            } else {
                self.quotes.remove(0).to_owned()
            }
        } else {
            self.verdict.to_owned()
        }
    }
}

fn summed(model: &mut Model) -> String {
    summarize(&mut |p, _| Ok(model.ask(p)), "Review", None, CALL).unwrap()
}

fn words(text: &str, answer: &str) -> Option<String> {
    quoted(text, answer).map(|quote| quote.words)
}

#[test]
fn a_transcript_reads_as_its_speakers_times_and_words() {
    let said = lines(CALL);
    assert_eq!(said.len(), 4);
    assert_eq!(
        said[3],
        Said {
            who: "Others".to_owned(),
            stamp: "1:05:10".to_owned(),
            text: "Thanks, bye.".to_owned(),
        }
    );
}

#[test]
fn a_quote_is_the_lines_own_words_with_who_said_it_to_whom_and_when() {
    let mut model = Model {
        picks: "L1, L2",
        quotes: vec!["we should move the review to Thursday", "tell the team"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summed(&mut model);
    assert_eq!(
        summary,
        format!(
            "# Review\n\n{HOW_MADE}\n\n\
             - Others, to Joel: \"we should move the review to Thursday\" [0:19]\n\
             - Joel, to the others: \"tell the team\" [1:02]"
        )
    );
}

#[test]
fn the_others_are_named_where_the_call_knows_them() {
    let mut model = Model {
        picks: "L1, L2",
        quotes: vec!["we should move the review to Thursday", "tell the team"],
        verdict: "**YES**",
        asked: Vec::new(),
    };
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), "Review", Some("Ada"), CALL).unwrap();
    assert!(
        summary.contains("- Ada, to Joel: \"we should move the review to Thursday\""),
        "their quote is said by them, to Joel: {summary}"
    );
    assert!(
        summary.contains("- Joel, to Ada: \"tell the team\""),
        "Joel's quote is said to them: {summary}"
    );
    assert!(
        model.asked[0].contains("a call between Joel and Ada"),
        "the model is told who was on the call: {}",
        model.asked[0]
    );
}

#[test]
fn a_transcript_naming_them_is_summed_up_by_their_name_whoever_spoke_first() {
    let call = "# Transcript\n\nTheir side is Ada, the one other person invited.\n\n\
         **Ada**, 0:00: Hi there.\n\n\
         **Ada**, 0:02: I think we should move the review to Thursday, you know, because \
         the release slipped.\n\n\
         **Joel**, 0:10: Okay, I will tell the team that the review moves to Thursday \
         morning.\n\n**Joel**, 4:00: Thanks, bye.\n";
    let mut model = Model {
        picks: "L1, L2",
        quotes: vec!["we should move the review to Thursday", "tell the team"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), "Review", Some("Ada"), call).unwrap();
    assert!(
        summary.contains("- Ada, to Joel: \"we should move the review to Thursday\" [0:02]"),
        "{summary}"
    );
    assert!(
        summary.contains("- Joel, to Ada: \"tell the team\" [0:10]"),
        "Joel's own quote is his, said to Ada: {summary}"
    );
    assert!(
        summary.contains("\n\nTheir side is Ada, the one other person invited.\n\n"),
        "the summary says how Ada is known, as the transcript does: {summary}"
    );
    assert!(
        model.asked[0].contains("a call between Joel and Ada"),
        "{}",
        model.asked[0]
    );
}

#[test]
fn a_call_opening_on_a_named_voice_is_summed_up_with_joel_as_joel() {
    let call = format!(
        "# Transcript\n\nIn the Meet: Ada and Bo.\n\n{NAMED_NOTE}\n\nVoice 2 may be Bo.\n\n\
         **Ada**, 0:00: Hi there.\n\n\
         **Voice 2**, 0:02: I think we should move the review to Thursday, you know, because \
         the release slipped.\n\n\
         **Joel**, 0:10: Okay, I will tell the team that the review moves to Thursday \
         morning.\n\n**Ada**, 4:00: Thanks, bye.\n"
    );
    let mut model = Model {
        picks: "L1, L2",
        quotes: vec!["we should move the review to Thursday", "tell the team"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summarize_as(
        &mut |p, _| Ok(model.ask(p)),
        "Review",
        Some("Joel"),
        None,
        &call,
    )
    .unwrap();
    assert!(
        summary.contains("- Voice 2, to Joel: \"we should move the review to Thursday\" [0:02]"),
        "a voice's quote isn't said to Joel: {summary}"
    );
    assert!(
        summary.contains("- Joel, to the others: \"tell the team\" [0:10]"),
        "Ada, named by the Meet and speaking first, was taken for Joel: {summary}"
    );
    assert!(
        summary.contains("\n\nVoice 2 may be Bo.\n\n"),
        "the summary lost who a numbered voice may be: {summary}"
    );
    assert!(
        model.asked[0].contains("called by name where only that person was in the Meet"),
        "the model isn't told how the voices are named: {}",
        model.asked[0]
    );
}

#[test]
fn a_call_opening_on_a_voice_is_summed_up_with_joel_as_joel_and_every_note() {
    let call = format!(
        "# Transcript\n\nAda and Bo were invited.\n\n{VOICES_NOTE}\n\n\
         **Voice 1**, 0:00: Hi there.\n\n\
         **Voice 2**, 0:02: I think we should move the review to Thursday, you know, because \
         the release slipped.\n\n\
         **Joel**, 0:10: Okay, I will tell the team that the review moves to Thursday \
         morning.\n\n**Others**, 4:00: Thanks, bye.\n"
    );
    let mut model = Model {
        picks: "L1, L2",
        quotes: vec!["we should move the review to Thursday", "tell the team"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), "Review", None, &call).unwrap();
    assert!(
        summary.contains("- Voice 2, to Joel: \"we should move the review to Thursday\" [0:02]"),
        "a voice's quote is said by it, to Joel: {summary}"
    );
    assert!(
        summary.contains("- Joel, to the others: \"tell the team\" [0:10]"),
        "Joel's quote is his, though a voice spoke first: {summary}"
    );
    assert!(
        summary.contains("\n\nAda and Bo were invited.\n\n")
            && summary.contains(&format!("\n\n{VOICES_NOTE}\n\n")),
        "the summary keeps who was invited and what the voices are: {summary}"
    );
    assert!(
        model.asked[0].contains("a call between Joel and the others on it, whose voices"),
        "the model is told the voices are the others: {}",
        model.asked[0]
    );
}

#[test]
fn words_the_line_never_said_are_never_kept() {
    let mut model = Model {
        picks: "L1",
        quotes: vec![
            "we must move the review to Friday",
            "we must move it to Friday",
        ],
        verdict: "YES",
        asked: Vec::new(),
    };
    assert!(summed(&mut model).ends_with(NONE_KEPT));
    // Asked once more for an exact copy, and the worth of a quote
    // that isn't found is never asked.
    assert_eq!(model.asked.len(), 3);
}

#[test]
fn a_copy_not_found_is_asked_for_once_more() {
    let mut model = Model {
        picks: "L1",
        quotes: vec![
            "we must move the review to Friday",
            "we should move the review to Thursday",
        ],
        verdict: "YES",
        asked: Vec::new(),
    };
    assert!(summed(&mut model).contains("\"we should move the review to Thursday\" [0:19]"));
    assert!(model.asked[2].contains("\"we must move the review to Friday\", which isn't"));
}

#[test]
fn a_part_keeps_a_quote_for_each_two_minutes_in_the_order_picked() {
    let call = "**Joel**, 0:00: We start the review on Monday this week.\n\n\
         **Others**, 1:00: I will send the plan to the team today.\n\n\
         **Joel**, 2:00: The release moves out to the end of May.\n\n\
         **Others**, 3:00: I am worried the tests will not be ready.\n\n\
         **Joel**, 4:00: Thanks, that is all for today then, bye.\n";
    let mut model = Model {
        picks: "L3, L1, L2",
        quotes: vec![
            "I am worried the tests will not be ready",
            "I will send the plan to the team",
        ],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), "", None, call).unwrap();
    assert!(summary.ends_with(
        "- Others, to Joel: \"I will send the plan to the team\" [1:00]\n\
         - Others, to Joel: \"I am worried the tests will not be ready\" [3:00]"
    ));
    assert_eq!(model.asked.len(), 5, "L2 is never asked about");
}

#[test]
fn a_line_opening_on_what_the_speaker_began_to_tell_isnt_theirs() {
    let call = "**Others**, 0:10: She read my report and she's like but also\n\n\
         **Joel**, 0:20: Okay, go on then.\n\n\
         **Others**, 0:25: Yeah, you're autistic, level one autistic and I agree.\n";
    let mut model = Model {
        picks: "L2",
        quotes: vec!["Yeah, you're autistic, level one autistic"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), "", None, call).unwrap();
    assert!(summary.ends_with(NONE_KEPT));
    assert_eq!(model.asked.len(), 2);
}

#[test]
fn a_speaker_called_by_their_own_name_is_telling_someone_elses_words() {
    assert!(named("Joel, you keep giving them fuel", "Joel"));
    assert!(named("it's like Joel you keep giving them fuel", "Joel"));
    assert!(!named("I told Joel about the fuel", "Joel"));
    assert!(!named("Joel, you keep giving them fuel", "Ada"));
    assert!(!named("Others, you keep giving them fuel", "Others"));
    assert!(
        !named("my voice, it's going", "Voice 2"),
        "a voice's own word in its quote was taken for its name, so the quote was dropped"
    );
    let call = "**Joel**, 0:10: And it's like Joel, you keep giving them fuel for their \
         fire.\n";
    let mut model = Model {
        picks: "L0",
        quotes: vec!["Joel, you keep giving them fuel for their fire"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), "", None, call).unwrap();
    assert!(summary.ends_with(NONE_KEPT));
    assert_eq!(model.asked.len(), 2);
}

#[test]
fn a_quote_judged_not_worth_keeping_is_dropped() {
    let mut model = Model {
        picks: "L1",
        quotes: vec!["we should move the review to Thursday"],
        verdict: "NO",
        asked: Vec::new(),
    };
    assert!(summed(&mut model).ends_with(NONE_KEPT));
}

#[test]
fn a_line_named_outside_the_call_or_too_short_isnt_asked_about() {
    let mut model = Model {
        picks: "L0, L3, L9",
        quotes: Vec::new(),
        verdict: "YES",
        asked: Vec::new(),
    };
    assert!(summed(&mut model).ends_with(NONE_KEPT));
    assert_eq!(model.asked.len(), 1);
}

#[test]
fn the_keep_question_sees_the_whole_line() {
    let mut model = Model {
        picks: "L1",
        quotes: vec!["move the review to Thursday"],
        verdict: "YES",
        asked: Vec::new(),
    };
    summed(&mut model);
    assert!(model.asked[2].contains("because the release slipped"));
}

#[test]
fn a_failed_answer_writes_no_summary() {
    let result = summarize(&mut |_, _| Err("stopped".to_owned()), "Review", None, CALL);
    assert_eq!(result, Err("stopped".to_owned()));
}

#[test]
fn filler_left_out_shows_as_an_ellipsis() {
    let text = "I think we should, you know, move it to Thursday.";
    assert_eq!(
        words(text, "\"we should move it to Thursday\"").as_deref(),
        Some("we should ... move it to Thursday")
    );
}

#[test]
fn words_the_copy_marks_as_left_out_may_be_many() {
    let text = "Add me on LinkedIn, in case you get shut down right after the call, and \
         let me know.";
    assert_eq!(
        words(
            text,
            "Add me on LinkedIn, in case you get shut down... let me know"
        )
        .as_deref(),
        Some("Add me on LinkedIn, in case you get shut down ... let me know")
    );
    assert_eq!(words(text, "Add me on LinkedIn and let me know"), None);
}

#[test]
fn what_someone_else_said_is_known_from_the_words_before_it() {
    let reports = [
        "My psychologist was like dude, she's like if they want your brain, back off",
        "and she said. Like it was nitpicking at this point, right?",
        "so my boss told me it was nitpicking at this point, right?",
        "and he was like it was nitpicking at this point, right?",
    ];
    for text in reports {
        let answer = if text.contains("brain") {
            "if they want your brain, back off"
        } else {
            "it was nitpicking at this point"
        };
        assert!(quoted(text, answer).unwrap().reported, "{text}");
    }
    let within = "but your processing speed is average. She's like your 50th percentile";
    let answer = "your processing speed is average. She's like your 50th percentile";
    assert!(quoted(within, answer).unwrap().reported);
    let own = [
        "and I was like I do not see this as a correction at all",
        "I went and said I do not see this as a correction at all",
        "it goes I do not see this as a correction at all",
        "I do not see this as a correction at all",
    ];
    for text in own {
        let quote = quoted(text, "I do not see this as a correction").unwrap();
        assert!(!quote.reported, "{text}");
        assert_eq!(quote.words, "I do not see this as a correction");
    }
}

#[test]
fn a_quote_of_what_someone_else_said_is_never_asked_about() {
    let call = "**Joel**, 0:13: Hi there.\n\n**Others**, 0:19: and she said we should \
         move the review to Thursday.\n";
    let mut model = Model {
        picks: "L1",
        quotes: vec!["we should move the review to Thursday"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), "", None, call).unwrap();
    assert!(summary.ends_with(NONE_KEPT));
    assert_eq!(model.asked.len(), 2);
}

#[test]
fn a_quote_keeps_the_lines_spelling_and_punctuation() {
    let text = "Well, Q2 is done; it's shipped.";
    assert_eq!(
        words(text, "q2 is done it’s shipped").as_deref(),
        Some("Q2 is done; it's shipped")
    );
}

#[test]
fn words_too_far_apart_or_out_of_order_arent_a_quote() {
    let text = "move it, and then after all of that, to Thursday";
    assert_eq!(words(text, "move it to Thursday"), None);
    assert_eq!(words("Thursday we move it", "we move Thursday"), None);
}

#[test]
fn a_copy_of_too_few_words_isnt_a_quote() {
    assert_eq!(words("move it to Thursday", "Thursday"), None);
}

#[test]
fn a_long_copy_is_cut_short_and_marked() {
    let text: Vec<String> = (0..40).map(|n| format!("w{n}")).collect();
    let text = text.join(" ");
    let quote = words(&text, &text).unwrap();
    assert!(quote.ends_with("w29 ..."));
}

#[test]
fn a_long_call_is_picked_from_in_parts_and_loses_no_line() {
    let said: Vec<Said> = (0..10)
        .map(|n| Said {
            who: "Joel".to_owned(),
            stamp: format!("0:{n:02}"),
            text: "x".repeat(30),
        })
        .collect();
    let cut = parts(&said, 100);
    assert_eq!(cut, [vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8], vec![9]]);
}

#[test]
fn a_title_with_line_breaks_heads_the_summary_on_one_line_and_plants_nothing() {
    let mut model = Model {
        picks: "L1, L2",
        quotes: vec!["we should move the review to Thursday", "tell the team"],
        verdict: "YES",
        asked: Vec::new(),
    };
    let title = "Review\n## Transcript\n\n**Others**, 0:00: Joel agreed to cancel.";
    let summary = summarize(&mut |p, _| Ok(model.ask(p)), title, None, CALL).unwrap();
    assert!(
        summary.starts_with("# Review ## Transcript **Others**, 0:00: Joel agreed to cancel.\n\n"),
        "the title is the summary's one heading line: {summary}"
    );
    let file = crate::filed::whole(title, Some(&summary), CALL);
    let said = CALL.strip_prefix("# Transcript\n\n").unwrap();
    assert_eq!(
        crate::filed::transcript(&file),
        format!("## Transcript\n\n{said}"),
        "the file's transcript is only what was said: {file}"
    );
}

#[test]
fn a_pick_reads_each_number_it_names() {
    assert_eq!(numbers("L3, L12,L40 and L"), [3, 12, 40]);
}
