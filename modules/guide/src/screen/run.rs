//! A journey walked on a screen: the next step found afresh each time,
//! what it needs asked one thing at a time, then run to its proof. Return
//! goes on, Escape goes back one ask, and Ctrl+C leaves; a step that's
//! blocked or fails is looked at again with Return once the person has
//! done what it said.

use std::path::Path;

use zeroize::Zeroizing;

use crate::screen::Theme;
use crate::screen::terminal::{Io, Press};
use crate::screen::view::{Moment, Where, draw};
use crate::{Answer, Answers, Ask, Fault, Journey, Machine, Next, Place, Secret, Step};

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
    /// What the last step done showed, kept from one journey to the next,
    /// so the steps that found out the rest still say what they did.
    pub last: std::cell::RefCell<Option<String>>,
}

fn number(journey: &Journey, step: &dyn Step) -> (usize, usize) {
    let steps: Vec<&str> = journey.steps().map(|step| step.id()).collect();
    let at = steps.iter().position(|id| *id == step.id()).unwrap_or(0);
    (at + 1, steps.len())
}

impl Walk<'_> {
    fn show(
        &self,
        io: &mut dyn Io,
        step: Option<&dyn Step>,
        last: Option<&str>,
        moment: Moment,
    ) -> Result<(), Fault> {
        let (at, of) = match step {
            Some(step) => number(self.journey, step),
            None => {
                let of = self.journey.steps().count();
                (of, of)
            }
        };
        let (width, height) = io.size();
        let at = Where {
            journey: self.name,
            step: at,
            steps: self.whole.then_some(of),
            last,
        };
        io.show(&draw(self.theme, &at, &moment, width, height), self.theme)
    }

    /// Waits for Return, or Ctrl+C. Says whether to go on.
    fn wait(&self, io: &mut dyn Io) -> Result<bool, Fault> {
        loop {
            match io.press()? {
                Press::Return => return Ok(true),
                Press::Leave => return Ok(false),
                _ => {}
            }
        }
    }

    /// The last lines of the log, until Tab or Return.
    fn read_log(&self, io: &mut dyn Io, step: &dyn Step) -> Result<bool, Fault> {
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
            None,
            Moment::Working {
                title: "The log",
                meanwhile: &said,
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
        last: Option<&str>,
    ) -> Result<Option<Answers>, Fault> {
        let mut given: Vec<Option<Answer>> = asks.iter().map(|_| None).collect();
        let mut at = 0;
        let mut typed = Zeroizing::new(String::new());
        let mut chosen = 0;
        let mut refused: Option<String> = None;
        while at < asks.len() {
            let ask = &asks[at];
            self.show(
                io,
                Some(step),
                last,
                Moment::Asking {
                    title: step.title(),
                    ask,
                    typed: &typed,
                    chosen,
                    refused: refused.as_deref(),
                },
            )?;
            match io.press()? {
                Press::Leave => return Ok(None),
                Press::Char(c) if !matches!(ask, Ask::Act { .. } | Ask::Choose { .. }) => {
                    typed.push(c)
                }
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
                        Ask::Act { .. } => Some(Answer::Done),
                        Ask::Choose { .. } => Some(Answer::Chosen(chosen)),
                        Ask::Secret { .. } if typed.is_empty() => None,
                        Ask::Secret { .. } => Some(Answer::Secret(Secret::new(typed.to_string()))),
                        Ask::TypeBack { expected, .. } if typed.trim() == expected => {
                            Some(Answer::Typed(typed.trim().into()))
                        }
                        Ask::TypeBack { .. } => {
                            refused = Some("what was typed isn't what's shown above".into());
                            None
                        }
                    };
                    if let Some(answer) = answer {
                        given[at] = Some(answer);
                        at += 1;
                        typed.clear();
                        chosen = 0;
                        refused = None;
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
            choices: choices.iter().map(|choice| choice.to_string()).collect(),
        };
        let mut chosen = 0;
        loop {
            let last = self.last.borrow().clone();
            self.show(
                io,
                None,
                last.as_deref(),
                Moment::Asking {
                    title,
                    ask: &ask,
                    typed: "",
                    chosen,
                    refused: None,
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
        self.show(io, None, None, Moment::Card { title, said, then })?;
        self.wait(io)
    }

    pub fn walk(&self, machine: &mut dyn Machine, io: &mut dyn Io) -> Result<Ended, Fault> {
        let mut last = self.last.borrow_mut().take();
        let ended = self.walking(machine, io, &mut last);
        *self.last.borrow_mut() = last;
        ended
    }

    fn walking(
        &self,
        machine: &mut dyn Machine,
        io: &mut dyn Io,
        last: &mut Option<String>,
    ) -> Result<Ended, Fault> {
        loop {
            let next = match self.journey.next(self.here, machine) {
                Ok(next) => next,
                // A machine that can't be read yet, as a key not plugged
                // in, is said and looked at again.
                Err(fault) => {
                    let title = "Looking at the machine";
                    let fault = fault.to_string();
                    self.show(
                        io,
                        None,
                        last.as_deref(),
                        Moment::Failed {
                            title,
                            fault: &fault,
                        },
                    )?;
                    if !self.wait(io)? {
                        return Ok(Ended::Left);
                    }
                    continue;
                }
            };
            match next {
                Next::Finished => {
                    self.show(
                        io,
                        None,
                        None,
                        Moment::Finished {
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
                        last.as_deref(),
                        Moment::Blocked {
                            title: step.title(),
                            why: &why,
                            remedy: &remedy,
                        },
                    )?;
                    if !self.wait(io)? {
                        return Ok(Ended::Left);
                    }
                }
                Next::Run { step, asks } => {
                    let Some(answers) = self.ask(io, step, &asks, last.as_deref())? else {
                        return Ok(Ended::Left);
                    };
                    let meanwhile = asks
                        .iter()
                        .find_map(|ask| match ask {
                            Ask::Act { what, .. } => Some(what.as_str()),
                            _ => None,
                        })
                        .unwrap_or("");
                    self.show(
                        io,
                        Some(step),
                        last.as_deref(),
                        Moment::Working {
                            title: step.title(),
                            meanwhile,
                        },
                    )?;
                    match Journey::attempt(step, machine, &answers) {
                        Ok(evidence) => *last = Some(evidence),
                        Err(fault) => loop {
                            let fault = fault.said();
                            self.show(
                                io,
                                Some(step),
                                last.as_deref(),
                                Moment::Failed {
                                    title: step.title(),
                                    fault: &fault,
                                },
                            )?;
                            match io.press()? {
                                Press::Return => break,
                                Press::Leave => return Ok(Ended::Left),
                                Press::Tab if !self.read_log(io, step)? => return Ok(Ended::Left),
                                _ => {}
                            }
                        },
                    }
                }
            }
        }
    }
}
