//! Whisper on this machine, shared by the programs that hear speech:
//! dictation (main.rs), which types what Joel says, and call-transcripts
//! (bin/call-transcripts), which writes down what's said on a call and
//! sums it up with a language model (language.rs). One copy of the
//! models' code keeps the two hearing alike.

pub mod cache;
pub mod genai;
pub mod graph;
pub mod language;
pub mod mel;
pub mod openvino;
pub mod speech;
pub mod takes;
pub mod transcript;
pub mod vocab;
pub mod wav;
pub mod whisper;
pub mod words;
