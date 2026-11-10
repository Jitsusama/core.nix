//! A language model that writes an answer to a prompt, through OpenVINO
//! `GenAI`'s vision-language pipeline given text alone, and the few calls
//! of its C API that needs, declared by hand behind an owner that frees
//! what it holds.
//!
//! Newer models, Gemma 4 among them, are exported as vision-language
//! models, which `GenAI`'s LLM pipeline can't read; its VLM pipeline reads
//! them, and answers a prompt with no picture as an LLM would. The prompt
//! is put in the model's own chat template, which its tokenizer carries,
//! so it reads as a question asked of it.

use std::ffi::{CStr, c_char};
use std::path::Path;
use std::ptr;

use crate::cache;
use crate::genai::{check, text_of};
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
#[repr(C)]
struct Tensor {
    _private: [u8; 0],
}

#[link(name = "openvino_genai_c")]
unsafe extern "C" {
    fn ov_genai_vlm_pipeline_create(
        models: *const c_char,
        device: *const c_char,
        arguments: usize,
        pipeline: *mut *mut Pipeline,
        ...
    ) -> i32;
    fn ov_genai_vlm_pipeline_free(pipeline: *mut Pipeline);
    fn ov_genai_vlm_pipeline_get_generation_config(
        pipeline: *const Pipeline,
        config: *mut *mut Config,
    ) -> i32;
    fn ov_genai_generation_config_free(config: *mut Config);
    fn ov_genai_generation_config_set_max_new_tokens(config: *mut Config, tokens: usize) -> i32;
    fn ov_genai_generation_config_set_do_sample(config: *mut Config, sample: bool) -> i32;
    fn ov_genai_vlm_pipeline_generate(
        pipeline: *mut Pipeline,
        inputs: *const c_char,
        pictures: *const *const Tensor,
        count: usize,
        config: *const Config,
        streamer: *const u8,
        results: *mut *mut Results,
    ) -> i32;
    fn ov_genai_vlm_decoded_results_free(results: *mut Results);
    fn ov_genai_vlm_decoded_results_get_string(
        results: *const Results,
        text: *mut c_char,
        size: *mut usize,
    ) -> i32;
}

/// The model's pipeline, and the settings each answer is written with.
pub struct LanguageModel {
    pipeline: *mut Pipeline,
    config: *mut Config,
}

// The pipeline is used by one thread at a time, the one that holds it.
unsafe impl Send for LanguageModel {}

impl LanguageModel {
    /// The pipeline for the export in `dir` on `device`, at `priority`
    /// among the device's jobs, its compiled model kept in a folder of
    /// `cache` for this model, device and build, the others there let go.
    ///
    /// # Errors
    ///
    /// When `GenAI` can't make the pipeline there, as when the GPU has
    /// too little memory for it.
    pub fn load(
        dir: &Path,
        device: &str,
        cache: Option<&Path>,
        priority: Priority,
    ) -> Result<Self, String> {
        let parts = [dir.as_os_str().as_encoded_bytes(), device.as_bytes()];
        let kept = cache.map(|cache| cache::kept(cache, &parts));
        let model = Self::load_in(dir, device, kept.as_deref(), priority)?;
        if let (Some(cache), Some(kept)) = (cache, &kept) {
            cache::forget_others(cache, kept);
        }
        Ok(model)
    }

    fn load_in(
        dir: &Path,
        device: &str,
        cache: Option<&Path>,
        priority: Priority,
    ) -> Result<Self, String> {
        let dir = text_of(&dir.to_string_lossy(), "the model's path")?;
        // The processor's plugin refuses a priority, so it's given only
        // where it's known.
        let knows_priority = device == "NPU" || device.starts_with("GPU");
        let device = text_of(device, "the device")?;
        let cache = cache
            .map(|cache| text_of(&cache.to_string_lossy(), "the cache's path"))
            .transpose()?;
        let mut properties: Vec<(&CStr, &CStr)> = Vec::new();
        if knows_priority {
            properties.push((c"MODEL_PRIORITY", priority.name()));
        }
        if let Some(cache) = &cache {
            properties.push((c"CACHE_DIR", cache.as_c_str()));
        }
        // The properties are a C variadic list, which can't be built as it
        // runs, so every slot is passed and the count says how many GenAI
        // reads; the rest are null and never read.
        let slot = |n: usize| -> [*const c_char; 2] {
            properties.get(n).map_or([ptr::null(); 2], |(key, value)| {
                [key.as_ptr(), value.as_ptr()]
            })
        };
        let [key0, value0] = slot(0);
        let [key1, value1] = slot(1);
        let mut pipeline = ptr::null_mut();
        // SAFETY: every pointer read is to a string that outlives the call,
        // and the count is of the property arguments that are set.
        let status = unsafe {
            ov_genai_vlm_pipeline_create(
                dir.as_ptr(),
                device.as_ptr(),
                2 * properties.len(),
                &raw mut pipeline,
                key0,
                value0,
                key1,
                value1,
            )
        };
        check(status, "VLM pipeline")?;
        let mut config = ptr::null_mut();
        // SAFETY: the pipeline was just made, and the config it gives is
        // owned here and freed with the model.
        let set = unsafe {
            check(
                ov_genai_vlm_pipeline_get_generation_config(pipeline, &raw mut config),
                "generation config",
            )
            // The same prompt gives the same answer: greedy, never sampled.
            .and_then(|()| {
                check(
                    ov_genai_generation_config_set_do_sample(config, false),
                    "greedy decoding",
                )
            })
        };
        let model = Self { pipeline, config };
        set.map(|()| model)
    }

    /// The model's answer to `prompt`, at most `tokens` long.
    ///
    /// # Errors
    ///
    /// When the answer fails, as when the GPU runs out of memory for it.
    pub fn answer(&mut self, prompt: &str, tokens: usize) -> Result<String, String> {
        let prompt = text_of(prompt, "the prompt")?;
        let mut results = ptr::null_mut();
        // SAFETY: the prompt and config outlive the call, no picture is
        // read, and the results it gives are read and freed here.
        unsafe {
            check(
                ov_genai_generation_config_set_max_new_tokens(self.config, tokens),
                "answer's length",
            )?;
            check(
                ov_genai_vlm_pipeline_generate(
                    self.pipeline,
                    prompt.as_ptr(),
                    ptr::null(),
                    0,
                    self.config,
                    ptr::null(),
                    &raw mut results,
                ),
                "answer",
            )?;
            let mut size = 0;
            let read = check(
                ov_genai_vlm_decoded_results_get_string(results, ptr::null_mut(), &raw mut size),
                "text",
            )
            .and_then(|()| {
                let mut text = vec![0u8; size];
                check(
                    ov_genai_vlm_decoded_results_get_string(
                        results,
                        text.as_mut_ptr().cast(),
                        &raw mut size,
                    ),
                    "text",
                )?;
                text.truncate(size.saturating_sub(1));
                Ok(String::from_utf8_lossy(&text).trim().to_owned())
            });
            ov_genai_vlm_decoded_results_free(results);
            read
        }
    }
}

impl Drop for LanguageModel {
    fn drop(&mut self) {
        // SAFETY: both were made by load and are freed once; a config
        // left null by a failed load isn't freed.
        unsafe {
            if !self.config.is_null() {
                ov_genai_generation_config_free(self.config);
            }
            ov_genai_vlm_pipeline_free(self.pipeline);
        }
    }
}
