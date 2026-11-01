//! The few calls of OpenVINO's C API that running Whisper takes, declared
//! by hand, behind owners that free what they hold. The non-variadic forms
//! are used, the ones OpenVINO offers for callers other than C.

use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::ptr;

/// Where a job stands among the NPU's others, as OpenVINO's
/// `MODEL_PRIORITY`. At the default, a short job can wait behind a long
/// one: with Whisper busy, a camera's frames waited up to 36 ms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    /// What Joel waits on, such as dictation's take.
    Waited,
    /// What nobody waits on, such as a call's transcript.
    Background,
}

impl Priority {
    /// The priority's name to OpenVINO. Live work on a call would say HIGH.
    pub fn name(self) -> &'static CStr {
        match self {
            Self::Waited => c"MEDIUM",
            Self::Background => c"LOW",
        }
    }
}

#[repr(C)]
struct OvCore {
    _private: [u8; 0],
}
#[repr(C)]
struct OvModel {
    _private: [u8; 0],
}
#[repr(C)]
struct OvCompiled {
    _private: [u8; 0],
}
#[repr(C)]
struct OvRequest {
    _private: [u8; 0],
}
#[repr(C)]
struct OvTensor {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Dimension {
    min: i64,
    max: i64,
}

#[repr(C)]
struct PartialShape {
    rank: Dimension,
    dims: *mut Dimension,
}

#[repr(C)]
struct Shape {
    rank: i64,
    dims: *mut i64,
}

#[repr(C)]
struct Property {
    key: *const c_char,
    value: *const c_void,
}

// ov_element_type_e, in its order: DYNAMIC, BOOLEAN, BF16, F16, F32, F64,
// I4, I8, I16, I32, I64.
const F16: i32 = 3;
const F32: i32 = 4;
const I32: i32 = 9;
const I64: i32 = 10;

#[link(name = "openvino_c")]
unsafe extern "C" {
    fn ov_get_last_err_msg() -> *const c_char;
    fn ov_core_create(core: *mut *mut OvCore) -> i32;
    fn ov_core_free(core: *mut OvCore);
    fn ov_core_set_properties(
        core: *const OvCore,
        device: *const c_char,
        count: usize,
        properties: *const Property,
    ) -> i32;
    fn ov_core_read_model(
        core: *const OvCore,
        xml: *const c_char,
        bin: *const c_char,
        model: *mut *mut OvModel,
    ) -> i32;
    fn ov_model_free(model: *mut OvModel);
    fn ov_partial_shape_create_static(rank: i64, dims: *const i64, shape: *mut PartialShape)
    -> i32;
    fn ov_partial_shape_free(shape: *mut PartialShape);
    fn ov_model_reshape_single_input(model: *const OvModel, shape: PartialShape) -> i32;
    fn ov_core_compile_model_props(
        core: *const OvCore,
        model: *const OvModel,
        device: *const c_char,
        count: usize,
        properties: *const Property,
        compiled: *mut *mut OvCompiled,
    ) -> i32;
    fn ov_compiled_model_free(compiled: *mut OvCompiled);
    fn ov_compiled_model_create_infer_request(
        compiled: *const OvCompiled,
        request: *mut *mut OvRequest,
    ) -> i32;
    fn ov_infer_request_free(request: *mut OvRequest);
    fn ov_infer_request_set_tensor(
        request: *mut OvRequest,
        name: *const c_char,
        tensor: *const OvTensor,
    ) -> i32;
    fn ov_infer_request_get_tensor(
        request: *const OvRequest,
        name: *const c_char,
        tensor: *mut *mut OvTensor,
    ) -> i32;
    fn ov_infer_request_infer(request: *mut OvRequest) -> i32;
    fn ov_shape_create(rank: i64, dims: *const i64, shape: *mut Shape) -> i32;
    fn ov_shape_free(shape: *mut Shape) -> i32;
    fn ov_tensor_create_from_host_ptr(
        kind: i32,
        shape: Shape,
        host: *mut c_void,
        tensor: *mut *mut OvTensor,
    ) -> i32;
    fn ov_tensor_free(tensor: *mut OvTensor);
    fn ov_tensor_data(tensor: *const OvTensor, data: *mut *mut c_void) -> i32;
    fn ov_tensor_get_shape(tensor: *const OvTensor, shape: *mut Shape) -> i32;
    fn ov_tensor_get_element_type(tensor: *const OvTensor, kind: *mut i32) -> i32;
}

/// What a call failed with: what was asked, and OpenVINO's own words.
pub type Result<T> = std::result::Result<T, String>;

fn check(status: i32, what: &str) -> Result<()> {
    if status == 0 {
        return Ok(());
    }
    // SAFETY: OpenVINO returns its last message as a C string it keeps,
    // or null.
    let message = unsafe {
        let text = ov_get_last_err_msg();
        if text.is_null() {
            String::new()
        } else {
            CStr::from_ptr(text).to_string_lossy().into_owned()
        }
    };
    Err(format!("{what} failed ({status}): {}", message.trim()))
}

fn c_string(text: &str) -> Result<CString> {
    CString::new(text).map_err(|_| format!("{text:?} holds a NUL"))
}

fn c_path(path: &Path) -> Result<CString> {
    c_string(
        path.to_str()
            .ok_or_else(|| format!("{} isn't UTF-8", path.display()))?,
    )
}

/// OpenVINO itself, which reads and compiles models.
pub struct Core(*mut OvCore);

impl Drop for Core {
    fn drop(&mut self) {
        // SAFETY: the pointer came from ov_core_create and is freed once.
        unsafe { ov_core_free(self.0) }
    }
}

impl Core {
    /// OpenVINO, keeping what it compiles in `cache` when given one, so
    /// the next compile of the same model is a read.
    ///
    /// # Errors
    ///
    /// When OpenVINO won't start or refuses the cache.
    pub fn new(cache: Option<&Path>) -> Result<Self> {
        let mut core = ptr::null_mut();
        // SAFETY: the out pointer is valid for the call.
        check(
            unsafe { ov_core_create(&raw mut core) },
            "starting OpenVINO",
        )?;
        let core = Self(core);
        if let Some(cache) = cache {
            let key = c_string("CACHE_DIR")?;
            let value = c_path(cache)?;
            let property = Property {
                key: key.as_ptr(),
                value: value.as_ptr().cast(),
            };
            // SAFETY: the property's strings outlive the call, and a null
            // device sets it for every device.
            check(
                unsafe { ov_core_set_properties(core.0, ptr::null(), 1, &raw const property) },
                "setting the cache",
            )?;
        }
        Ok(core)
    }

