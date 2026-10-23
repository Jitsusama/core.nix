//! A journey walked on a screen: the next step found afresh each time,
//! what it needs asked one thing at a time, then run to its proof. Return
//! goes on, Escape goes back one ask, and Ctrl+C leaves, even partway
//! through a step at work, whose programs are ended first; a step that's
//! blocked or fails is looked at again with Return once the person has
//! done what it said. Every step done stays on the screen, and a step at
//! work says how far along it is, once a second, until it's done.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use zeroize::Zeroizing;

use crate::progress::Meter;
use crate::screen::Theme;
use crate::screen::terminal::{Io, Press};
use crate::screen::view::{Moment, Where, draw};
use crate::{Answer, Answers, Ask, Fault, Journey, Machine, Next, Place, Secret, Step, words};

/// How a walk ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// Every step done.
    Finished,
    /// Everything here done, and the next step waits at another place.
    Elsewhere { step: &'static str },
    /// The person left.
    Left,
}

/// Everything a walk draws on and reads from.
pub struct Walk<'a> {
    pub journey: &'a Journey,
    pub here: Place,
    /// What the journey is, as the person would say it.
    pub name: &'a str,
    pub theme: &'a Theme,
    /// The raw log, for Tab when a step fails.
    pub log: Option<&'a Path>,
    /// Whether the journey is all its steps, so how far along it is can be
    /// said: not while it's only the steps that find out the rest.
    pub whole: bool,
    /// What each step done showed, kept from one journey to the next, so
    /// the steps that found out the rest still say what they did.
    pub done: std::cell::RefCell<Vec<String>>,
}

fn number(journey: &Journey, step: &dyn Step) -> (usize, usize) {
    let steps: Vec<&str> = journey.steps().map(Step::id).collect();
    let at = steps.iter().position(|id| *id == step.id()).unwrap_or(0);
    (at + 1, steps.len())
}

/// How often a step at work is drawn again.
const TICK: Duration = Duration::from_secs(1);

/// How often a screen saying what's in the way looks at the machine again,
/// so it goes on by itself once the person has done what it said.
const LOOK_AGAIN: Duration = Duration::from_secs(2);

/// A step at work, drawn again once a second from a thread of its own,
/// until it's stopped.
struct Ticking {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Ticking {
    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Walk<'_> {
    fn place(&self, step: Option<&dyn Step>) -> (usize, Option<usize>) {
        let (at, of) = if let Some(step) = step {
            number(self.journey, step)
        } else {
            let of = self.journey.steps().count();
            (of, of)
        };
        (at, self.whole.then_some(of))
    }

    fn show(
        &self,
        io: &mut dyn Io,
        step: Option<&dyn Step>,
        done: &[String],
        moment: &Moment,
    ) -> Result<(), Fault> {
        let (at, steps) = self.place(step);
        let (width, height) = io.size();
        let at = Where {
            journey: self.name,
            step: at,
            steps,
            done,
        };
        io.show(&draw(self.theme, &at, moment, width, height), self.theme)
    }

    /// The step's working screen, drawn now and then once a second with
    /// what its meter says, where the screen can be drawn from another
    /// thread.
    fn tick(
        &self,
        io: &mut dyn Io,
        step: &dyn Step,
        done: &[String],
        meanwhile: &str,
        meter: &Meter,
    ) -> Result<Ticking, Fault> {
        let said = meter.said();
        self.show(
            io,
            Some(step),
            done,
            &Moment::Working {
                title: step.title(),
                meanwhile,
                progress: Some(&said),
            },
        )?;
        let stopping = Arc::new(AtomicBool::new(false));
        let Some(mut painter) = io.painter() else {
            return Ok(Ticking {
                stop: stopping,
                thread: None,
            });
        };
        let (at, steps) = self.place(Some(step));
        let journey = self.name.to_string();
        let title = step.title().to_string();
        let meanwhile = meanwhile.to_string();
        let done = done.to_vec();
        let theme = self.theme.clone();
        let meter = meter.clone();
        let stopped = stopping.clone();
        let thread = std::thread::spawn(move || {
            let slice = Duration::from_millis(100);
            loop {
                let mut waited = Duration::ZERO;
                while waited < TICK {
                    if stopped.load(Ordering::Relaxed) {
                        return;
                    }
                    if painter.left(slice) {
                        painter.stop(&title, meter.program());
                        return;
                    }
                    waited += slice;
                }
                let said = meter.said();
                let at = Where {
                    journey: &journey,
                    step: at,
                    steps,
                    done: &done,
                };
                let moment = Moment::Working {
                    title: &title,
                    meanwhile: &meanwhile,
                    progress: Some(&said),
                };
                let (width, height) = painter.size();
                if painter
                    .paint(&draw(&theme, &at, &moment, width, height), &theme)
                    .is_err()
                {
                    return;
                }
            }
        });
        Ok(Ticking {
            stop: stopping,
            thread: Some(thread),
        })
    }

