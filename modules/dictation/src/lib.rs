//! Whisper on this machine, shared by the programs that hear speech:
//! dictation (main.rs), which types what Joel says, and call-transcripts
//! (bin/call-transcripts.rs), which writes down what's said on a call. One
//! copy of the models' code keeps the two hearing alike.

pub mod cache;
pub mod genai;
pub mod mel;
pub mod openvino;
pub mod speech;
pub mod takes;
pub mod vocab;
pub mod wav;
pub mod whisper;
pub mod words;