    /// The model in `xml`, its weights in the .bin beside it; or an ONNX
    /// model, which holds its weights itself.
    ///
    /// # Errors
    ///
    /// When either file can't be read as a model.
    pub fn read(&self, xml: &Path) -> Result<Model> {
        let xml_name = c_path(xml)?;
        let bin_name = if xml.extension().is_some_and(|kind| kind == "onnx") {
            None
        } else {
            Some(c_path(&xml.with_extension("bin"))?)
        };
        let bin_name = bin_name.as_deref().map_or(ptr::null(), CStr::as_ptr);
        let mut model = ptr::null_mut();
        // SAFETY: the names outlive the call, a null weights path asks for
        // none, and the out pointer is valid.
        check(
            unsafe { ov_core_read_model(self.0, xml_name.as_ptr(), bin_name, &raw mut model) },
            &format!("reading {}", xml.display()),
        )?;
        Ok(Model(model))
    }

    /// `model` compiled for `device`, such as NPU or CPU, saying its
    /// `priority` on the NPU, the one device it's for.
    ///
    /// # Errors
    ///
    /// When the device is missing or can't run the model.
    pub fn compile(&self, model: &Model, device: &str, priority: Priority) -> Result<Compiled> {
        let name = c_string(device)?;
        let priority = [Property {
            key: c"MODEL_PRIORITY".as_ptr(),
            value: priority.name().as_ptr().cast(),
        }];
        let properties: &[Property] = if device == "NPU" { &priority } else { &[] };
        let mut compiled = ptr::null_mut();
        // SAFETY: the model is alive, the properties' strings are static and
        // counted, and the out pointer is valid.
        check(
            unsafe {
                ov_core_compile_model_props(
                    self.0,
                    model.0,
                    name.as_ptr(),
                    properties.len(),
                    properties.as_ptr(),
                    &raw mut compiled,
                )
            },
            &format!("compiling for {device}"),
        )?;
        Ok(Compiled(compiled))
    }

