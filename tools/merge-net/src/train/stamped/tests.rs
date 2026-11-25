use super::*;

/// A log of this test's own, by `name`, gone from any earlier run.
fn log_for(name: &str) -> std::path::PathBuf {
    let log = std::env::temp_dir().join(format!(
        "merge-net-stamped-{name}-{}.log",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&log);
    log
}

/// `script` run by `sh` as a gate, waiting `wait` for its last words: how
/// it exited, and its log's bytes.
fn gated_by(name: &str, script: &str, wait: Duration) -> (ExitStatus, Vec<u8>) {
    let log = log_for(name);
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    let status = run_within(command, &log, wait).expect("the gate should run");
    let bytes = std::fs::read(&log).expect("the gate should leave a log");
    let _ = std::fs::remove_file(&log);
    (status, bytes)
}

/// A logged line's stamp in seconds and what follows it, or `None` when it
/// isn't opened by one.
fn unstamped(line: &str) -> Option<(u64, &str)> {
    let (stamp, said) = line.strip_prefix('[')?.split_once("] ")?;
    let (minutes, seconds) = stamp.split_once(':')?;
    let seconds: u64 = seconds.parse().ok()?;
    (seconds < 60 && stamp.len() >= 4).then_some(())?;
    Some((minutes.parse::<u64>().ok()? * 60 + seconds, said))
}

/// Every line of `bytes`, each read past its stamp, failing on one with
/// none.
fn said_lines(bytes: &[u8]) -> Vec<(u64, String)> {
    let words = String::from_utf8_lossy(bytes);
    words
        .lines()
        .map(|line| {
            let (seconds, said) =
                unstamped(line).unwrap_or_else(|| panic!("a line wasn't stamped: {line:?}"));
            (seconds, said.to_owned())
        })
        .collect()
}

#[test]
fn a_stamp_says_minutes_and_seconds() {
    assert_eq!(stamp(0), "[0:00] ", "the first second's stamp");
    assert_eq!(stamp(1_573), "[26:13] ", "a stamp past the 26 minutes seen");
    assert_eq!(stamp(6_000), "[100:00] ", "a stamp past 99 minutes");
    assert_eq!(
        unstamped("[26:13] said"),
        Some((1_573, "said")),
        "the tests should read a stamp back as it's written"
    );
}

// Both streams reach the log, every line stamped once, and the exit is the
// command's own, since the train reads a red from it.
#[test]
fn every_line_of_both_streams_is_stamped_and_the_exit_kept() {
    let (status, bytes) = gated_by(
        "streams",
        "echo said; echo erred >&2; printf 'no newline'; exit 3",
        LAST_WORDS,
    );
    assert_eq!(status.code(), Some(3), "the gate's own exit was lost");
    let mut said: Vec<String> = said_lines(&bytes).into_iter().map(|(_, s)| s).collect();
    said.sort_unstable();
    assert_eq!(
        said,
        ["erred", "no newline", "said"],
        "the log wasn't each line once: {}",
        String::from_utf8_lossy(&bytes)
    );
}

// A stamp is the time since the gate began rather than a count started
// afresh for each line, so a gate that waits shows the wait.
#[test]
fn stamps_count_from_the_gates_start() {
    let (_, bytes) = gated_by("forward", "sleep 1; echo a; sleep 1; echo b", LAST_WORDS);
    let lines = said_lines(&bytes);
    let [(at_a, a), (at_b, b)] = lines.as_slice() else {
        panic!("the log wasn't two lines: {lines:?}");
    };
    assert_eq!(
        (a.as_str(), b.as_str()),
        ("a", "b"),
        "a stream's order was lost"
    );
    assert!(*at_a >= 1, "a line said after a second was stamped {at_a}");
    assert!(at_b > at_a, "the stamps didn't move on: {at_a} then {at_b}");
}

// A process the gate leaves behind holding its output doesn't hold the
// train: the wait ends, the log says so, and nothing said later is kept.
#[test]
fn a_gate_whose_output_outlives_it_is_left_after_the_wait() {
    let log = log_for("outlives");
    let mut command = Command::new("sh");
    command.args(["-c", "echo said; (sleep 3; echo later) & exit 4"]);
    let began = Instant::now();
    let status =
        run_within(command, &log, Duration::from_millis(200)).expect("the gate should run");
    // The leftover speaks at 3 s; a run held past 2 s waited on it rather
    // than its 200 ms, with room left for a busy builder's slow start.
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "the gate was waited for past its wait: {:?}",
        began.elapsed()
    );
    assert_eq!(status.code(), Some(4), "the gate's own exit was lost");
    // Past the leftover's word, which its copier is still alive to hear,
    // so a log that kept it would show it now.
    thread::sleep(Duration::from_millis(3_500));
    let said: Vec<String> = said_lines(&std::fs::read(&log).expect("a log"))
        .into_iter()
        .map(|(_, s)| s)
        .collect();
    let _ = std::fs::remove_file(&log);
    assert_eq!(
        said.len(),
        2,
        "the log wasn't the word and the cut: {said:?}"
    );
    assert_eq!(said[0], "said", "the gate's word before the wait was lost");
    assert!(
        said[1].starts_with("merge-net: stopped waiting for the gate's last words 200 ms"),
        "the log didn't say where the verdict stopped reading: {said:?}"
    );
}

