//! A journey: steps in order, place by place. What comes next is always
//! found by observing, never remembered, so the guide starting again after
//! a reboot, a crash or a pulled plug finds the same next step it would
//! have taken.

use crate::{Answers, Ask, Fault, Machine, Place, Standing, Step};

pub struct Journey {
    steps: Vec<Box<dyn Step>>,
}

/// What the person does next.
pub enum Next<'a> {
    /// Run this step, after giving it what it asks for.
    Run { step: &'a dyn Step, asks: Vec<Ask> },
    /// This step can't run here and now.
    Blocked {
        step: &'a dyn Step,
        why: String,
        remedy: String,
    },
    /// Everything here is done, and this step waits at its own place.
    Elsewhere { step: &'a dyn Step },
    /// Every step is done.
    Finished,
}

impl Journey {
    /// The steps in the order they're taken. Their places can only move
    /// forward, since a journey never goes back to a place it left, and no
    /// two may share an id, since the journal names a step by it.
    pub fn new(steps: Vec<Box<dyn Step>>) -> Result<Journey, String> {
        for pair in steps.windows(2) {
            if pair[1].place() < pair[0].place() {
                return Err(format!(
                    "{} comes after {} but runs on {}, which the journey has left",
                    pair[1].id(),
                    pair[0].id(),
                    pair[1].place()
                ));
            }
        }
        for (index, step) in steps.iter().enumerate() {
            if steps[..index]
                .iter()
                .any(|earlier| earlier.id() == step.id())
            {
                return Err(format!("two steps are called {}", step.id()));
            }
        }
        Ok(Journey { steps })
    }

    /// The first step here that isn't done. Steps at a place the journey
    /// has left are behind it, and can't be observed from here.
    pub fn next(&self, here: Place, machine: &mut dyn Machine) -> Result<Next<'_>, Fault> {
        for step in self.steps.iter().filter(|step| step.place() == here) {
            match step.observe(machine)? {
                Standing::Done { .. } => {}
                Standing::Needed { asks } => {
                    return Ok(Next::Run {
                        step: step.as_ref(),
                        asks,
                    });
                }
                Standing::Blocked { why, remedy } => {
                    return Ok(Next::Blocked {
                        step: step.as_ref(),
                        why,
                        remedy,
                    });
                }
            }
        }
        Ok(match self.steps.iter().find(|step| step.place() > here) {
            Some(step) => Next::Elsewhere {
                step: step.as_ref(),
            },
            None => Next::Finished,
        })
    }

    /// Runs a step and proves it: observes first, so a step already done is
    /// never run again; refuses to start without every answer it asks for;
    /// and only once observing again says it's done is it done. Gives back
    /// what shows it.
    pub fn attempt(
        step: &dyn Step,
        machine: &mut dyn Machine,
        answers: &Answers,
    ) -> Result<String, Fault> {
        match step.observe(machine)? {
            Standing::Done { evidence } => return Ok(evidence),
            Standing::Blocked { why, .. } => {
                return Err(Fault::Blocked {
                    step: step.id(),
                    why,
                });
            }
            Standing::Needed { asks } => {
                if let Some(ask) = asks.iter().find(|ask| !answers.has(ask.id())) {
                    return Err(Fault::Unanswered {
                        step: step.id(),
                        ask: ask.id(),
                    });
                }
            }
        }
        step.run(machine, answers)?;
        match step.observe(machine)? {
            Standing::Done { evidence } => Ok(evidence),
            standing => Err(Fault::Unproven {
                step: step.id(),
                standing: standing.to_string(),
            }),
        }
    }
}
