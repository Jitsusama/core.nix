use super::*;
use std::cell::RefCell;
use std::collections::BTreeSet;

#[test]
fn a_cause_s_text_is_cut_and_a_stretch_says_only_so_many() {
    let mut watch = Watch::default();
    // Two-byte characters, a bell and a C1 control, so the cut counts
    // characters and the journal gets none of the daemon's controls.
    let long = format!("\u{7}ab\u{9b}{}", "\u{e9}".repeat(5000));
    let said = watch.heard(Err(long)).0;
    assert_eq!(said.len(), 1, "the first cause wasn't said once");
    assert!(
        said[0].ends_with(&format!(": ab{}...", "\u{e9}".repeat(BRIEF - 2))),
        "a long cause wasn't cut to its first {BRIEF} plain characters: {}",
        said[0]
    );
    let mut said = Vec::new();
    for n in 0..100 {
        said.extend(watch.heard(Err(format!("refused at column {n}"))).0);
    }
    assert_eq!(
        said.len(),
        CAUSES,
        "causes that vary without end weren't stopped at {CAUSES}: {said:?}"
    );
    assert!(
        said[CAUSES - 1].ends_with("its further causes go unsaid until it answers"),
        "the journal wasn't told that further causes go unsaid: {said:?}"
    );
    assert_eq!(
        watch.heard(Ok(Vec::new())).0,
        vec!["<5>the calendar answers again"],
        "the calendar answering again went unsaid"
    );
    // A new stretch starts with nothing said, so its first cause is said
    // even when an earlier stretch said it.
    assert_eq!(
        watch.heard(Err("refused at column 1".into())).0,
        vec![
            "<4>the calendar can't be read, so the 0 meetings it last said stand: \
              refused at column 1"
        ],
        "a new stretch's first cause went unsaid"
    );
    assert_eq!(
        watch.heard(Err("refused at column 2".into())).0,
        vec!["<4>the calendar still can't be read, now: refused at column 2"],
        "a cause said in an earlier stretch went unsaid in a new one"
    );
}

#[test]
fn each_failure_to_keep_is_said_once_and_keeping_again_is_said() {
    // The start's read failed with the same text the first keep fails with,
    // as an unwritable state folder gives both; the keep is still said.
    let mut keeping = Keeping::unread();
    assert!(
        keeping.owed(),
        "a kept file that couldn't be read wasn't owed a keep"
    );
    let why = "meetings.json: Permission denied";
    assert_eq!(
        keeping.kept(Err(why.into())),
        Some(format!(
            "<4>the meetings couldn't be kept for the next run: {why}"
        )),
        "a keep failing as the start's read did went unsaid"
    );
    assert_eq!(
        keeping.kept(Err(why.into())),
        None,
        "the same failure to keep was said at every answer"
    );
    assert!(keeping.owed(), "a failed keep wasn't owed another");
    assert_eq!(
        keeping.kept(Ok(())),
        Some("<5>the meetings are kept for the next run again".to_owned()),
        "keeping again went unsaid, leaving the journal naming the failure"
    );
    assert!(!keeping.owed(), "a keep that worked was still owed");
    assert_eq!(keeping.kept(Ok(())), None, "each good keep was said");
    let mut fresh = Keeping::default();
    assert_eq!(
        fresh.kept(Ok(())),
        None,
        "a first good keep was said as a recovery"
    );
    // A kept file the start couldn't read, written afresh at once.
    let mut rewritten = Keeping::unread();
    assert_eq!(
        rewritten.kept(Ok(())),
        Some("<5>the meetings are kept for the next run again".to_owned()),
        "writing afresh a kept file the start couldn't read went unsaid"
    );
}

const MIN: u64 = 60_000;

/// Cgroups held in memory: which carry the program, which aren't there,
/// and which the kernel refuses.
#[derive(Default)]
struct Fake {
    on: RefCell<BTreeSet<PathBuf>>,
    missing: BTreeSet<PathBuf>,
    refused: BTreeSet<PathBuf>,
}

impl Cgroups for Fake {
    fn set(&self, path: &Path, on: bool) -> io::Result<bool> {
        if self.missing.contains(path) {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        if self.refused.contains(path) {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        let mut held = self.on.borrow_mut();
        Ok(if on {
            held.insert(path.to_owned())
        } else {
            held.remove(path)
        })
    }
}

fn agents() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/cg/jobs.slice"),
        PathBuf::from("/cg/agents.slice"),
    ]
}

fn review(start: u64) -> Meeting {
    Meeting {
        title: "Design review".into(),
        start,
        end: start + 30 * MIN,
    }
}

