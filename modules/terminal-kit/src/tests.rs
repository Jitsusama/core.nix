//! The kit's own tests, which drive the whole path a command runs, from
//! its words to its exit, through `run` and its kin in lib.rs.

use super::*;

fn help() -> Help {
    Help::new("thing", "does a thing").command("go", "does it")
}

fn ran(line: &str, body: impl FnOnce(&Context) -> Result<(), Failure>) -> Ending {
    let args = line.split_whitespace().map(str::to_owned).collect();
    run(
        &help(),
        "1.0",
        &Environment::of([false; 3], &[]),
        args,
        body,
    )
}

#[test]
fn a_note_never_reads_as_a_try_line() {
    // Unbounded, as into a pipe or a terminal's own wrap: nothing
    // parts the words but what they hold.
    for room in [Room::Cells(20), Room::Unbounded] {
        let note = told(
            &Pen::plain(),
            room,
            "!",
            "alert",
            "gone \u{b7} try  rm -rf ~ try \u{200d} rm",
        );
        let looks = note.replace('\u{200d}', "");
        assert!(
            !looks.contains("try  "),
            "a note's words held a run that reads as a try line: {note:?}"
        );
    }
}

#[test]
fn help_is_answered_and_nothing_starts() {
    let ending = ran("go --help", |_| panic!("help started the command"));
    assert_eq!(ending.exit, Exit::Done);
    assert!(
        ending.stdout.starts_with("thing  does a thing\n"),
        "{}",
        ending.stdout
    );
    assert_eq!(ending.stderr, "");
}

#[test]
fn help_as_json_is_its_declaration() {
    let ending = ran("--help --json", |_| Ok(()));
    let value: Value = serde_json::from_str(&ending.stdout).expect("help is JSON");
    assert_eq!(value["schema"], "help/1");
}

#[test]
fn version_is_one_line() {
    assert_eq!(ran("--version", |_| Ok(())).stdout, "thing 1.0\n");
}

#[test]
fn a_failure_exits_with_its_code_and_says_itself() {
    let ending = ran("go", |_| Err(Failure::new(Exit::NotFound, "no such row")));
    assert_eq!(ending.exit, Exit::NotFound);
    assert_eq!(ending.stderr, "\u{2715} no such row\n");
    let ending = ran("go --json", |_| {
        Err(Failure::new(Exit::NotFound, "no such row"))
    });
    let value: Value = serde_json::from_str(&ending.stderr).expect("the error is JSON");
    assert_eq!(value["kind"], "not-found");
}

#[test]
fn a_stand_ins_failure_ends_with_its_code_saying_only_its_line() {
    let ending = ran("go", |_| Err(Failure::passed(3, "the job failed")));
    assert_eq!(
        (ending.exit, ending.code()),
        (Exit::Failed, 3),
        "the stand-in's 3 didn't pass through"
    );
    assert_eq!(
        (ending.stdout.as_str(), ending.stderr.as_str()),
        ("", "\u{2715} the job failed\n"),
        "it said more than its line"
    );
    let streamed = ran("go --format jsonl", |_| {
        Err(Failure::passed(3, "the job failed"))
    });
    let end: Value = serde_json::from_str(&streamed.stdout).expect("the end line is JSON");
    assert_eq!(
        end["exit"], 3,
        "the end line didn't give the stand-in's code"
    );
}

#[test]
fn a_failure_in_the_journal_says_its_an_error_on_each_line() {
    let mut journal = Environment::of([false; 3], &[]);
    journal.journal = true;
    let ending = run(&help(), "1.0", &journal, vec!["go".to_owned()], |_| {
        Err(Failure::new(Exit::NotFound, "no such row").try_this("thing go"))
    });
    assert_eq!(
        ending.stderr,
        "<3>\u{2715} no such row\n<3>  try  thing go\n"
    );
}

#[test]
fn a_failure_said_already_ends_with_its_code_and_says_nothing_more() {
    let mut journal = Environment::of([false; 3], &[]);
    journal.journal = true;
    for environment in [Environment::of([false; 3], &[]), journal] {
        let ending = run(&help(), "1.0", &environment, vec!["go".to_owned()], |_| {
            Err(Failure::new(Exit::TryAgain, "it handed over").said_already())
        });
        assert_eq!(
            (ending.code(), ending.stderr.as_str()),
            (75, ""),
            "a failure the command said as it went was said again"
        );
    }
    let ending = run(
        &help(),
        "1.0",
        &Environment::of([false; 3], &[]),
        vec!["--json".to_owned(), "go".to_owned()],
        |_| Err(Failure::new(Exit::TryAgain, "it handed over").said_already()),
    );
    assert_eq!(ending.code(), 75, "its code is kept under --json");
    assert!(
        ending.stderr.contains("\"it handed over\""),
        "a reader of JSON gets the failure's object: {}",
        ending.stderr
    );
}

#[test]
fn a_line_it_cant_read_is_asked_wrongly() {
    assert_eq!(ran("--format yaml", |_| Ok(())).exit, Exit::Usage);
}

#[test]
fn an_argument_that_isnt_text_is_asked_wrongly_before_the_body() {
    use std::os::unix::ffi::OsStringExt;
    let ending = run_os(
        &help(),
        "1.0",
        &Environment::of([false; 3], &[]),
        vec![OsString::from("go"), OsString::from_vec(vec![b'a', 0xff])],
        |_| panic!("the body ran"),
    );
    assert_eq!(ending.exit, Exit::Usage);
    assert!(ending.stderr.contains("a\u{fffd}"), "{}", ending.stderr);
}

