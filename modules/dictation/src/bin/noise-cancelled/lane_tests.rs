use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::*;

/// What the stand-in NPU does with each frame started on it.
#[derive(Clone, Copy)]
enum Frame {
    /// Back within the bound.
    OnTime,
    /// Back only after this many more looks.
    Late(u32),
    /// Refused.
    Fails,
}

/// What the stand-in NPU was asked, shared with the test.
#[derive(Default)]
struct Asked {
    started: Vec<f32>,
    states: Vec<Vec<f32>>,
}

/// An NPU that doubles a frame and counts its state on by one.
struct StandIn {
    frames: VecDeque<Frame>,
    running: Option<(Frame, f32, Vec<f32>)>,
    asked: Rc<RefCell<Asked>>,
}

impl Run for StandIn {
    fn start(&mut self, frame: &[f32; FRAME], state: &[f32]) -> Result<(), String> {
        let next = self.frames.pop_front().unwrap_or(Frame::OnTime);
        if matches!(next, Frame::Fails) {
            return Err("the stand-in refused it".to_owned());
        }
        let mut asked = self.asked.borrow_mut();
        asked.started.push(frame[0]);
        asked.states.push(state.to_vec());
        self.running = Some((next, frame[0], state.to_vec()));
        Ok(())
    }

    fn done(&mut self, _within: Duration) -> Result<bool, String> {
        let Some((how, _, _)) = self.running.as_mut() else {
            return Err("nothing was started".to_owned());
        };
        match how {
            Frame::OnTime | Frame::Late(0) => Ok(true),
            Frame::Late(looks) => {
                *looks -= 1;
                Ok(false)
            }
            Frame::Fails => Err("the stand-in failed".to_owned()),
        }
    }

    fn taken(&mut self) -> Result<Cleaned, String> {
        let (_, sample, mut state) = self.running.take().ok_or("nothing was done")?;
        state.push(1.0);
        Ok(([sample * 2.0; FRAME], state))
    }
}

/// A lane on a stand-in NPU doing `frames`, and what it's asked.
fn lane(frames: &[Frame]) -> (Lane, Rc<RefCell<Asked>>) {
    let asked = Rc::new(RefCell::new(Asked::default()));
    let run = StandIn {
        frames: frames.iter().copied().collect(),
        running: None,
        asked: Rc::clone(&asked),
    };
    (Lane::on(Some(Box::new(run))), asked)
}

/// The processor behind: trebles a frame and counts its state on by two.
fn behind(frame: &[f32; FRAME], state: &[f32]) -> Cleaned {
    let mut next = state.to_vec();
    next.push(2.0);
    ([frame[0] * 3.0; FRAME], next)
}

/// Cleans frames of 1, 2, 3 and on through `lane`, saying what came out of
/// each and the state left.
fn cleaned(lane: &mut Lane, count: u8) -> (Vec<f32>, Vec<f32>) {
    let mut state = Vec::new();
    let out = (1..=count)
        .map(|at| {
            let frame = [f32::from(at); FRAME];
            lane.clean(&frame, &mut state, |frame, state| Ok(behind(frame, state)))
                .expect("cleans")[0]
        })
        .collect();
    (out, state)
}

#[test]
fn a_frame_back_in_time_is_the_npus() {
    let (mut lane, _) = lane(&[Frame::OnTime, Frame::OnTime]);
    let (out, state) = cleaned(&mut lane, 2);
    assert_eq!(out, [2.0, 4.0], "doubled, as the stand-in NPU cleans");
    assert_eq!(state, [1.0, 1.0], "the state moved on by the NPU twice");
    assert_eq!(lane.late(), 0);
}

#[test]
fn a_late_frame_is_cleaned_behind_from_the_same_state() {
    let (mut lane, asked) = lane(&[Frame::OnTime, Frame::Late(1)]);
    let (out, state) = cleaned(&mut lane, 2);
    assert_eq!(
        out,
        [2.0, 6.0],
        "the second trebled, as the processor cleans"
    );
    assert_eq!(
        state,
        [1.0, 2.0],
        "the state the late frame left is the processor's"
    );
    assert_eq!(
        asked.borrow().states[1],
        [1.0],
        "started from the first frame's state"
    );
    assert_eq!(lane.late(), 1);
}

#[test]
fn frames_stay_behind_while_the_npu_is_busy() {
    let (mut lane, asked) = lane(&[Frame::Late(3), Frame::OnTime]);
    let (out, state) = cleaned(&mut lane, 4);
    assert_eq!(
        out,
        [3.0, 6.0, 9.0, 8.0],
        "behind until the late frame is done, then on the NPU"
    );
    assert_eq!(state, [2.0, 2.0, 2.0, 1.0]);
    assert_eq!(
        asked.borrow().started,
        [1.0, 4.0],
        "nothing started while busy"
    );
    assert_eq!(
        asked.borrow().states[1],
        [2.0, 2.0, 2.0],
        "from where the processor left it"
    );
}

#[test]
fn a_failing_npu_closes_its_lane() {
    let (mut lane, asked) = lane(&[Frame::Fails, Frame::OnTime]);
    let (out, _) = cleaned(&mut lane, 3);
    assert_eq!(out, [3.0, 6.0, 9.0], "every frame cleaned behind");
    assert!(
        asked.borrow().started.is_empty(),
        "the lane closed when the NPU failed, starting nothing after"
    );
}

#[test]
fn no_npu_cleans_every_frame_behind() {
    let mut lane = Lane::on(None);
    let (out, state) = cleaned(&mut lane, 2);
    assert_eq!(out, [3.0, 6.0]);
    assert_eq!(state, [2.0, 2.0]);
}