    /// Says to put the stick back in, until it is.
    fn stick_out(
        &self,
        io: &mut dyn Io,
        step: Option<&dyn Step>,
        done: &[String],
    ) -> Result<bool, Fault> {
        self.show(
            io,
            step,
            done,
            &Moment::Blocked {
                title: "Put the stick back in",
                why: "the next step needs what's on the stick",
                remedy: "plug the stick back in, into any port: the guide carries on by itself once it's in",
            },
        )?;
        wait_or_look(io)
    }

    /// The last lines of the log, until Tab or Return.
    fn read_log(&self, io: &mut dyn Io, step: &dyn Step, done: &[String]) -> Result<bool, Fault> {
        let text = self
            .log
            .and_then(|log| std::fs::read_to_string(log).ok())
            .unwrap_or_else(|| "nothing has been logged".into());
        let (_, height) = io.size();
        let tail: Vec<&str> = text
            .lines()
            .rev()
            .take(height.saturating_sub(8) as usize)
            .collect();
        let said: String = tail.into_iter().rev().collect::<Vec<_>>().join(" / ");
        self.show(
            io,
            Some(step),
            done,
            &Moment::Working {
                title: "The log",
                meanwhile: &said,
                progress: None,
            },
        )?;
        loop {
            match io.press()? {
                Press::Tab | Press::Return | Press::Escape => return Ok(true),
                Press::Leave => return Ok(false),
                _ => {}
            }
        }
    }

    /// Everything a step asks for, one at a time. None when the person
    /// left.
    fn ask(
        &self,
        io: &mut dyn Io,
        step: &dyn Step,
        asks: &[Ask],
        done: &[String],
    ) -> Result<Option<Answers>, Fault> {
        let mut given: Vec<Option<Answer>> = asks.iter().map(|_| None).collect();
        let mut at = 0;
        let mut typed = Zeroizing::new(String::new());
        let mut chosen = 0;
        let mut refused: Option<String> = None;
        // Whether what's shown for the person's phone is on the screen alone.
        let mut alone = false;
        while at < asks.len() {
            let ask = &asks[at];
            let moment = match ask {
                Ask::Show { shown, .. } if alone => Moment::Showing { shown },
                _ => Moment::Asking {
                    title: step.title(),
                    ask,
                    typed: &typed,
                    chosen,
                    refused: refused.as_deref(),
                    back: at > 0,
                },
            };
            self.show(io, Some(step), done, &moment)?;
            match io.press()? {
                Press::Leave => return Ok(None),
                Press::Char(c)
                    if !matches!(ask, Ask::Act { .. } | Ask::Choose { .. } | Ask::Show { .. }) =>
                {
                    typed.push(c);
                }
                Press::Return if matches!(ask, Ask::Show { .. }) && !alone => alone = true,
                Press::Escape if alone => alone = false,
                Press::Up if matches!(ask, Ask::Choose { .. }) => chosen = chosen.saturating_sub(1),
                Press::Down => {
                    if let Ask::Choose { choices, .. } = ask {
                        chosen = (chosen + 1).min(choices.len().saturating_sub(1));
                    }
                }
                Press::Backspace => {
                    typed.pop();
                }
                Press::Escape if at > 0 => {
                    at -= 1;
                    given[at] = None;
                    typed.clear();
                    chosen = 0;
                    refused = None;
                }
                Press::Return => {
                    let answer = match ask {
                        Ask::Act { .. } | Ask::Show { .. } => Some(Answer::Done),
                        Ask::Choose { .. } => Some(Answer::Chosen(chosen)),
                        Ask::Secret { .. } | Ask::SecretOr { .. } if typed.is_empty() => None,
                        Ask::Secret { .. } | Ask::SecretOr { .. } => {
                            Some(Answer::Secret(Secret::new(typed.to_string())))
                        }
                        Ask::TypeBack { expected, .. } if typed.trim() == expected => {
                            Some(Answer::Typed(typed.trim().into()))
                        }
                        Ask::TypeBack { .. } => {
                            refused = Some("what was typed isn't what's shown above".into());
                            None
                        }
                        Ask::Recall { expected, .. }
                            if words::typed(&typed) == words::typed(expected) =>
                        {
                            Some(Answer::Typed(words::typed(&typed)))
                        }
                        Ask::Recall { .. } => {
                            refused = Some("that isn't it: read it off your phone again, or press Esc to see it on the screen again".into());
                            None
                        }
                    };
                    if let Some(answer) = answer {
                        given[at] = Some(answer);
                        at += 1;
                        typed.clear();
                        chosen = 0;
                        refused = None;
                        alone = false;
                    }
                }
                _ => {}
            }
        }
        let mut answers = Answers::new();
        for (ask, answer) in asks.iter().zip(given) {
            answers.give(ask.id(), answer.expect("every ask answered"));
        }
        Ok(Some(answers))
    }

