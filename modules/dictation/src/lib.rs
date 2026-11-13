//! Speech models on this machine, shared by the programs that hear
//! speech: dictation (main.rs), which types what Joel says with Whisper,
//! tidied first by a small language model where that's asked for (tidy.rs),
//! and call-transcripts (bin/call-transcripts), which writes down what's
//! said on a call with Parakeet (parakeet.rs) and sums it up with a
//! language model (language.rs). The two share OpenVINO's calls and the
//! cache of what's compiled. screen-text
//! (bin/screen-text), which reads the words in screenshots, and meaning
//! (bin/meaning), which finds the launcher's things and pictures by what
//! they mean, share its OpenVINO calls, its reading of pictures
//! (picture.rs) and its hearing of the folders they're kept in
//! (watch.rs), and they and call-transcripts serve the shell alike
//! (daemon.rs). Dictation and noise-cancelled find the microphone Joel
//! picked, and its noise-cancelled twin, alike (microphone.rs).

pub mod cache;
pub mod daemon;
pub mod genai;
pub mod graph;
pub mod language;
pub mod mel;
pub mod microphone;
pub mod openvino;
pub mod parakeet;
pub mod picture;
pub mod speech;
pub mod takes;
pub mod tidy;
pub mod transcript;
pub mod vocab;
pub mod watch;
pub mod wav;
pub mod whisper;
pub mod words;

/// Whether the machine has an NPU, which shows as an accel device. Only
/// its compile is slow enough to do ahead of need; without one a model
/// compiles for the CPU as quickly as it loads, so loading it early would
/// only take memory for nothing.
#[must_use]
pub fn npu() -> bool {
    std::fs::read_dir("/dev/accel").is_ok_and(|mut entries| entries.next().is_some())
}
