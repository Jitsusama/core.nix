//! Whisper whole on the NPU, through OpenVINO `GenAI`'s static Whisper
//! pipeline and the few calls of its C API a take needs, declared by hand
//! behind an owner that frees what it holds.
//!
//! The decoder as it's exported won't compile for the NPU, since what it
//! has read grows a token at a time (whisper.rs). `GenAI`'s pipelines
//! remake it with a cache of fixed length, so the NPU runs every step, and
//! the CPU, which a loaded machine starves, runs none. Both read four
//! tokens at the first step, too few for Joel's words: the NPU's plugin
//! fixes that length for the default one, while the static one, as fast
//! here, makes it long enough for a prompt given when it's made
//! (openvino-genai-npu-prompt.patch).
//!
//! The C API keeps an exception's reason to itself, which is why a failure
//! names only the call and its status.

use std::ffi::{CString, c_char};
use std::path::Path;
use std::ptr;

use crate::openvino::Priority;

#[repr(C)]
struct Pipeline {
    _private: [u8; 0],
}
#[repr(C)]
struct Config {
    _private: [u8; 0],
}
#[repr(C)]
struct Results {
    _private: [u8; 0],
}

#[link(name = "openvino_genai_c")]
unsafe extern "C" {
    fn ov_genai_whisper_pipeline_create(
        models: *const c_char,
        device: *const c_char,
        arguments: usize,
        pipeline: *mut *mut Pipeline,
        ...
    ) -> i32;
    fn ov_genai_whisper_pipeline_free(pipeline: *mut Pipeline);
    fn ov_genai_whisper_pipeline_get_generation_config(
        pipeline: *const Pipeline,
        config: *mut *mut Config,
    ) -> i32;
    fn ov_genai_whisper_pipeline_set_generation_config(
        pipeline: *mut Pipeline,
        config: *mut Config,
    ) -> i32;
    fn ov_genai_whisper_generation_config_free(config: *mut Config);
    fn ov_genai_whisper_generation_config_set_language(
        config: *mut Config,
        language: *const c_char,
    ) -> i32;
    fn ov_genai_whisper_generation_config_set_task(config: *mut Config, task: *const c_char)
    -> i32;
    fn ov_genai_whisper_generation_config_set_return_timestamps(
        config: *mut Config,
        timestamps: bool,
    ) -> i32;
    fn ov_genai_whisper_pipeline_generate(
        pipeline: *mut Pipeline,
        speech: *const f32,
        samples: usize,
        config: *const Config,
        results: *mut *mut Results,
    ) -> i32;
    fn ov_genai_whisper_decoded_results_free(results: *mut Results);
    fn ov_genai_whisper_decoded_results_get_string(
        results: *const Results,
        text: *mut c_char,
        size: *mut usize,
    ) -> i32;
}

/// What failed, by the call and the status `GenAI` gave.
fn check(status: i32, call: &str) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("GenAI's {call} failed with status {status}"))
    }
}

fn text_of(value: &str, what: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| format!("{what} holds a NUL"))
}

/// Whisper's pipeline, made and set for English transcription without
/// timestamps.
pub struct Whisper {
    pipeline: *mut Pipeline,
}

// The pipeline is used by one thread at a time, the worker that holds it.
unsafe impl Send for Whisper {}

