//! A request started without waiting for it, for a model whose caller has
//! somewhere else to turn when a device is slow, as a microphone's frame
//! has the processor. OpenVINO's own wait with a timeout doesn't say
//! whether it timed out, so a callback marks the run done.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::{Input, OvRequest, Request, Result, check};

#[link(name = "openvino_c")]
unsafe extern "C" {
    fn ov_infer_request_start_async(request: *mut OvRequest) -> i32;
    fn ov_infer_request_wait(request: *mut OvRequest) -> i32;
    fn ov_infer_request_wait_for(request: *mut OvRequest, timeout: i64) -> i32;
    fn ov_infer_request_set_callback(request: *mut OvRequest, callback: *const Callback) -> i32;
}

/// OpenVINO's `ov_callback_t`: what it calls when a run is done, and with.
#[repr(C)]
struct Callback {
    function: unsafe extern "C" fn(*mut c_void),
    args: *mut c_void,
}

/// Marks the run whose flag `done` is as done.
///
/// # Safety
///
/// `done` must point at the request's flag, alive as long as the request.
unsafe extern "C" fn finished(done: *mut c_void) {
    // SAFETY: as the function's contract says.
    unsafe { (*done.cast::<AtomicBool>()).store(true, Ordering::Release) }
}

/// A request to run a compiled model, started without waiting.
pub struct Started {
    // Declared first, so it's freed before the callback it keeps a
    // pointer to and the flag that marks.
    request: Request,
    _callback: Box<Callback>,
    done: Box<AtomicBool>,
    running: bool,
}

impl Drop for Started {
    fn drop(&mut self) {
        if self.running {
            // SAFETY: the request is alive; a run still going is waited
            // out, so it never marks a flag already freed or reads inputs
            // already gone.
            unsafe { ov_infer_request_wait(self.request.0) };
        }
    }
}

impl Started {
    /// `request`, to be started without waiting.
    ///
    /// # Errors
    ///
    /// When OpenVINO won't take the callback.
    pub fn new(request: Request) -> Result<Self> {
        let done = Box::new(AtomicBool::new(false));
        let callback = Box::new(Callback {
            function: finished,
            args: std::ptr::from_ref::<AtomicBool>(&done).cast_mut().cast(),
        });
        // SAFETY: the request is alive. OpenVINO keeps the pointer to the
        // callback rather than a copy (a callback on the stack crashed the
        // second request made), so it's boxed, as the flag it points at
        // is, and both stay put while the request lives.
        check(
            unsafe { ov_infer_request_set_callback(request.0, &raw const *callback) },
            "setting a request's callback",
        )?;
        Ok(Self {
            request,
            _callback: callback,
            done,
            running: false,
        })
    }

    /// Starts the model on `inputs`, whose values must stay alive and
    /// unchanged until it's done.
    ///
    /// # Errors
    ///
    /// When it's still running, an input is refused, or it won't start.
    pub fn start(&mut self, inputs: &[Input<'_>]) -> Result<()> {
        if self.running {
            return Err("starting a request still running".to_owned());
        }
        self.request.given(inputs)?;
        self.done.store(false, Ordering::Release);
        // SAFETY: every input is set, and the caller keeps their values.
        check(
            unsafe { ov_infer_request_start_async(self.request.0) },
            "starting the model",
        )?;
        self.running = true;
        Ok(())
    }

    /// Whether what was started is done, waiting at most `within`.
    ///
    /// # Errors
    ///
    /// When nothing was started, or the run failed.
    pub fn done(&mut self, within: Duration) -> Result<bool> {
        if !self.running {
            return Err("asking after a request never started".to_owned());
        }
        if !self.done.load(Ordering::Acquire) && !within.is_zero() {
            let millis = i64::try_from(within.as_millis()).unwrap_or(i64::MAX);
            // SAFETY: the request is alive and running. What it says is
            // left unread: it may say a timeout as a failure, and a run
            // that failed says so to the wait below once it's done.
            let _ = unsafe { ov_infer_request_wait_for(self.request.0, millis) };
        }
        if !self.done.load(Ordering::Acquire) {
            return Ok(false);
        }
        self.running = false;
        // SAFETY: the run is done, so this returns at once, with its
        // failure if it failed.
        check(
            unsafe { ov_infer_request_wait(self.request.0) },
            "running the model",
        )?;
        Ok(true)
    }

    /// Reads the f32 output `name` of the run done, as `Request::output`.
    ///
    /// # Errors
    ///
    /// When the output isn't there or isn't a float.
    pub fn output(&self, name: &str, tail: Option<usize>) -> Result<(Vec<i64>, Vec<f32>)> {
        self.request.output(name, tail)
    }
}
