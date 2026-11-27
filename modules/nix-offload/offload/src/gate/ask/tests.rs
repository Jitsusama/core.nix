use super::*;

fn refusal(why: &str) -> Result<(), Said> {
    Err(Said::Refused(why.to_owned()))
}

#[test]
fn a_start_the_node_refused_is_a_refusal_though_mycelium_ended_0() {
    let said = "{\"Err\":\"this asks as a fenced agent, so it starts nothing\"}\n";
    assert_eq!(
        judged(true, said, ""),
        refusal("this asks as a fenced agent, so it starts nothing"),
        "a reply of Err must read as the node's refusal, whatever the exit"
    );
}

#[test]
fn a_start_the_node_took_is_no_failure() {
    let said = "{\"Ok\":{\"Started\":{\"machine\":\"far\"}}}\nshared\n";
    assert_eq!(
        judged(true, said, ""),
        Ok(()),
        "a reply of Ok refuses nothing"
    );
}

#[test]
fn the_first_reply_decides() {
    let said = "{\"Ok\":null}\n{\"Err\":\"later\"}\n";
    assert_eq!(
        judged(true, said, ""),
        Ok(()),
        "an Err after an Ok mustn't undo it"
    );
    let said = "{\"Err\":\"first\"}\n{\"Ok\":null}\n";
    assert_eq!(
        judged(true, said, ""),
        refusal("first"),
        "an Ok after an Err mustn't undo it"
    );
}

#[test]
fn a_reply_printed_across_lines_is_read_whole() {
    let said = "{\n  \"Err\": \"no room\"\n}\n";
    assert_eq!(
        judged(true, said, ""),
        refusal("no room"),
        "a reply spread over lines refuses"
    );
}

#[test]
fn a_refusal_that_isnt_words_or_says_nothing_still_says_something() {
    let said = "{\"Err\":{\"Fenced\":3}}\n";
    assert_eq!(
        judged(true, said, ""),
        refusal("{\"Fenced\":3}"),
        "a refusal that isn't a string is told as its JSON"
    );
    assert_eq!(
        judged(true, "{\"Err\":\" \"}\n", ""),
        refusal("the node refused, saying nothing why"),
        "an empty refusal still says it was one"
    );
}

#[test]
fn mycelium_ending_badly_without_a_reply_failed_and_says_why() {
    assert_eq!(
        judged(false, "shared\n", " can't reach the node\n"),
        Err(Said::Failed("can't reach the node".to_owned())),
        "a failed run with no reply is a failure, told by its errors"
    );
    assert_eq!(
        judged(false, "", ""),
        Err(Said::Failed(
            "mycelium failed, saying nothing why".to_owned()
        )),
        "a failed run that says nothing still says it failed"
    );
    assert_eq!(
        judged(true, "shared\n", ""),
        Ok(()),
        "plain words at exit 0 are no failure"
    );
}

#[test]
fn a_refusal_ends_as_refused_and_a_failure_as_unavailable() {
    let refused = failed_by(Said::Refused("no".to_owned()), "it", "Try.");
    let failure = failed_by(Said::Failed("down".to_owned()), "it", "Try.");
    assert_eq!(
        refused.exit,
        Exit::Refused,
        "a node's no must end as a refusal"
    );
    assert_eq!(
        failure.exit,
        Exit::Unavailable,
        "a run that failed must end as unavailable"
    );
}

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A pace of a few milliseconds a look, `crossing` after an end and
/// `longest` in all, so a test waits as a real ask does, only shorter.
const fn pace(crossing: u64, longest: u64) -> Pace {
    Pace {
        longest: Duration::from_millis(longest),
        crossing: Duration::from_millis(crossing),
        looking: Duration::from_millis(5),
    }
}

/// An answer that ends as `ends` says once asked `after` times, writing
/// `writes` into its gate folder as it ends, and counts whether it was
/// stopped.
struct Fake {
    ends: Option<End>,
    after: usize,
    writes: Option<(PathBuf, String)>,
    stopped: bool,
}

impl Fake {
    fn ending(ends: Option<End>, after: usize) -> Self {
        Self {
            ends,
            after,
            writes: None,
            stopped: false,
        }
    }

    fn writing(mut self, gate: &Path, found: &Found) -> Self {
        self.writes = Some((gate.join(RESULT), found.json()));
        self
    }
}

impl Answering for Fake {
    fn ended(&mut self) -> Option<End> {
        if self.after > 0 {
            self.after -= 1;
            return None;
        }
        if let Some((path, text)) = self.writes.take() {
            fs::write(path, text).expect("the answer's result");
        }
        self.ends.clone()
    }

    fn stop(&mut self) {
        self.stopped = true;
    }
}

fn found(exit: u8) -> Found {
    Found {
        commit: COMMIT.to_owned(),
        exit,
        said: "the commit's changed checks passed".to_owned(),
    }
}

