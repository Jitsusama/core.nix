use super::*;
use std::cell::RefCell;
use std::collections::BTreeSet;

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
        watch.heard(Err("again".into())).0.is_empty(),
        "the calendar's failure was said again at every read"
    );
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