    /// One of a few choices, asked on its own under a title, the arrows
    /// moving and Return choosing; none when the person leaves.
    pub fn choose(
        &self,
        io: &mut dyn Io,
        title: &str,
        question: &str,
        choices: &[&str],
    ) -> Result<Option<usize>, Fault> {
        let ask = Ask::Choose {
            id: "choice",
            question: question.into(),
            choices: choices.iter().map(ToString::to_string).collect(),
        };
        let mut chosen = 0;
        loop {
            let done = self.done.borrow().clone();
            self.show(
                io,
                None,
                &done,
                &Moment::Asking {
                    title,
                    ask: &ask,
                    typed: "",
                    chosen,
                    refused: None,
                    back: false,
                },
            )?;
            match io.press()? {
                Press::Up => chosen = chosen.saturating_sub(1),
                Press::Down => chosen = (chosen + 1).min(choices.len().saturating_sub(1)),
                Press::Return => return Ok(Some(chosen)),
                Press::Leave => return Ok(None),
                _ => {}
            }
        }
    }

    /// A card for what happens next away from the guide, until Return,
    /// which says to go on, or Ctrl+C.
    pub fn card(
        &self,
        io: &mut dyn Io,
        title: &str,
        said: &str,
        then: &str,
    ) -> Result<bool, Fault> {
        let done = self.done.borrow().clone();
        self.show(io, None, &done, &Moment::Card { title, said, then })?;
        wait(io)
    }

    pub fn walk(&self, machine: &mut dyn Machine, io: &mut dyn Io) -> Result<Ended, Fault> {
        let mut done = self.done.borrow_mut().split_off(0);
        let ended = self.walking(machine, io, &mut done);
        *self.done.borrow_mut() = done;
        ended
    }

