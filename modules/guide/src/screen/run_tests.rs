use std::collections::VecDeque;
use std::sync::Mutex;

use super::*;
use crate::screen::plain;
use crate::screen::terminal::{Keys, Paint, Screen};
use crate::screen::view::Drawn;
use crate::stand_in::StandIn;
use crate::{Standing, Step};

/// Keys pressed in turn, and every screen drawn, from the walk's own
/// thread and from a working step's.
#[derive(Default)]
struct Stand {
    keys: VecDeque<Press>,
    shown: Arc<Mutex<Vec<Vec<String>>>>,
    paints: bool,
    /// How many times the painter waits for a key before Ctrl+C is
    /// pressed, if ever.
    leaves_after: Option<usize>,
    /// The step the painter was told to stop partway through, if any.
    stopped: Arc<Mutex<Vec<String>>>,
    /// The size the screen is, 80 by 24 until it's changed.
    size: Option<(u16, u16)>,
    /// The size it changes to as each `Press::Resized` is read.
    resizes: VecDeque<(u16, u16)>,
}

fn text(drawn: &Drawn) -> Vec<String> {
    drawn
        .lines
        .iter()
        .map(plain)
        .map(|line| line.trim_end().to_string())
        .collect()
}

impl Keys for Stand {
    fn press(&mut self) -> Result<Press, Fault> {
        let press = self.keys.pop_front().unwrap_or(Press::Leave);
        if press == Press::Resized {
            self.size = self.resizes.pop_front();
        }
        Ok(press)
    }

    /// No key pressed in time, once the keys run out.
    fn press_within(&mut self, _wait: Duration) -> Result<Option<Press>, Fault> {
        Ok(self.keys.pop_front())
    }
}

impl Screen for Stand {
    fn size(&self) -> (u16, u16) {
        self.size.unwrap_or((80, 24))
    }

    fn show(&mut self, drawn: &Drawn, _theme: &Theme) -> Result<(), Fault> {
        self.shown.lock().unwrap().push(text(drawn));
        Ok(())
    }

    fn painter(&self) -> Option<Box<dyn Paint>> {
        self.paints.then(|| {
            Box::new(Recorder {
                shown: self.shown.clone(),
                leaves_after: self.leaves_after,
                waited: 0,
                stopped: self.stopped.clone(),
            }) as Box<dyn Paint>
        })
    }
}

struct Recorder {
    shown: Arc<Mutex<Vec<Vec<String>>>>,
    leaves_after: Option<usize>,
    waited: usize,
    stopped: Arc<Mutex<Vec<String>>>,
}

impl Paint for Recorder {
    fn size(&self) -> (u16, u16) {
        (80, 24)
    }

    fn paint(&mut self, drawn: &Drawn, _theme: &Theme) -> Result<(), Fault> {
        self.shown.lock().unwrap().push(text(drawn));
        Ok(())
    }

    fn left(&mut self, within: Duration) -> bool {
        std::thread::sleep(within);
        self.waited += 1;
        Some(self.waited) == self.leaves_after
    }

    /// Only says so: the real one ends the guide.
    fn stop(&mut self, during: &str, _running: Option<u32>) {
        self.stopped.lock().unwrap().push(during.into());
    }
}

/// Copies something slowly, feeding the meter halfway.
struct Slow;

impl Step for Slow {
    fn id(&self) -> &'static str {
        "slow"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Copy something slowly"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(match machine.read("/copied")? {
            Some(_) => Standing::Done {
                evidence: "something was copied".into(),
            },
            None => Standing::Needed { asks: vec![] },
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let meter = machine.meter();
        meter.total(1000);
        std::thread::sleep(Duration::from_millis(1300));
        meter.advance(500);
        std::thread::sleep(Duration::from_millis(1300));
        machine.write("/copied", b"yes")
    }
}

/// Asks for a word, done once it's been given.
struct Asking;

impl Step for Asking {
    fn id(&self) -> &'static str {
        "asking"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Give a PIN"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(match machine.read("/pin")? {
            Some(_) => Standing::Done {
                evidence: "the PIN was given".into(),
            },
            None => Standing::Needed {
                asks: vec![Ask::Secret {
                    id: "pin",
                    name: "a PIN".into(),
                }],
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        machine.write("/pin", b"given")
    }
}