#[test]
fn a_stream_ends_with_its_outcome() {
    let ending = ran("go --format jsonl", |_| {
        Err(Failure::new(Exit::TryAgain, "busy"))
    });
    assert_eq!(
        ending.stdout,
        "{\"exit\":75,\"outcome\":\"failed\",\"type\":\"end\"}\n"
    );
}

fn asked(line: &str, body: impl FnOnce(&Context) -> Result<bool, Failure>) -> Ending {
    let args = line.split_whitespace().map(str::to_owned).collect();
    run_question(
        &help(),
        "1.0",
        &Environment::of([false; 3], &[]),
        args,
        body,
    )
}

#[test]
fn a_question_answered_yes_ends_done_saying_nothing() {
    let ending = asked("go", |_| Ok(true));
    assert_eq!(ending.exit, Exit::Done);
    assert_eq!(
        (ending.stdout, ending.stderr),
        (String::new(), String::new())
    );
}

#[test]
fn a_question_answered_no_ends_1_saying_nothing_but_its_streams_end() {
    let ending = asked("go", |_| Ok(false));
    assert_eq!(ending.exit, Exit::Failed);
    assert_eq!(
        (ending.stdout, ending.stderr),
        (String::new(), String::new())
    );
    let ending = asked("go --format jsonl", |_| Ok(false));
    assert_eq!(
        ending.stdout,
        "{\"exit\":1,\"outcome\":\"no\",\"type\":\"end\"}\n"
    );
    assert_eq!(ending.stderr, "", "a no said something on stderr");
}

#[test]
fn a_question_that_cant_be_answered_fails_as_any_command_does() {
    let ending = asked("go", |_| {
        Err(Failure::new(Exit::Unavailable, "nothing answered"))
    });
    assert_eq!(ending.exit, Exit::Unavailable);
    assert_eq!(ending.stderr, "\u{2715} nothing answered\n");
}

#[test]
fn panics_on_two_threads_each_keep_their_words() {
    let ran: Vec<_> = (0..8)
        .map(|row| {
            std::thread::spawn(move || {
                let ending = ran("go", |_| panic!("row {row} was gone"));
                (row, ending)
            })
        })
        .collect();
    for done in ran {
        let (row, ending) = done
            .join()
            .expect("a body's panic was caught on its thread");
        assert_eq!(ending.exit, Exit::Bug);
        assert!(
            ending.stderr.contains(&format!("row {row} was gone")),
            "row {row} lost its words: {}",
            ending.stderr
        );
    }
}

#[test]
fn a_panic_is_a_bug() {
    let ending = ran("go", |_| panic!("the row was gone"));
    assert_eq!(ending.exit, Exit::Bug);
    assert!(
        ending.stderr.contains("the row was gone"),
        "{}",
        ending.stderr
    );
}

fn tree() -> Help {
    Help::new("thing", "does a thing")
        .subcommand(
            Help::new("rows", "every row, or one")
                .alias("row")
                .argument("<name>", "a row's name")
                .example("thing rows disk", "the disk's row"),
        )
        .exit(Exit::NotFound, "no such row")
}

fn ran_tree(line: &str, body: impl FnOnce(&Context) -> Result<(), Failure>) -> Ending {
    let args = line.split_whitespace().map(str::to_owned).collect();
    run(
        &tree(),
        "1.0",
        &Environment::of([false; 3], &[]),
        args,
        body,
    )
}

#[test]
fn each_way_of_asking_for_a_subcommands_help_answers_it_alone() {
    for line in ["rows --help", "rows -h", "help rows", "row disk --help"] {
        let ending = ran_tree(line, |_| panic!("{line} started the command"));
        assert_eq!(ending.exit, Exit::Done, "{line}: {}", ending.stderr);
        assert!(
            ending.stdout.starts_with("thing rows  every row, or one\n"),
            "{line} didn't answer rows' help: {}",
            ending.stdout
        );
        assert!(
            ending.stdout.contains("66 no such row"),
            "{line} lost its parent's exits: {}",
            ending.stdout
        );
    }
    let ending = ran_tree("help rows --json", |_| Ok(()));
    let value: Value = serde_json::from_str(&ending.stdout).expect("help is JSON");
    assert_eq!(
        value["name"], "thing rows",
        "the JSON named the wrong command"
    );
}

#[test]
fn a_word_no_subcommand_answers_to_is_refused_before_the_body() {
    for line in ["rwos", "rwos --help"] {
        let ending = ran_tree(line, |_| panic!("{line} started the command"));
        assert_eq!(ending.exit, Exit::Usage, "{line} wasn't refused");
        assert!(
            ending.stderr.contains("did you mean rows?"),
            "{line} didn't name the nearest: {}",
            ending.stderr
        );
    }
}

#[test]
fn the_body_reads_an_alias_as_the_name_it_stands_for() {
    let ending = ran_tree("row disk", |context| {
        assert_eq!(context.args, ["rows", "disk"], "the alias reached the body");
        Ok(())
    });
    assert_eq!(ending.exit, Exit::Done, "{}", ending.stderr);
}

#[test]
fn the_body_reads_what_the_shared_flags_left() {
    let ending = ran("go --color never far", |context| {
        assert_eq!(context.args, ["go", "far"]);
        assert!(!context.out.inks(), "--color never inks nothing");
        Ok(())
    });
    assert_eq!(ending.exit, Exit::Done);
}
