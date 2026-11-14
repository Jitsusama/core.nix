//! The service's thread that holds the tidying model (tidy.rs) as
//! Whisper's holds Whisper: compiled as the service starts, loaded and
//! warmed as listening starts, let go after ten minutes unused. It tidies
//! each take it's given by its time, or says it can't, and logs how long
//! each took, never what was said. The service holds a take's 2 s cap
//! itself, so nothing here can keep a take waiting.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Instant;

use dictation::openvino::Priority;
use dictation::tidy::Tidier;

use crate::service::next;

pub enum Job {
    /// Compile the model into the cache, and let it go again.
    Prime,
    /// Load and warm the model, as a take is coming.
    Warm,
    /// A take, by its number, to be tidied by when it's typed as heard.
    Tidy(String, u64, Instant),
}

/// The model that tidies prose before it's typed, and where it runs.
pub struct Tidying {
    /// The model's folder (tidy-model.nix).
    pub model: PathBuf,
    /// The OpenVINO device it runs on.
    pub device: String,
    /// Where what it compiles is kept, a folder of its own.
    pub cache: Option<PathBuf>,
}

/// Starts the thread, which tells `done` each take's tidy by its number,
/// or none where it may not be typed, until `done` says nobody listens.
pub fn start(
    tidying: Tidying,
    done: impl Fn(u64, Option<String>) -> bool + Send + 'static,
) -> Sender<Job> {
    let (jobs, work) = mpsc::channel::<Job>();
    thread::spawn(move || {
        let mut tidier: Option<Tidier> = None;
        let fresh = || {
            Tidier::load(
                &tidying.model,
                &tidying.device,
                tidying.cache.as_deref(),
                Priority::Waited,
            )
        };
        let load = |tidier: &mut Option<Tidier>| -> Result<(), String> {
            if tidier.is_none() {
                let started = Instant::now();
                let mut loaded = fresh()?;
                loaded.warm()?;
                eprintln!(
                    "dictation: the tidier loaded and warmed in {} ms",
                    started.elapsed().as_millis()
                );
                *tidier = Some(loaded);
            }
            Ok(())
        };
        while let Some(job) = next(&work, &mut tidier) {
            match job {
                Job::Prime => {
                    if tidier.is_none() {
                        let started = Instant::now();
                        match fresh() {
                            Ok(_) => eprintln!(
                                "dictation: the tidier is ready in {} ms",
                                started.elapsed().as_millis()
                            ),
                            Err(error) => eprintln!("dictation: {error}"),
                        }
                    }
                }
                Job::Warm => {
                    if let Err(error) = load(&mut tidier) {
                        eprintln!("dictation: {error}");
                    }
                }
                Job::Tidy(take, number, by) => {
                    let started = Instant::now();
                    // A take whose time ran out while another was tidied
                    // is typed already.
                    let written = if started < by {
                        load(&mut tidier).and_then(|()| {
                            tidier
                                .as_mut()
                                .ok_or("the tidier isn't loaded")?
                                .tidy(&take, by)
                        })
                    } else {
                        Err("its time ran out before it was begun".to_owned())
                    };
                    let took = started.elapsed().as_millis();
                    match &written {
                        Ok(_) => eprintln!("dictation: tidied in {took} ms"),
                        Err(why) => eprintln!("dictation: typed as heard after {took} ms: {why}"),
                    }
                    if !done(number, written.ok()) {
                        return;
                    }
                }
            }
        }
    });
    jobs
}