#[test]
fn an_answer_that_ends_without_a_result_is_told_once_its_crossing_is_past() {
    let gate = tempfile::tempdir().expect("a gate folder");
    // It runs a while first, so the looking goes round before it ends.
    let mut answer = Fake::ending(Some(End::Exited(UNSTARTED)), 3);
    let since = Instant::now();
    let waited = waited(gate.path(), COMMIT, pace(100, 60_000), &mut answer);
    assert_eq!(
        waited,
        Waited::Ended(End::Exited(UNSTARTED)),
        "an answer its fence refused should be told by how it ended"
    );
    let took = since.elapsed();
    assert!(
        took >= Duration::from_millis(100) && took < Duration::from_secs(5),
        "an ended answer should be told once its crossing is past, not waited out: {took:?}"
    );
}

/// A result written `after` from now into `gate`, as one crossing back.
fn crossing_back(gate: &Path, found: &Found, after: Duration) -> thread::JoinHandle<()> {
    let (path, text) = (gate.join(RESULT), found.json());
    thread::spawn(move || {
        thread::sleep(after);
        fs::write(path, text).expect("a result");
    })
}

#[test]
fn a_failed_gates_result_crossing_back_after_its_end_is_the_answer() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let late = crossing_back(gate.path(), &found(1), Duration::from_millis(200));
    let mut answer = Fake::ending(Some(End::Exited(1)), 0);
    let waited = waited(gate.path(), COMMIT, pace(60_000, 60_000), &mut answer);
    late.join().expect("the result's writer");
    assert_eq!(
        waited,
        Waited::Found(found(1)),
        "a gate that failed ends its answer with its code, and its result should be waited for"
    );
}

#[test]
fn an_answer_ending_255_is_waited_for_like_any_other_end() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let late = crossing_back(gate.path(), &found(0), Duration::from_millis(200));
    let mut answer = Fake::ending(Some(End::Exited(UNRUN)), 0);
    let waited = waited(gate.path(), COMMIT, pace(60_000, 60_000), &mut answer);
    late.join().expect("the result's writer");
    assert_eq!(
        waited,
        Waited::Found(found(0)),
        "a 255 may be a dropped link after the gate ran, so its result should be waited for"
    );
}

#[test]
fn a_pass_that_never_crosses_back_is_told_once_its_crossing_is_past() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let mut answer = Fake::ending(Some(End::Exited(0)), 0);
    assert_eq!(
        waited(gate.path(), COMMIT, pace(20, 60_000), &mut answer),
        Waited::Ended(End::Exited(0)),
        "an answer that ended 0 with nothing back shouldn't be waited out to the longest"
    );
}

#[test]
fn a_result_written_as_the_answer_ended_is_the_answer() {
    let gate = tempfile::tempdir().expect("a gate folder");
    // The result lands only as the end is heard, so the first look missed
    // it and only the look once more on telling the end finds it.
    let mut answer = Fake::ending(Some(End::Exited(0)), 0).writing(gate.path(), &found(0));
    assert_eq!(
        waited(gate.path(), COMMIT, pace(0, 60_000), &mut answer),
        Waited::Found(found(0)),
        "a result written as the answer ended should be read, not told as missing"
    );
    assert!(!answer.stopped, "an answer that ended needn't be stopped");
}

#[test]
fn an_answer_still_running_once_a_result_is_back_is_left_to_its_run() {
    let gate = tempfile::tempdir().expect("a gate folder");
    fs::write(gate.path().join(RESULT), found(0).json()).expect("a result");
    let mut answer = Fake::ending(None, 0);
    assert_eq!(
        waited(gate.path(), COMMIT, pace(0, 60_000), &mut answer),
        Waited::Found(found(0)),
        "a result back while the answer runs should be the answer"
    );
    assert!(
        !answer.stopped,
        "a result there may be an earlier run's, so the running answer shouldn't be stopped"
    );
}

#[test]
fn a_result_for_another_commit_isnt_the_answer() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let mut other = found(0);
    other.commit = "f".repeat(40);
    fs::write(gate.path().join(RESULT), other.json()).expect("a stale result");
    let mut answer = Fake::ending(Some(End::Exited(66)), 0);
    assert_eq!(
        waited(gate.path(), COMMIT, pace(0, 60_000), &mut answer),
        Waited::Ended(End::Exited(66)),
        "another commit's result shouldn't answer this one"
    );
}

#[test]
fn an_answer_still_running_at_the_longest_is_stopped() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let mut answer = Fake::ending(None, 0);
    assert_eq!(
        waited(gate.path(), COMMIT, pace(0, 0), &mut answer),
        Waited::Late,
        "an answer past the longest wait should be late"
    );
    assert!(
        answer.stopped,
        "an answer ask stops waiting for should be stopped"
    );
}

#[test]
fn an_answer_ended_past_the_longest_is_late_and_stops_nothing() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let mut answer = Fake::ending(Some(End::Exited(0)), 0);
    assert_eq!(
        waited(gate.path(), COMMIT, pace(60_000, 50), &mut answer),
        Waited::Late,
        "an answer whose crossing outlasts the longest wait should be late"
    );
    assert!(!answer.stopped, "an answer that ended needn't be stopped");
}