    /// `model` compiled for the CPU to run on one thread, for a model run
    /// often on little at a time, as a microphone's every frame is, where
    /// spreading each run across every core costs more than it saves.
    ///
    /// # Errors
    ///
    /// When the CPU can't run the model.
    pub fn compile_on_one_thread(&self, model: &Model) -> Result<Compiled> {
        let properties = [Property {
            key: c"INFERENCE_NUM_THREADS".as_ptr(),
            value: c"1".as_ptr().cast(),
        }];
        let mut compiled = ptr::null_mut();
        // SAFETY: the model is alive, the properties' strings are static and
        // counted, and the out pointer is valid.
        check(
            unsafe {
                ov_core_compile_model_props(
                    self.0,
                    model.0,
                    c"CPU".as_ptr(),
                    properties.len(),
                    properties.as_ptr(),
                    &raw mut compiled,
                )
            },
            "compiling for the CPU on one thread",
        )?;
        Ok(Compiled(compiled))
    }
}

/// A model as read, before it's compiled.
pub struct Model(*mut OvModel);

impl Drop for Model {
    fn drop(&mut self) {
        // SAFETY: the pointer came from ov_core_read_model and is freed
        // once.
        unsafe { ov_model_free(self.0) }
    }
}

impl Model {
    /// Fixes the shape of the model's one input, as a device that takes
    /// only fixed shapes, such as the NPU, needs.
    ///
    /// # Errors
    ///
    /// When the model has more than one input or refuses the shape.
    pub fn fix_input(&mut self, dims: &[i64]) -> Result<()> {
        let rank = i64::try_from(dims.len()).map_err(|_| "too many dimensions".to_owned())?;
        let mut shape = PartialShape {
            rank: Dimension { min: 0, max: 0 },
            dims: ptr::null_mut(),
        };
        // SAFETY: dims holds rank values and the out struct is valid.
        check(
            unsafe { ov_partial_shape_create_static(rank, dims.as_ptr(), &raw mut shape) },
            "making the input's shape",
        )?;
        // SAFETY: the shape was just made and is read, not kept, by the
        // reshape; it's freed once after.
        let status = unsafe {
            let status = ov_model_reshape_single_input(
                self.0,
                PartialShape {
                    rank: shape.rank,
                    dims: shape.dims,
                },
            );
            ov_partial_shape_free(&raw mut shape);
            status
        };
        check(status, "fixing the input's shape")
    }
}

/// A model compiled for a device.
pub struct Compiled(*mut OvCompiled);

impl Drop for Compiled {
    fn drop(&mut self) {
        // SAFETY: the pointer came from ov_core_compile_model_props and is
        // freed once.
        unsafe { ov_compiled_model_free(self.0) }
    }
}

impl Compiled {
    /// Somewhere to run it; a stateful model starts each request afresh.
    ///
    /// # Errors
    ///
    /// When OpenVINO can't make one.
    pub fn request(&self) -> Result<Request> {
        let mut request = ptr::null_mut();
        // SAFETY: the out pointer is valid for the call.
        check(
            unsafe { ov_compiled_model_create_infer_request(self.0, &raw mut request) },
            "making a request",
        )?;
        Ok(Request(request))
    }
}

/// One input to a run: its name, shape and values.
pub enum Input<'a> {
    F32(&'a str, &'a [i64], &'a [f32]),
    I64(&'a str, &'a [i64], &'a [i64]),
    I32(&'a str, &'a [i64], &'a [i32]),
}

/// A request to run a compiled model.
pub struct Request(*mut OvRequest);

impl Drop for Request {
    fn drop(&mut self) {
        // SAFETY: the pointer came from ov_compiled_model_create_infer_request
        // and is freed once.
        unsafe { ov_infer_request_free(self.0) }
    }
}

impl Request {
    /// Runs the model on `inputs`, then reads the f32 output `output`,
    /// keeping only its last `tail` values when given, and returns its
    /// shape and values.
    ///
    /// # Errors
    ///
    /// When an input is refused, the run fails or the output isn't there.
    pub fn run(
        &mut self,
        inputs: &[Input<'_>],
        output: &str,
        tail: Option<usize>,
    ) -> Result<(Vec<i64>, Vec<f32>)> {
        for input in inputs {
            let (name, dims, kind, data, len) = match input {
                Input::F32(name, dims, values) => (
                    *name,
                    *dims,
                    F32,
                    values.as_ptr().cast::<c_void>(),
                    values.len(),
                ),
                Input::I64(name, dims, values) => (
                    *name,
                    *dims,
                    I64,
                    values.as_ptr().cast::<c_void>(),
                    values.len(),
                ),
                Input::I32(name, dims, values) => (
                    *name,
                    *dims,
                    I32,
                    values.as_ptr().cast::<c_void>(),
                    values.len(),
                ),
            };
            let count: i64 = dims.iter().product();
            if usize::try_from(count).ok() != Some(len) {
                return Err(format!("{name} has {len} values for a shape of {count}"));
            }
            self.set(name, dims, kind, data)?;
        }
        // SAFETY: every input is set and their values outlive the run.
        check(
            unsafe { ov_infer_request_infer(self.0) },
            "running the model",
        )?;
        self.read(output, tail)
    }

    fn set(&mut self, name: &str, dims: &[i64], kind: i32, data: *const c_void) -> Result<()> {
        let name = c_string(name)?;
        let rank = i64::try_from(dims.len()).map_err(|_| "too many dimensions".to_owned())?;
        let mut shape = Shape {
            rank: 0,
            dims: ptr::null_mut(),
        };
        // SAFETY: dims holds rank values and the out struct is valid.
        check(
            unsafe { ov_shape_create(rank, dims.as_ptr(), &raw mut shape) },
            "making a shape",
        )?;
        let mut tensor = ptr::null_mut();
        // SAFETY: the tensor wraps data, which the caller keeps alive and
        // unchanged until the run is over; OpenVINO only reads an input.
        // The shape is copied into the tensor and freed once after.
        let status = unsafe {
            let status = ov_tensor_create_from_host_ptr(
                kind,
                Shape {
                    rank: shape.rank,
                    dims: shape.dims,
                },
                data.cast_mut(),
                &raw mut tensor,
            );
            ov_shape_free(&raw mut shape);
            status
        };
        check(status, "wrapping an input")?;
        // SAFETY: the request takes its own reference to the tensor, so
        // this handle is freed once, straight after.
        let status = unsafe {
            let status = ov_infer_request_set_tensor(self.0, name.as_ptr(), tensor);
            ov_tensor_free(tensor);
            status
        };
        check(status, &format!("setting {}", name.to_string_lossy()))
    }

    /// Reads the f32 output `name` of the last run, as `run` reads the one
    /// it's given, for a model with more than one output wanted.
    ///
    /// # Errors
    ///
    /// When the output isn't there or isn't a float.
    pub fn output(&self, name: &str, tail: Option<usize>) -> Result<(Vec<i64>, Vec<f32>)> {
        self.read(name, tail)
    }

    fn read(&self, name: &str, tail: Option<usize>) -> Result<(Vec<i64>, Vec<f32>)> {
        let c_name = c_string(name)?;
        let mut tensor = ptr::null_mut();
        // SAFETY: the out pointer is valid for the call.
        check(
            unsafe { ov_infer_request_get_tensor(self.0, c_name.as_ptr(), &raw mut tensor) },
            &format!("reading {name}"),
        )?;
        // SAFETY: the tensor is alive until it's freed at the end, its shape
        // is copied out before the shape is freed, and its data holds as
        // many values of its element type as its shape says.
        unsafe {
            let mut shape = Shape {
                rank: 0,
                dims: ptr::null_mut(),
            };
            let mut kind = 0;
            let mut data = ptr::null_mut();
            let read = check(
                ov_tensor_get_shape(tensor, &raw mut shape),
                "reading a shape",
            )
            .and_then(|()| {
                let rank = usize::try_from(shape.rank).map_err(|_| "a negative rank".to_owned())?;
                let dims = std::slice::from_raw_parts(shape.dims, rank).to_vec();
                ov_shape_free(&raw mut shape);
                Ok(dims)
            })
            .and_then(|dims| {
                check(
                    ov_tensor_get_element_type(tensor, &raw mut kind),
                    "reading a type",
                )?;
                check(ov_tensor_data(tensor, &raw mut data), "reading values")?;
                let count =
                    usize::try_from(dims.iter().product::<i64>()).map_err(|_| "a negative size")?;
                let skip = tail.map_or(0, |tail| count.saturating_sub(tail));
                let values = match kind {
                    F32 => std::slice::from_raw_parts(data.cast::<f32>(), count)[skip..].to_vec(),
                    F16 => std::slice::from_raw_parts(data.cast::<u16>(), count)[skip..]
                        .iter()
                        .map(|&h| half(h))
                        .collect(),
                    other => return Err(format!("{name} is of type {other}, not a float")),
                };
                Ok((dims, values))
            });
            ov_tensor_free(tensor);
            read
        }
    }
}

/// An IEEE half as a single.
fn half(h: u16) -> f32 {
    let sign = u32::from(h >> 15) << 31;
    let exponent = u32::from((h >> 10) & 0x1f);
    let fraction = u32::from(h & 0x3ff);
    let bits = match (exponent, fraction) {
        (0, 0) => sign,
        (0, _) => {
            // A subnormal half is a normal single: shift until the leading
            // one falls off.
            let mut e = 127 - 15 + 1;
            let mut f = fraction;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            sign | (e << 23) | ((f & 0x3ff) << 13)
        }
        (0x1f, _) => sign | 0x7f80_0000 | (fraction << 13),
        _ => sign | ((exponent + 127 - 15) << 23) | (fraction << 13),
    };
    f32::from_bits(bits)
}

#[cfg(test)]
mod tests {
    use super::half;

    #[test]
    fn halves_read_as_singles() {
        assert!((half(0x3c00) - 1.0).abs() < f32::EPSILON);
        assert!((half(0xc000) + 2.0).abs() < f32::EPSILON);
        assert!((half(0x3555) - 0.333_251_95).abs() < 1e-7);
        assert!(half(0x0000).abs() < f32::EPSILON);
        assert!((half(0x0001) - 5.960_464_5e-8).abs() < 1e-12);
        assert!(half(0x7c00).is_infinite());
    }
}
