//! The journey's promises, each with toy steps on a fake machine: what
//! comes next is found by observing; a step is done only when observing
//! again says so; nothing runs without everything it asks for; and a step
//! already done never runs again.

use std::cell::Cell;
use std::rc::Rc;

use guide::fake::Fake;
use guide::{
    Answer, Answers, Ask, Fault, Journey, Machine, Next, Place, Secret, Standing, Step, Tool,
};

/// A step whose work is a fact in the journal, which it observes. It can
/// be told to claim success without doing its work, as a broken step would.
struct Note {
    id: &'static str,
    place: Place,
    asks: Vec<Ask>,
    /// Does nothing when it runs, and says it worked.
    lies: bool,
    runs: Rc<Cell<u32>>,
}

fn note(id: &'static str, place: Place) -> Note {
    Note {
        id,
        place,
        asks: vec![],
        lies: false,
        runs: Rc::new(Cell::new(0)),
    }
}

impl Step for Note {
    fn id(&self) -> &'static str {
        self.id
    }

    fn place(&self) -> Place {
        self.place
    }

    fn title(&self) -> &'static str {
        "Note it down"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(if machine.journal().knows(self.id, "noted") {
            Standing::Done {
                evidence: "the journal has it".into(),
            }
        } else {
            Standing::Needed {
                asks: self.asks.clone(),
            }
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        self.runs.set(self.runs.get() + 1);
        if self.lies {
            Ok(())
        } else {
            machine.journal().record(self.id, "noted")
        }
    }
}

/// A step that's blocked until the TPM has wrong PINs to spare.
struct ReadsTheTpm;

impl Step for ReadsTheTpm {
    fn id(&self) -> &'static str {
        "tpm"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Check the TPM"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let lockout = guide::observe::tpm::Lockout::observe(machine)?;
        Ok(if lockout.left() < 3 {
            Standing::Blocked {
                why: format!("the TPM allows {} more wrong PINs", lockout.left()),
                remedy: "wait for it to forgive them".into(),
            }
        } else {
            Standing::Done {
                evidence: format!("{} wrong PINs to spare", lockout.left()),
            }
        })
    }

    fn run(&self, _machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        Ok(())
    }
}

fn journey(steps: Vec<Box<dyn Step>>) -> Journey {
    Journey::new(steps).unwrap()
}

#[test]
fn a_step_that_says_it_worked_but_didnt_is_unproven() {
    let mut liar = note("liar", Place::Stick);
    liar.lies = true;
    let mut machine = Fake::new();
    let result = Journey::attempt(&liar, &mut machine, &Answers::new());
    assert!(
        matches!(&result, Err(Fault::Unproven { step: "liar", .. })),
        "{result:?}"
    );
    assert_eq!(liar.runs.get(), 1);
}

#[test]
fn a_step_is_done_with_what_shows_it() {
    let step = note("format", Place::Stick);
    let mut machine = Fake::new();
    assert_eq!(
        Journey::attempt(&step, &mut machine, &Answers::new()),
        Ok("the journal has it".into())
    );
}

#[test]
fn a_step_already_done_never_runs_again() {
    let step = note("format", Place::Stick);
    let mut machine = Fake::new();
    machine.journal().record("format", "noted").unwrap();
    assert!(Journey::attempt(&step, &mut machine, &Answers::new()).is_ok());
    assert_eq!(step.runs.get(), 0);
}

#[test]
fn nothing_runs_without_everything_it_asks_for() {
    let mut step = note("enrol", Place::Stick);
    step.asks = vec![
        Ask::Secret {
            id: "pin",
            name: "the Nano's PIN".into(),
        },
        Ask::Act {
            id: "plug",
            what: "plug in the Nano".into(),
        },
    ];
    let mut machine = Fake::new();
    let mut answers = Answers::new();
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    assert_eq!(
        Journey::attempt(&step, &mut machine, &answers),
        Err(Fault::Unanswered {
            step: "enrol",
            ask: "plug"
        })
    );
    assert_eq!(step.runs.get(), 0);

    answers.give("plug", Answer::Done);
    assert!(Journey::attempt(&step, &mut machine, &answers).is_ok());
}