    fn walking(
        &self,
        machine: &mut dyn Machine,
        io: &mut dyn Io,
        done: &mut Vec<String>,
    ) -> Result<Ended, Fault> {
        loop {
            let next = match self.journey.next(self.here, machine) {
                Ok(next) => next,
                Err(Fault::StickOut) => {
                    if !self.stick_out(io, None, done)? {
                        return Ok(Ended::Left);
                    }
                    continue;
                }
                // A machine that can't be read yet, as a key not plugged
                // in, is said and looked at again.
                Err(fault) => {
                    let title = "Looking at the machine";
                    let fault = fault.to_string();
                    self.show(
                        io,
                        None,
                        done,
                        &Moment::Failed {
                            title,
                            fault: &fault,
                        },
                    )?;
                    if !wait_or_look(io)? {
                        return Ok(Ended::Left);
                    }
                    continue;
                }
            };
            match next {
                Next::Finished => {
                    let last = done.last().cloned();
                    let shown = &done[..done.len().saturating_sub(1)];
                    self.show(
                        io,
                        None,
                        shown,
                        &Moment::Finished {
                            said: last.as_deref().unwrap_or("everything is done"),
                        },
                    )?;
                    return Ok(Ended::Finished);
                }
                Next::Elsewhere { step } => return Ok(Ended::Elsewhere { step: step.id() }),
                Next::Blocked { step, why, remedy } => {
                    self.show(
                        io,
                        Some(step),
                        done,
                        &Moment::Blocked {
                            title: step.title(),
                            why: &why,
                            remedy: &remedy,
                        },
                    )?;
                    if !wait_or_look(io)? {
                        return Ok(Ended::Left);
                    }
                }
                Next::Run { step, asks } => {
                    let Some(answers) = self.ask(io, step, &asks, done)? else {
                        return Ok(Ended::Left);
                    };
                    let meanwhile = asks
                        .iter()
                        .find_map(|ask| match ask {
                            Ask::Act { what, .. } => Some(what.as_str()),
                            _ => None,
                        })
                        .unwrap_or("");
                    let meter = machine.meter();
                    meter.start();
                    let ticking = self.tick(io, step, done, meanwhile, &meter)?;
                    let attempt = Journey::attempt(step, machine, &answers);
                    ticking.stop();
                    match attempt {
                        Ok(evidence) => done.push(evidence),
                        Err(Fault::StickOut) => {
                            if !self.stick_out(io, Some(step), done)? {
                                return Ok(Ended::Left);
                            }
                        }
                        Err(fault) => {
                            if !self.failed(io, step, done, &fault)? {
                                return Ok(Ended::Left);
                            }
                        }
                    }
                }
            }
        }
    }

    /// What stopped a step, said until Return goes on, Tab showing the log
    /// meanwhile, or Ctrl+C. Says whether to go on.
    fn failed(
        &self,
        io: &mut dyn Io,
        step: &dyn Step,
        done: &[String],
        fault: &Fault,
    ) -> Result<bool, Fault> {
        loop {
            let said = fault.said();
            self.show(
                io,
                Some(step),
                done,
                &Moment::Failed {
                    title: step.title(),
                    fault: &said,
                },
            )?;
            match io.press()? {
                Press::Return => return Ok(true),
                Press::Leave => return Ok(false),
                Press::Tab if !self.read_log(io, step, done)? => return Ok(false),
                _ => {}
            }
        }
    }
}

/// Waits for Return, or for the machine to change, looking at it again
/// every so often by itself, or for Ctrl+C. Says whether to go on.
fn wait_or_look(io: &mut dyn Io) -> Result<bool, Fault> {
    match io.press_within(LOOK_AGAIN)? {
        Some(Press::Leave) => Ok(false),
        _ => Ok(true),
    }
}

/// Waits for Return, or Ctrl+C. Says whether to go on.
fn wait(io: &mut dyn Io) -> Result<bool, Fault> {
    loop {
        match io.press()? {
            Press::Return => return Ok(true),
            Press::Leave => return Ok(false),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;
    use crate::fake::Fake;
    use crate::screen::plain;
    use crate::screen::terminal::{Keys, Paint, Screen};
    use crate::screen::view::Drawn;
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
            Ok(self.keys.pop_front().unwrap_or(Press::Leave))
        }

        /// No key pressed in time, once the keys run out.
        fn press_within(&mut self, _wait: Duration) -> Result<Option<Press>, Fault> {
            Ok(self.keys.pop_front())
        }
    }

    impl Screen for Stand {
        fn size(&self) -> (u16, u16) {
            (80, 24)
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
            .walk(&mut fake(), &mut stand)
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
                .any(|line| line.to_lowercase().contains("everything is done")),
            "{shown:#?}"
        );
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

    fn fake() -> Fake {
        let mut fake = Fake::new();
        fake.no_file("/copied").no_file("/pin");
        fake
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
            .walk(&mut fake(), &mut stand)
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
                .any(|line| line.contains("something was copied")),
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
            .walk(&mut fake(), &mut stand)
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
    fn every_step_done_stays_on_the_screen_for_the_next() {
        let journey = Journey::new(vec![Box::new(Slow), Box::new(Asking)]).unwrap();
        let theme = Theme::from("", "", true);
        let mut stand = Stand::default();
        stand.keys.extend([Press::Char('1'), Press::Return]);
        walk(&journey, &theme)
            .walk(&mut fake(), &mut stand)
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
}