/// `gated` over answers that end as `ends` say, one a start, each
/// writing `writes` as it ends where it says, and how many it started.
fn gated_over(
    gate: &Path,
    ends: Vec<(End, Option<Found>)>,
    pace: Pace,
) -> (Result<Found, Failure>, usize) {
    let mut ends = ends.into_iter();
    let mut starts = 0;
    let gated = gated("far", gate, COMMIT, pace, &mut |_| {
        starts += 1;
        let (end, found) = ends.next().expect("no more starts than ends");
        let fake = Fake::ending(Some(end), 0);
        Ok(Box::new(match found {
            Some(found) => fake.writing(gate, &found),
            None => fake,
        }) as Box<dyn Answering>)
    });
    (gated, starts)
}

#[test]
fn an_answer_that_couldnt_start_is_started_again() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let unstarted = (End::Exited(UNSTARTED), None);
    let (gated, starts) = gated_over(
        gate.path(),
        vec![
            unstarted.clone(),
            unstarted,
            (End::Exited(0), Some(found(0))),
        ],
        pace(0, 60_000),
    );
    assert_eq!(
        gated.map(|found| found.exit).ok(),
        Some(0),
        "the third start's pass should be the answer"
    );
    assert_eq!(
        starts, 3,
        "an answer its fence refused should be started again"
    );
}

#[test]
fn an_answer_that_never_starts_is_told_after_its_last_start() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let unstarted = (End::Exited(UNSTARTED), None);
    let (gated, starts) = gated_over(
        gate.path(),
        vec![unstarted.clone(), unstarted.clone(), unstarted],
        pace(0, 60_000),
    );
    let failure = gated.err().expect("no start should mean no result");
    assert_eq!(
        failure.exit,
        Exit::Unavailable,
        "it should end as unavailable"
    );
    assert_eq!(starts, STARTS as usize, "it should stop at its last start");
}

#[test]
fn each_end_without_a_result_is_told_with_its_own_exit() {
    let gate = tempfile::tempdir().expect("a gate folder");
    let (unrun, _) = gated_over(
        gate.path(),
        vec![(End::Exited(UNRUN), None)],
        pace(0, 60_000),
    );
    let unrun = unrun.err().expect("a 255 without a result is no answer");
    assert_eq!(
        unrun.exit,
        Exit::Unavailable,
        "a 255 should end as unavailable"
    );
    let (late, _) = gated_over(gate.path(), vec![(End::Exited(0), None)], pace(60_000, 0));
    assert_eq!(
        late.err().map(|failure| failure.exit),
        Some(NOT_YET),
        "no result within the longest wait should end as not yet"
    );
}

#[test]
fn an_end_is_said_in_words() {
    assert_eq!(End::Exited(126).said(), "exited 126");
    assert_eq!(End::Signalled.said(), "was ended by a signal");
    assert_eq!(
        End::Unwatched("gone".to_owned()).said(),
        "couldn't be watched: gone"
    );
}

#[test]
fn the_answer_runs_in_its_gates_folder_for_the_longest_wait() {
    let run = answer_run(
        "QEST-20261115-ABC123",
        "optimus",
        COMMIT,
        Path::new("/home/j/q/handover/gates/c"),
        Duration::from_secs(7201),
    );
    assert_eq!(
        run.join(" "),
        format!(
            "run --need machine=optimus --folder /home/j/q/handover/gates/c --lasts 121m \
             -- nix-offload answer QEST-20261115-ABC123 {COMMIT}"
        ),
        "the answer should run, waited on, in the gate's folder"
    );
}

fn words(text: &str) -> Vec<String> {
    text.split(' ').map(str::to_owned).collect()
}

#[test]
fn an_ask_ends_with_the_gates_own_codes() {
    assert_eq!(
        REFUSES.code(),
        REFUSED,
        "a refusal should keep the gate's code"
    );
    assert_eq!(
        NOT_YET.code(),
        NOT_NOW,
        "a not-now should keep the gate's code"
    );
}

#[test]
fn an_ask_names_its_quest_and_may_name_the_rest() {
    assert_eq!(
        asked(&words(
            "--quest QEST-20261115-ABC123 --session SK1FF0 --on kestrel --longest 60"
        )),
        Ok(Asked {
            quest: "QEST-20261115-ABC123".to_owned(),
            session: Some("SK1FF0".to_owned()),
            on: Some("kestrel".to_owned()),
            longest: Duration::from_secs(60),
        }),
        "every option should be read"
    );
    assert!(
        asked(&words("--session SK1FF0")).is_err(),
        "an ask without a quest should be refused"
    );
    assert!(
        asked(&words("--quest ../x")).is_err(),
        "a quest that isn't an id should be refused"
    );
    assert!(
        asked(&words("--quest QEST-20261115-ABC123 --session $(x)")).is_err(),
        "a session that isn't an address should be refused"
    );
}