#[test]
fn a_meeting_ten_minutes_off_puts_the_program_on_every_agents_slice() {
    let start = 1_000 * MIN;
    let cgroups = Fake::default();
    let mut watch = Watch::from(vec![review(start)]);
    let said = watch.look(&cgroups, &agents(), start - 10 * MIN);
    assert_eq!(
        *cgroups.on.borrow(),
        agents().into_iter().collect(),
        "an agents' slice was left open 10 minutes before a meeting"
    );
    assert_eq!(
        said,
        vec![
            "<5>agents may open no camera until Design review ends",
            "<6>guarded /cg/jobs.slice",
            "<6>guarded /cg/agents.slice",
        ],
        "the journal didn't say the guard went up"
    );
    let ended = watch.look(&cgroups, &agents(), start + 30 * MIN);
    assert!(
        cgroups.on.borrow().is_empty(),
        "the program stayed on after the meeting ended"
    );
    assert_eq!(
        ended[0], "<5>agents may open a camera",
        "the journal didn't say the guard came down"
    );
}

#[test]
fn a_look_that_changes_nothing_says_nothing() {
    let start = 1_000 * MIN;
    let cgroups = Fake::default();
    let mut watch = Watch::from(vec![review(start)]);
    watch.look(&cgroups, &agents(), start - 10 * MIN);
    assert!(
        watch.look(&cgroups, &agents(), start - 9 * MIN).is_empty(),
        "a look with the guard already up wrote to the journal"
    );
}

#[test]
fn a_slice_made_again_without_the_program_is_guarded_at_the_next_look() {
    let start = 1_000 * MIN;
    let cgroups = Fake::default();
    let mut watch = Watch::from(vec![review(start)]);
    watch.look(&cgroups, &agents(), start - 10 * MIN);
    cgroups
        .on
        .borrow_mut()
        .remove(Path::new("/cg/agents.slice"));
    let said = watch.look(&cgroups, &agents(), start - 9 * MIN);
    assert!(
        cgroups.on.borrow().contains(Path::new("/cg/agents.slice")),
        "a slice that lost the program wasn't guarded again"
    );
    assert_eq!(
        said,
        vec!["<6>guarded /cg/agents.slice"],
        "guarding it again wasn't said"
    );
}

#[test]
fn an_unreadable_calendar_keeps_the_meetings_it_last_said() {
    let start = 1_000 * MIN;
    let cgroups = Fake::default();
    let mut watch = Watch::default();
    let (_, changed) = watch.heard(Ok(vec![review(start)]));
    assert!(changed, "the first meetings weren't kept");
    let (said, changed) = watch.heard(Err("io: failed to lookup address".into()));
    assert!(!changed, "an unreadable calendar changed the meetings");
    assert_eq!(
        said,
        vec![
            "<4>the calendar can't be read, so the 1 meetings it last said stand: \
             io: failed to lookup address"
        ],
        "the calendar's failure wasn't said"
    );
    assert!(
        watch
            .heard(Err("io: failed to lookup address".into()))
            .0
            .is_empty(),
        "the calendar's failure was said again at every read"
    );
    // A stall after a refusal, as main.rs's watch hears one, is a new cause.
    assert_eq!(
        watch
            .heard(Err("google.sock hasn't answered in a minute".into()))
            .0,
        vec!["<4>the calendar still can't be read, now: google.sock hasn't answered in a minute"],
        "a new cause went unsaid, leaving the journal naming the old one"
    );
    // A daemon that stalls each ask and then fails it alternates between
    // the two, each already said.
    for why in [
        "google.sock hasn't answered in a minute",
        "io: failed to lookup address",
        "google.sock hasn't answered in a minute",
    ] {
        assert!(
            watch.heard(Err(why.into())).0.is_empty(),
            "a cause already said was said again: {why}"
        );
    }
    watch.look(&cgroups, &agents(), start - 5 * MIN);
    assert_eq!(
        cgroups.on.borrow().len(),
        2,
        "a known meeting didn't keep the guard up while the calendar was unreadable"
    );
    let (said, changed) = watch.heard(Ok(Vec::new()));
    assert!(changed, "a meeting taken off the calendar was kept");
    assert_eq!(
        said,
        vec!["<5>the calendar answers again"],
        "the calendar's return wasn't said"
    );
}

#[test]
fn a_missing_slice_is_passed_over_and_a_refusal_is_said_once() {
    let start = 1_000 * MIN;
    let cgroups = Fake {
        missing: [PathBuf::from("/cg/jobs.slice")].into(),
        refused: [PathBuf::from("/cg/agents.slice")].into(),
        ..Fake::default()
    };
    let mut watch = Watch::from(vec![review(start)]);
    let said = watch.look(&cgroups, &agents(), start - 10 * MIN);
    assert_eq!(
        said,
        vec![
            "<5>agents may open no camera until Design review ends",
            "<3>couldn't guard /cg/agents.slice: permission denied",
        ],
        "a missing slice was said, or a refused one wasn't"
    );
    assert!(
        watch.look(&cgroups, &agents(), start - 9 * MIN).is_empty(),
        "the same refusal was said at every look"
    );
}