// A line written in pieces while the other stream speaks stays whole.
#[test]
fn a_line_said_in_pieces_stays_whole_beside_the_other_stream() {
    let (_, bytes) = gated_by(
        "pieces",
        "printf hal; sleep 0.3; echo erred >&2; sleep 0.3; echo f",
        LAST_WORDS,
    );
    let mut said: Vec<String> = said_lines(&bytes).into_iter().map(|(_, s)| s).collect();
    said.sort_unstable();
    assert_eq!(
        said,
        ["erred", "half"],
        "a line was split by the other stream"
    );
}

// Both streams talking at once keep every line whole and each stream's
// lines in order.
#[test]
fn both_streams_at_once_keep_each_line_whole_and_in_order() {
    let (_, bytes) = gated_by(
        "together",
        "seq 1 3000 | sed 's/^/o/' & seq 1 3000 | sed 's/^/e/' >&2; wait",
        LAST_WORDS,
    );
    let lines = said_lines(&bytes);
    for stream in ['o', 'e'] {
        let numbers: Vec<u32> = lines
            .iter()
            .filter_map(|(_, said)| said.strip_prefix(stream))
            .map(|number| number.parse().expect("a whole number"))
            .collect();
        assert_eq!(
            numbers,
            (1..=3000).collect::<Vec<u32>>(),
            "the {stream} stream's lines weren't whole and in order"
        );
    }
    assert_eq!(lines.len(), 6000, "a line was mixed into another");
}

// A byte that isn't UTF-8 passes through, and a line past the longest
// piece is written in pieces rather than held whole.
#[test]
fn an_odd_byte_and_a_long_line_pass_through() {
    let (_, bytes) = gated_by(
        "long",
        "printf '\\377\\n'; head -c 70000 /dev/zero | tr '\\0' x; echo; echo after",
        LAST_WORDS,
    );
    let lines: Vec<&[u8]> = bytes.split(|byte| *byte == b'\n').collect();
    let past = |line: &[u8]| -> Vec<u8> {
        let at = line.iter().position(|byte| *byte == b' ').expect("a stamp");
        line[at + 1..].to_vec()
    };
    assert_eq!(past(lines[0]), b"\xff", "a byte that isn't UTF-8 was lost");
    let piece = usize::try_from(LONGEST_PIECE).expect("a piece fits in memory");
    assert_eq!(
        past(lines[1]).len(),
        piece,
        "a long line wasn't cut at a piece"
    );
    assert_eq!(
        past(lines[2]).len(),
        70_000 - piece,
        "a long line's rest was lost"
    );
    assert_eq!(
        past(lines[3]),
        b"after",
        "the words after a long line were lost"
    );
}

// A line exactly a piece long ends with its newline, not with an empty
// stamped line after it.
#[test]
fn a_line_a_piece_long_is_one_line() {
    let (_, bytes) = gated_by(
        "piece",
        &format!("head -c {LONGEST_PIECE} /dev/zero | tr '\\0' x; echo; echo after"),
        LAST_WORDS,
    );
    let lines = said_lines(&bytes);
    let lengths: Vec<usize> = lines.iter().map(|(_, said)| said.len()).collect();
    let piece = usize::try_from(LONGEST_PIECE).expect("a piece fits in memory");
    assert_eq!(lengths, [piece, 5], "a piece-long line wasn't one line");
}

/// A stream that says `said` and then can't be read.
struct Breaks(&'static [u8]);

impl Read for Breaks {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.0.is_empty() {
            return Err(io::Error::other("the stream broke"));
        }
        let length = self.0.len().min(into.len());
        into[..length].copy_from_slice(&self.0[..length]);
        self.0 = &self.0[length..];
        Ok(length)
    }
}

// A stream that breaks keeps what was read of it, and its error, so the
// gate isn't taken as heard whole.
#[test]
fn a_stream_that_breaks_keeps_its_words_and_its_error() {
    let log = log_for("breaks");
    let shared = Log {
        file: Mutex::new(File::create(&log).expect("a log")),
        closed: AtomicBool::new(false),
        lost: Mutex::new(None),
    };
    copy(Breaks(b"said\nhal"), &shared, Instant::now());
    let said: Vec<String> = said_lines(&std::fs::read(&log).expect("a log"))
        .into_iter()
        .map(|(_, s)| s)
        .collect();
    let _ = std::fs::remove_file(&log);
    assert_eq!(
        said,
        ["said", "hal"],
        "the words before the break were lost"
    );
    let lost = shared.lost.lock().expect("the lock").take();
    assert!(
        lost.is_some_and(|error| error.to_string() == "the stream broke"),
        "the break wasn't kept"
    );
}

// A log that can't be written fails the gate, as a full disk failed the
// gate's own writes before the stamps. The gate says more than a pipe
// holds, so a copier that stopped reading at the failed write would hold
// the gate, and the test, for good.
#[test]
fn a_log_that_cant_be_written_fails_the_gate() {
    let mut command = Command::new("sh");
    command.args(["-c", "seq 1 200000"]);
    let gated = run(command, Path::new("/dev/full"));
    assert!(
        gated
            .as_ref()
            .is_err_and(|error| error.to_string().starts_with("the gate's log lost words")),
        "a gate whose words were lost read as heard: {gated:?}"
    );
}