impl Whisper {
    /// The static pipeline for the export in `dir` on `device`, its
    /// compiled models kept in `cache` when one is given, with `prompt`
    /// read before every take, at `priority` among the NPU's jobs.
    ///
    /// # Errors
    ///
    /// When `GenAI` can't make the pipeline there, as where there is no NPU.
    pub fn load(
        dir: &Path,
        device: &str,
        cache: Option<&Path>,
        prompt: &str,
        priority: Priority,
    ) -> Result<Self, String> {
        let dir = text_of(&dir.to_string_lossy(), "the model's path")?;
        let device = text_of(device, "the device")?;
        let pipeline_key = c"STATIC_PIPELINE";
        let static_pipeline = c"YES";
        let cache = cache
            .map(|cache| text_of(&cache.to_string_lossy(), "the cache's path"))
            .transpose()?;
        // GenAI puts the space Whisper starts a word with before the prompt,
        // and an empty one would still put a start before every take.
        let prompt = Some(text_of(prompt.trim(), "the prompt")?).filter(|p| !p.is_empty());
        // The properties are a C variadic list, which can't be built as it
        // runs, so a slot with nothing to say repeats the static pipeline's,
        // which the map keeps once.
        let (cache_key, cache) = match &cache {
            Some(cache) => (c"CACHE_DIR", cache.as_c_str()),
            None => (pipeline_key, static_pipeline),
        };
        let (prompt_key, prompt) = match &prompt {
            Some(prompt) => (c"initial_prompt", prompt.as_c_str()),
            None => (pipeline_key, static_pipeline),
        };
        let mut pipeline = ptr::null_mut();
        // SAFETY: every pointer is to a string that outlives the call, and
        // the count is of the property arguments after the pipeline.
        let status = unsafe {
            ov_genai_whisper_pipeline_create(
                dir.as_ptr(),
                device.as_ptr(),
                8,
                &raw mut pipeline,
                pipeline_key.as_ptr(),
                static_pipeline.as_ptr(),
                c"MODEL_PRIORITY".as_ptr(),
                priority.name().as_ptr(),
                cache_key.as_ptr(),
                cache.as_ptr(),
                prompt_key.as_ptr(),
                prompt.as_ptr(),
            )
        };
        check(status, "Whisper pipeline")?;
        let whisper = Self { pipeline };
        whisper.set_for_transcription()?;
        Ok(whisper)
    }

    fn set_for_transcription(&self) -> Result<(), String> {
        let mut config = ptr::null_mut();
        // SAFETY: the pipeline is live, and the config it gives is freed here.
        unsafe {
            check(
                ov_genai_whisper_pipeline_get_generation_config(self.pipeline, &raw mut config),
                "generation config",
            )?;
            let set = check(
                ov_genai_whisper_generation_config_set_language(config, c"<|en|>".as_ptr()),
                "language",
            )
            .and_then(|()| {
                check(
                    ov_genai_whisper_generation_config_set_task(config, c"transcribe".as_ptr()),
                    "task",
                )
            })
            .and_then(|()| {
                check(
                    ov_genai_whisper_generation_config_set_return_timestamps(config, false),
                    "timestamps",
                )
            })
            .and_then(|()| {
                check(
                    ov_genai_whisper_pipeline_set_generation_config(self.pipeline, config),
                    "generation config",
                )
            });
            ov_genai_whisper_generation_config_free(config);
            set
        }
    }

    /// What's said in 16 kHz audio.
    ///
    /// # Errors
    ///
    /// When the take fails.
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<String, String> {
        let mut results = ptr::null_mut();
        // SAFETY: the samples outlive the call, and the results it gives
        // are read and freed here.
        unsafe {
            check(
                ov_genai_whisper_pipeline_generate(
                    self.pipeline,
                    audio.as_ptr(),
                    audio.len(),
                    ptr::null(),
                    &raw mut results,
                ),
                "take",
            )?;
            let mut size = 0;
            let read = check(
                ov_genai_whisper_decoded_results_get_string(
                    results,
                    ptr::null_mut(),
                    &raw mut size,
                ),
                "text",
            )
            .and_then(|()| {
                let mut text = vec![0u8; size];
                check(
                    ov_genai_whisper_decoded_results_get_string(
                        results,
                        text.as_mut_ptr().cast(),
                        &raw mut size,
                    ),
                    "text",
                )?;
                text.truncate(size.saturating_sub(1));
                Ok(String::from_utf8_lossy(&text).trim().to_owned())
            });
            ov_genai_whisper_decoded_results_free(results);
            read
        }
    }
}

impl Drop for Whisper {
    fn drop(&mut self) {
        // SAFETY: the pipeline was made by load and is freed once.
        unsafe { ov_genai_whisper_pipeline_free(self.pipeline) };
    }
}