/// Needs the stick, which is out for the first few looks.
struct NeedsStick {
    looks: std::cell::Cell<u32>,
}

impl Step for NeedsStick {
    fn id(&self) -> &'static str {
        "needs-stick"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Read the stick"
    }

    fn observe(&self, _machine: &mut dyn Machine) -> Result<Standing, Fault> {
        self.looks.set(self.looks.get() + 1);
        if self.looks.get() < 3 {
            Err(Fault::StickOut)
        } else {
            Ok(Standing::Done {
                evidence: "the stick was read".into(),
            })
        }
    }

    fn run(&self, _machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        Ok(())
    }
}

#[test]
fn with_the_stick_out_it_says_to_put_it_back_and_goes_on_by_itself_once_it_is() {
    let journey = Journey::new(vec![Box::new(NeedsStick {
        looks: std::cell::Cell::default(),
    })])
    .unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand::default();
    let ended = walk(&journey, &theme)
        .walk(&mut stand_in(), &mut stand)
        .unwrap();
    assert_eq!(ended, Ended::Finished, "no key was pressed");
    let shown = stand.shown.lock().unwrap().clone();
    let out = shown
        .iter()
        .filter(|lines| lines.iter().any(|line| line == "  Put the stick back in"))
        .count();
    assert_eq!(out, 2, "{shown:#?}");
    assert!(
        shown[0]
            .iter()
            .any(|line| line.contains("Plug the stick back in, into any port")),
        "{shown:#?}"
    );
    assert!(
        shown
            .last()
            .unwrap()
            .iter()
            .any(|line| line.contains("The stick was read")),
        "{shown:#?}"
    );
}

/// A screen that changes size, as the stick's console does when its
/// font loads once the graphics driver has taken it over, is laid out
/// again for its new size, so no line runs off its edge and its keys
/// stay at its foot.
fn laid_out_again(shown: &[Vec<String>]) {
    assert_eq!(shown.len(), 2, "{shown:#?}");
    assert_eq!(shown[0].len(), 23, "{shown:#?}");
    assert_eq!(shown[1].len(), 49, "{shown:#?}");
    let widest = |lines: &[String]| lines.iter().map(|l| l.chars().count()).max();
    assert!(widest(&shown[0]) <= Some(80), "{shown:#?}");
    assert!(widest(&shown[1]) > Some(80), "{shown:#?}");
}

const LONG: &str = concat!(
    "This computer's disk already holds a machine: a disk with no model name, ",
    "8 GB, serial root. Recover it, or install hedgehog on it again?",
);

#[test]
fn a_choice_is_laid_out_again_when_the_screen_changes_size() {
    let journey = Journey::new(vec![]).unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand::default();
    stand.keys.extend([Press::Resized, Press::Return]);
    stand.resizes.push_back((160, 50));
    let chosen = walk(&journey, &theme)
        .choose(
            &mut stand,
            "A machine is already here",
            LONG,
            &["Recover it"],
        )
        .unwrap();
    assert_eq!(chosen, Some(0));
    laid_out_again(&stand.shown.lock().unwrap());
}

#[test]
fn a_card_is_laid_out_again_when_the_screen_changes_size() {
    let journey = Journey::new(vec![]).unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand::default();
    stand.keys.extend([Press::Resized, Press::Return]);
    stand.resizes.push_back((160, 50));
    let on = walk(&journey, &theme)
        .card(&mut stand, "Installed", LONG, "restart")
        .unwrap();
    assert!(on, "Return after the resize didn't carry on from the card");
    laid_out_again(&stand.shown.lock().unwrap());
}

fn walk<'a>(journey: &'a Journey, theme: &'a Theme) -> Walk<'a> {
    Walk {
        journey,
        here: Place::Stick,
        name: "Install hedgehog",
        theme,
        log: None,
        whole: true,
        done: std::cell::RefCell::default(),
    }
}

fn stand_in() -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.no_file("/copied").no_file("/pin");
    stand_in
}

