//! The summarizer's language model, loaded in a process of its own that
//! answers prompts until it's told no more, so the daemon can stop it at
//! once, mid-answer, when a call starts or the laptop sleeps, and so its
//! leaving frees every buffer the GPU held, which no cgroup is charged
//! for. A failure in OpenVINO, as when the GPU runs out of memory
//! (`CL_OUT_OF_RESOURCES`), ends this process, never the daemon.
//!
//! It reads a prompt a line, as JSON, `{"prompt": ..., "tokens": n}`,
//! and answers each with a line, `{"answer": ...}`, until its input ends.
//! It loads only when the memory free covers the model with a margin,
//! and otherwise ends with `SHORT_OF_MEMORY`, for the daemon to try again
//! later.

use std::io::{BufRead, Write};
use std::path::Path;

use dictation::language::LanguageModel;
use dictation::openvino::Priority;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure};

/// What this ends with when there's too little memory to load the model,
/// sysexits' temporary failure, so the daemon knows to wait and ask again.
pub const SHORT_OF_MEMORY: Exit = Exit::TryAgain;

/// The memory that must be free beyond the model's own size, in bytes. A
/// model on the GPU takes about its size on disk in system memory that
/// nothing can swap, and more as it reads; with less than this free
/// beside Gemma 4's 16 GB, the GPU's driver ran out mid-run (2026-11-03).
const MARGIN: u64 = 10 << 30;

/// Answers prompts with the model in `dir`, as the module's comment says.
///
/// # Errors
///
/// `SHORT_OF_MEMORY` when there's too little memory to load the model,
/// and failed when it doesn't load, a prompt isn't JSON or the model
/// fails to answer.
pub fn serve(dir: &Path) -> Result<(), Failure> {
    let mut model = load(dir).map_err(|unloaded| match unloaded {
        Unloaded::Short(why) => Failure::new(SHORT_OF_MEMORY, why),
        Unloaded::Failed(why) => {
            Failure::new(Exit::Failed, format!("the summarizer didn't load: {why}"))
        }
    })?;
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let asked: Value = match serde_json::from_str(&line) {
            Ok(asked) => asked,
            Err(why) => {
                return Err(Failure::new(
                    Exit::Failed,
                    format!("a prompt isn't JSON: {why}"),
                ));
            }
        };
        let prompt = asked["prompt"].as_str().unwrap_or_default();
        let tokens = asked["tokens"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(1);
        match model.answer(prompt, tokens) {
            Ok(answer) => {
                let said = writeln!(out, "{}", json!({ "answer": answer }));
                if said.and_then(|()| out.flush()).is_err() {
                    break;
                }
            }
            Err(why) => {
                return Err(Failure::new(
                    Exit::Failed,
                    format!("the summarizer failed to answer: {why}"),
                ));
            }
        }
    }
    Ok(())
}

/// Why the model wasn't loaded.
pub enum Unloaded {
    /// There's too little memory free for it now.
    Short(String),
    /// Loading it failed.
    Failed(String),
}

/// The model in `dir`, on `SUMMARIZER_DEVICE` (the GPU unless it's set),
/// its compiled model kept in `SUMMARIZER_CACHE`, once there's room.
///
/// # Errors
///
/// When there's too little memory free, or it fails to load.
pub fn load(dir: &Path) -> Result<LanguageModel, Unloaded> {
    let needed = size_of(dir).map_err(Unloaded::Failed)? + MARGIN;
    let meminfo = std::fs::read_to_string("/proc/meminfo")
        .map_err(|why| Unloaded::Failed(format!("reading /proc/meminfo: {why}")))?;
    let free = available(&meminfo).unwrap_or(0);
    if free < needed {
        return Err(Unloaded::Short(format!(
            "the summarizer waits for {} GB free, and {} GB is",
            needed >> 30,
            free >> 30
        )));
    }
    let device = std::env::var("SUMMARIZER_DEVICE").unwrap_or_else(|_| "GPU".to_owned());
    let cache = std::env::var_os("SUMMARIZER_CACHE").map(std::path::PathBuf::from);
    LanguageModel::load(dir, &device, cache.as_deref(), Priority::Background)
        .map_err(Unloaded::Failed)
}

/// The bytes of the files in `dir`, which a linkFarm holds as links.
fn size_of(dir: &Path) -> Result<u64, String> {
    let read = |why: std::io::Error| format!("reading {}: {why}", dir.display());
    let mut size = 0;
    for entry in std::fs::read_dir(dir).map_err(read)? {
        size += std::fs::metadata(entry.map_err(read)?.path())
            .map_err(read)?
            .len();
    }
    Ok(size)
}

/// The memory the kernel says is free for a new program, in bytes.
fn available(meminfo: &str) -> Option<u64> {
    let line = meminfo.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kilobytes: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kilobytes * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_free_is_read_in_bytes() {
        let meminfo = "MemTotal:       65536000 kB\nMemFree:         1000 kB\n\
                       MemAvailable:   27262976 kB\n";
        assert_eq!(available(meminfo), Some(26 << 30));
    }

    #[test]
    fn memory_not_said_is_none() {
        assert_eq!(available("MemTotal: 1 kB\n"), None);
    }

    #[test]
    fn a_models_size_is_its_files_through_their_links() {
        let dir = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        std::fs::write(files.path().join("weights"), [0u8; 300]).unwrap();
        std::os::unix::fs::symlink(files.path().join("weights"), dir.path().join("weights"))
            .unwrap();
        std::fs::write(dir.path().join("config"), [0u8; 12]).unwrap();
        assert_eq!(size_of(dir.path()), Ok(312));
    }
}
