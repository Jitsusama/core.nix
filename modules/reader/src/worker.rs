//! A thread of its own that takes jobs in turn, and that is stopped and
//! waited for when it's let go. The loader is one: it makes textures on a
//! context of the driver's, and a thread left making one as the reader
//! exits died in the driver, whose own teardown at exit freed what it used.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

/// Where jobs go to the thread. Letting it go stops the thread once the job
/// in hand is done, skips the rest, and returns once the thread has ended.
pub struct Worker<J> {
    jobs: Option<Sender<J>>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// The jobs as the thread takes them: each in turn until the worker is let
/// go, and none after, however many still wait.
pub struct Jobs<J> {
    queue: Receiver<J>,
    stopping: Arc<AtomicBool>,
}

impl<J> Iterator for Jobs<J> {
    type Item = J;

    fn next(&mut self) -> Option<J> {
        let job = self.queue.recv().ok()?;
        (!self.stopping.load(Ordering::Acquire)).then_some(job)
    }
}

impl<J: Send + 'static> Worker<J> {
    /// Starts a thread named `name` that runs `work` over the jobs sent.
    pub fn start(
        name: &str,
        work: impl FnOnce(Jobs<J>) + Send + 'static,
    ) -> std::io::Result<Worker<J>> {
        let (jobs, queue) = std::sync::mpsc::channel();
        let stopping = Arc::new(AtomicBool::new(false));
        let taken = Jobs {
            queue,
            stopping: Arc::clone(&stopping),
        };
        let thread = std::thread::Builder::new()
            .name(name.into())
            .spawn(move || work(taken))?;
        Ok(Worker {
            jobs: Some(jobs),
            stopping,
            thread: Some(thread),
        })
    }
}

impl<J> Worker<J> {
    /// Sends a job. False once the thread has ended.
    pub fn send(&self, job: J) -> bool {
        self.jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(job).is_ok())
    }
}

impl<J> Drop for Worker<J> {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        // The queue closed wakes a thread waiting on it.
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Worker;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn a_worker_let_go_has_ended_its_job_and_its_thread_and_skips_the_rest() {
        let said = Arc::new(Mutex::new(Vec::new()));
        let heard = Arc::clone(&said);
        let worker = Worker::start("worker-test", move |jobs| {
            for job in jobs {
                heard.lock().expect("the log").push(format!("began {job}"));
                std::thread::sleep(Duration::from_millis(50));
                heard.lock().expect("the log").push(format!("ended {job}"));
            }
            heard.lock().expect("the log").push("let go".to_string());
        })
        .expect("the thread");
        for job in 0..10 {
            assert!(worker.send(job), "the thread took no job {job}");
        }
        // Let go while the first job is in hand, as the reader exits while
        // the loader makes a texture.
        while said.lock().expect("the log").is_empty() {
            std::thread::sleep(Duration::from_millis(1));
        }
        drop(worker);
        let said = said.lock().expect("the log").clone();
        assert_eq!(
            said.last().map(String::as_str),
            Some("let go"),
            "the worker returned while its thread still ran: {said:?}"
        );
        let began = said.iter().filter(|line| line.starts_with("began")).count();
        let ended = said.iter().filter(|line| line.starts_with("ended")).count();
        assert_eq!(began, ended, "a job was left half done: {said:?}");
        assert!(
            began < 10,
            "every job waiting was done before it stopped: {said:?}"
        );
    }
}