#[test]
fn next_is_the_first_step_here_not_done() {
    let journey = journey(vec![
        Box::new(note("write", Place::Host)),
        Box::new(note("format", Place::Stick)),
        Box::new(note("install", Place::Stick)),
    ]);
    let mut machine = Fake::new();
    machine.journal().record("format", "noted").unwrap();
    let Next::Run { step, .. } = journey.next(Place::Stick, &mut machine).unwrap() else {
        panic!("a step to run")
    };
    assert_eq!(step.id(), "install");
}

#[test]
fn steps_behind_the_journey_are_never_observed() {
    // The host's step isn't done, but the stick can't see the host, so it
    // mustn't hold the stick's steps back.
    let journey = journey(vec![
        Box::new(note("write", Place::Host)),
        Box::new(note("format", Place::Stick)),
    ]);
    let mut machine = Fake::new();
    let Next::Run { step, .. } = journey.next(Place::Stick, &mut machine).unwrap() else {
        panic!("a step to run")
    };
    assert_eq!(step.id(), "format");
}

#[test]
fn when_everything_here_is_done_it_says_where_to_go() {
    let journey = journey(vec![
        Box::new(note("format", Place::Stick)),
        Box::new(note("seal", Place::Installed)),
    ]);
    let mut machine = Fake::new();
    machine.journal().record("format", "noted").unwrap();
    let Next::Elsewhere { step } = journey.next(Place::Stick, &mut machine).unwrap() else {
        panic!("elsewhere")
    };
    assert_eq!(step.place(), Place::Installed);

    machine.journal().record("seal", "noted").unwrap();
    assert!(matches!(
        journey.next(Place::Installed, &mut machine).unwrap(),
        Next::Finished
    ));
}

#[test]
fn the_guide_starting_again_finds_the_same_next_step() {
    // A reboot between steps is the guide starting again with what the
    // machine kept: here, the journal on its disk.
    let journey = journey(vec![
        Box::new(note("format", Place::Stick)),
        Box::new(note("install", Place::Stick)),
    ]);
    let mut machine = Fake::new();
    let Next::Run { step, .. } = journey.next(Place::Stick, &mut machine).unwrap() else {
        panic!("a step to run")
    };
    Journey::attempt(step, &mut machine, &Answers::new()).unwrap();

    let Next::Run { step, .. } = journey.next(Place::Stick, &mut machine).unwrap() else {
        panic!("a step to run")
    };
    assert_eq!(step.id(), "install");
}

#[test]
fn a_blocked_step_says_why_and_what_to_do() {
    let journey = journey(vec![Box::new(ReadsTheTpm)]);
    let mut machine = Fake::new();
    let listing =
        include_str!("../fixtures/microsoft-surface-laptop-4-1958/tpm2-properties-variable.txt")
            .replace(
                "TPM2_PT_LOCKOUT_COUNTER: 0x0",
                "TPM2_PT_LOCKOUT_COUNTER: 0x1E",
            );
    machine.prints(
        Tool::Tpm2Getcap,
        &["-T", "device:/dev/tpmrm0", "properties-variable"],
        &listing,
    );
    let Next::Blocked { why, remedy, .. } = journey.next(Place::Installed, &mut machine).unwrap()
    else {
        panic!("blocked")
    };
    assert_eq!(why, "the TPM allows 2 more wrong PINs");
    assert_eq!(remedy, "wait for it to forgive them");

    let result = Journey::attempt(&ReadsTheTpm, &mut machine, &Answers::new());
    assert!(matches!(result, Err(Fault::Blocked { step: "tpm", .. })));
}

#[test]
fn a_journey_never_goes_back_to_a_place_it_left() {
    let result = Journey::new(vec![
        Box::new(note("seal", Place::Installed)),
        Box::new(note("format", Place::Stick)),
    ]);
    assert!(result.is_err());
}

#[test]
fn no_two_steps_share_an_id() {
    let result = Journey::new(vec![
        Box::new(note("format", Place::Stick)),
        Box::new(note("format", Place::Stick)),
    ]);
    assert_eq!(result.err().as_deref(), Some("two steps are called format"));
}