#[test]
fn a_step_at_work_is_drawn_again_with_how_far_along_it_is() {
    let journey = Journey::new(vec![Box::new(Slow)]).unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand {
        paints: true,
        ..Stand::default()
    };
    let ended = walk(&journey, &theme)
        .walk(&mut stand_in(), &mut stand)
        .unwrap();
    assert_eq!(ended, Ended::Finished);
    let shown = stand.shown.lock().unwrap().clone();
    let working: Vec<&Vec<String>> = shown
        .iter()
        .filter(|lines| lines.iter().any(|line| line == "  Copy something slowly"))
        .collect();
    assert!(
        working.len() >= 3,
        "drawn once, then again each second: {shown:#?}"
    );
    assert!(
        working[0]
            .iter()
            .any(|line| line.contains("Running for 0 seconds")),
        "{shown:#?}"
    );
    assert!(
        working
            .iter()
            .any(|lines| lines.iter().any(|line| line.contains("500 bytes of 1 KB"))),
        "{shown:#?}"
    );
    assert!(
        shown
            .last()
            .unwrap()
            .iter()
            .any(|line| line.contains("Something was copied")),
        "{shown:#?}"
    );
}

#[test]
fn ctrl_c_while_a_step_is_at_work_stops_it_there() {
    let journey = Journey::new(vec![Box::new(Slow)]).unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand {
        paints: true,
        leaves_after: Some(5),
        ..Stand::default()
    };
    walk(&journey, &theme)
        .walk(&mut stand_in(), &mut stand)
        .unwrap();
    let stopped = stand.stopped.lock().unwrap().clone();
    assert_eq!(stopped, ["Copy something slowly"]);
    let shown = stand.shown.lock().unwrap().clone();
    let working: Vec<&Vec<String>> = shown
        .iter()
        .filter(|lines| lines.iter().any(|line| line == "  Copy something slowly"))
        .collect();
    assert_eq!(working.len(), 1, "nothing drawn once stopped: {shown:#?}");
    assert_eq!(working[0].last().unwrap(), "  Ctrl+C stop", "{shown:#?}");
}

#[test]
fn a_step_found_done_when_the_guide_starts_again_is_listed_as_done() {
    let journey = Journey::new(vec![Box::new(Slow), Box::new(Asking)]).unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand::default();
    stand.keys.extend([Press::Char('1'), Press::Return]);
    let mut machine = stand_in();
    machine.file("/copied", b"yes");
    walk(&journey, &theme)
        .walk(&mut machine, &mut stand)
        .unwrap();
    let shown = stand.shown.lock().unwrap().clone();
    assert!(
        shown[0]
            .iter()
            .any(|line| line.contains("Something was copied")),
        "{shown:#?}"
    );
}

#[test]
fn a_step_two_journeys_share_is_listed_once() {
    let opening = Journey::new(vec![Box::new(Slow)]).unwrap();
    let whole = Journey::new(vec![Box::new(Slow), Box::new(Asking)]).unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand::default();
    stand.keys.extend([Press::Char('1'), Press::Return]);
    let mut machine = stand_in();
    machine.file("/copied", b"yes");
    let mut walk = walk(&opening, &theme);
    walk.walk(&mut machine, &mut stand).unwrap();
    walk.journey = &whole;
    walk.walk(&mut machine, &mut stand).unwrap();
    let shown = stand.shown.lock().unwrap().clone();
    for lines in &shown {
        let listed = lines
            .iter()
            .filter(|line| line.contains("Something was copied"))
            .count();
        assert!(listed <= 1, "{lines:#?}");
    }
}

#[test]
fn every_step_done_stays_on_the_screen_for_the_next() {
    let journey = Journey::new(vec![Box::new(Slow), Box::new(Asking)]).unwrap();
    let theme = Theme::from("", "", true);
    let mut stand = Stand::default();
    stand.keys.extend([Press::Char('1'), Press::Return]);
    walk(&journey, &theme)
        .walk(&mut stand_in(), &mut stand)
        .unwrap();
    let shown = stand.shown.lock().unwrap().clone();
    let asking = shown
        .iter()
        .find(|lines| {
            lines
                .iter()
                .any(|line| line == "  Type a PIN, then press Return.")
        })
        .unwrap_or_else(|| panic!("{shown:#?}"));
    assert!(
        asking
            .iter()
            .any(|line| line.contains("Something was copied")),
        "{asking:#?}"
    );
}
