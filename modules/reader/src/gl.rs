//! The compositor's GL: an EGL context on the window's surface, and one
//! textured quad drawn per band in view. A frame is a clear and a handful of
//! quads, so it costs the same however much the page holds.

use std::ffi::c_void;

use glow::HasContext;
use khronos_egl as egl;
use smithay_client_toolkit::reexports::calloop;
use wayland_client::Proxy;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::worker::{Jobs, Worker};

const VERTEX: &str = r"#version 300 es
uniform vec4 rect;
out highp vec2 uv;
void main() {
    vec2 corner = vec2(float(gl_VertexID & 1), float(gl_VertexID >> 1));
    uv = corner;
    gl_Position = vec4(mix(rect.xy, rect.zw, corner), 0.0, 1.0);
}
";

const FRAGMENT: &str = r"#version 300 es
precision mediump float;
uniform sampler2D band;
in highp vec2 uv;
out vec4 colour;
void main() {
    colour = texture(band, uv);
}
";

#[expect(
    clippy::struct_field_names,
    reason = "glow's context, as glow names it"
)]
pub struct Gl {
    egl: egl::Instance<egl::Static>,
    display: egl::Display,
    surface: egl::Surface,
    config: egl::Config,
    context: egl::Context,
    window: wayland_egl::WlEglSurface,
    pub gl: glow::Context,
    program: glow::Program,
    rect: glow::UniformLocation,
    _vertices: glow::VertexArray,
    /// What the window shows, in device pixels: the buffer's top left.
    pub width: i32,
    pub height: i32,
    /// The buffer's own size. It grows in steps and never shrinks, so a
    /// resize seldom has the driver find new buffers, which cost a frame or
    /// more; the window shows only its top left, cut by the viewport.
    /// Without a viewport to cut it, it's the window's size.
    held: (i32, i32),
    cut: bool,
}

/// What the loader shares the main context with, made on this thread and
/// used on the loader's.
struct Share {
    display: egl::Display,
    context: egl::Context,
}

// SAFETY: EGL handles name objects of the display, not of a thread; the
// loader's context is made current on its thread alone.
unsafe impl Send for Share {}

/// A GL fence, which names a sync object of the share group.
pub struct Fence(glow::Fence);

// SAFETY: a sync object belongs to the contexts sharing it, not to a thread.
unsafe impl Send for Fence {}

struct Job<T> {
    tag: T,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// A texture the loader made, with what it was made for, and the fence
/// that passes once the GPU holds it whole.
pub struct Made<T> {
    pub tag: T,
    texture: Texture,
    fence: Fence,
}

/// Where bands go to be made into textures off the frame's path. Let go,
/// it stops the loader and waits for it to let go of its context, so it
/// must go before the `Gl` it shares.
pub struct Loader<T> {
    jobs: Worker<Job<T>>,
}

impl<T> Loader<T> {
    /// Sends a band's pixels to be made into a texture, four bytes a pixel.
    /// False once the loader has stopped.
    pub fn load(&self, tag: T, width: u32, height: u32, pixels: Vec<u8>) -> bool {
        self.jobs.send(Job {
            tag,
            width,
            height,
            pixels,
        })
    }
}

/// Makes a texture of a band's pixels on the current context, and a fence
/// after it, flushed so the GPU starts on it now.
#[expect(
    clippy::cast_possible_wrap,
    reason = "GL takes sizes and places as i32 and f32; a texture or window is a few thousand pixels at most"
)]
unsafe fn make<T>(
    gl: &glow::Context,
    job: &Job<T>,
    warm: bool,
) -> Result<(glow::Texture, glow::Fence), String> {
    unsafe {
        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        for (name, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, name, value as i32);
        }
        gl.tex_storage_2d(
            glow::TEXTURE_2D,
            1,
            glow::RGBA8,
            job.width as i32,
            job.height as i32,
        );
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            job.width as i32,
            job.height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(&job.pixels)),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        // Reads a pixel back, so whatever the driver does the first time a
        // texture is read (a resolve of its compression, a wait) is done
        // here rather than in the first frame that draws it.
        if warm {
            let framebuffer = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::READ_FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            let mut pixel = [0u8; 4];
            gl.read_pixels(
                0,
                0,
                1,
                1,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixel)),
            );
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
            gl.delete_framebuffer(framebuffer);
        }
        let fence = gl.fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0)?;
        gl.flush();
        // Waited for here, off the frame's path, so a texture reaches the
        // loop made. One sent before its fence signalled was put back to try
        // on the next frame, and before the window shows nothing but a band
        // arriving makes one: a short page's last texture waited for ever,
        // and its window never showed.
        let _ = gl.client_wait_sync(fence, glow::SYNC_FLUSH_COMMANDS_BIT, 1_000_000_000);
        Ok((texture, fence))
    }
}

/// Makes each band sent into a texture on the loader's context, current on
/// this thread, and sends it back, until the loader is let go or nothing
/// takes what it makes.
fn make_all<T>(
    egl: &egl::Instance<egl::Static>,
    jobs: Jobs<Job<T>>,
    made: &calloop::channel::Sender<Made<T>>,
) {
    let gl = unsafe {
        glow::Context::from_loader_function(|name| {
            egl.get_proc_address(name)
                .map_or(std::ptr::null(), |f| f as *const c_void)
        })
    };
    unsafe { gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4) };
    let warm = std::env::var_os("READER_WARM").is_some();
    for job in jobs {
        match unsafe { make(&gl, &job, warm) } {
            Ok((texture, fence)) => {
                let texture = Texture {
                    handle: texture,
                    width: job.width,
                    height: job.height,
                };
                let fence = Fence(fence);
                if made
                    .send(Made {
                        tag: job.tag,
                        texture,
                        fence,
                    })
                    .is_err()
                {
                    return;
                }
            }
            Err(error) => eprintln!("can't load a band: {error}"),
        }
    }
}

/// The step a buffer grows by, in device pixels.
const GROWS_BY: i32 = 256;

fn grown(size: i32, held: i32) -> i32 {
    if size <= held {
        held
    } else {
        (size + GROWS_BY - 1) / GROWS_BY * GROWS_BY
    }
}

/// How a mark recolours the page under it, as Neovim's Search colours a
/// match: the page's ground becomes the mark's ground and its text the
/// mark's text, with what lies between, a glyph's soft edge, in proportion.
/// Each channel is a line through those two points, `source + constant *
/// under`, or, for a mark darker under light text, `source * (1 - under) +
/// constant * under`, which GL's blend draws in one pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blend {
    pub source: [f32; 3],
    pub constant: [f32; 3],
    pub flipped: bool,
}

impl Blend {
    pub fn between(
        page_ground: [f32; 3],
        page_ink: [f32; 3],
        ground: [f32; 3],
        ink: [f32; 3],
    ) -> Blend {
        let slope: [f32; 3] =
            std::array::from_fn(|c| (ink[c] - ground[c]) / (page_ink[c] - page_ground[c]));
        let flipped = slope.iter().any(|s| *s < 0.0);
        let source: [f32; 3] = std::array::from_fn(|c| ground[c] - slope[c] * page_ground[c]);
        let constant: [f32; 3] = std::array::from_fn(|c| {
            if flipped {
                source[c] + slope[c]
            } else {
                slope[c]
            }
        });
        Blend {
            source: source.map(|v| v.clamp(0.0, 1.0)),
            constant: constant.map(|v| v.clamp(0.0, 1.0)),
            flipped,
        }
    }

    /// What the blend makes of a colour under it.
    #[cfg(test)]
    pub fn over(&self, under: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|c| {
            let kept = if self.flipped { 1.0 - under[c] } else { 1.0 };
            (self.source[c] * kept + self.constant[c] * under[c]).clamp(0.0, 1.0)
        })
    }
}

pub struct Texture {
    handle: glow::Texture,
    pub width: u32,
    pub height: u32,
}

impl Gl {
    /// # Safety
    /// `display` must be the live Wayland display the surface belongs to.
    pub unsafe fn new(
        display: *mut c_void,
        surface: &WlSurface,
        width: i32,
        height: i32,
        cut: bool,
    ) -> Result<Gl, String> {
        let held = if cut {
            (grown(width, 0), grown(height, 0))
        } else {
            (width, height)
        };
        let egl = egl::Instance::new(egl::Static);
        let display = unsafe { egl.get_display(display) }.ok_or("no EGL display")?;
        egl.initialize(display).map_err(|e| format!("EGL: {e}"))?;
        egl.bind_api(egl::OPENGL_ES_API)
            .map_err(|e| format!("EGL: {e}"))?;
        let wanted = [
            egl::RED_SIZE,
            8,
            egl::GREEN_SIZE,
            8,
            egl::BLUE_SIZE,
            8,
            egl::SURFACE_TYPE,
            egl::WINDOW_BIT,
            egl::RENDERABLE_TYPE,
            egl::OPENGL_ES3_BIT,
            egl::NONE,
        ];
        let config = egl
            .choose_first_config(display, &wanted)
            .map_err(|e| format!("EGL: {e}"))?
            .ok_or("no EGL config for GLES 3")?;
        let context = egl
            .create_context(
                display,
                config,
                None,
                &[egl::CONTEXT_MAJOR_VERSION, 3, egl::NONE],
            )
            .map_err(|e| format!("EGL: {e}"))?;
        let window = wayland_egl::WlEglSurface::new(surface.id(), held.0, held.1)
            .map_err(|e| format!("wayland-egl: {e}"))?;
        let surface = unsafe {
            egl.create_window_surface(display, config, window.ptr() as egl::NativeWindowType, None)
        }
        .map_err(|e| format!("EGL: {e}"))?;
        egl.make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(|e| format!("EGL: {e}"))?;
        // The frame callbacks pace the drawing; a swap mustn't wait as well.
        egl.swap_interval(display, 0)
            .map_err(|e| format!("EGL: {e}"))?;
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                egl.get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const c_void)
            })
        };
        let (program, rect, vertices) = unsafe {
            let program = gl.create_program()?;
            for (kind, source) in [
                (glow::VERTEX_SHADER, VERTEX),
                (glow::FRAGMENT_SHADER, FRAGMENT),
            ] {
                let shader = gl.create_shader(kind)?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    return Err(gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }
            gl.use_program(Some(program));
            let rect = gl
                .get_uniform_location(program, "rect")
                .ok_or("no rect uniform")?;
            // GLES 3 draws only with a vertex array bound, though the quad's
            // corners come from the vertex's number.
            let vertices = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vertices));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            (program, rect, vertices)
        };
        Ok(Gl {
            egl,
            display,
            surface,
            config,
            context,
            window,
            gl,
            program,
            rect,
            _vertices: vertices,
            width,
            height,
            held,
            cut,
        })
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        let held = if self.cut {
            (grown(width, self.held.0), grown(height, self.held.1))
        } else {
            (width, height)
        };
        if held != self.held {
            self.window.resize(held.0, held.1, 0, 0);
            self.held = held;
        }
        self.width = width;
        self.height = height;
    }

    /// Starts the thread that makes textures, on a context sharing this
    /// one's, and sends each back through `made` as it's sent to the GPU.
    /// Making a texture took 6 to 10 ms, more than a frame at 120 Hz, and
    /// can't be cut into parts; there it costs a frame nothing.
    pub fn loader<T: Send + 'static>(
        &self,
        made: calloop::channel::Sender<Made<T>>,
    ) -> Result<Loader<T>, String> {
        let context = self
            .egl
            .create_context(
                self.display,
                self.config,
                Some(self.context),
                &[egl::CONTEXT_MAJOR_VERSION, 3, egl::NONE],
            )
            .map_err(|e| format!("EGL: {e}"))?;
        let share = Share {
            display: self.display,
            context,
        };
        let jobs = Worker::start("loader", move |jobs| {
            let share = share;
            let egl = egl::Instance::new(egl::Static);
            // No surface: the loader only makes textures.
            if let Err(error) = egl.make_current(share.display, None, None, Some(share.context)) {
                eprintln!("the loader can't start: {error}");
                let _ = egl.destroy_context(share.display, share.context);
                return;
            }
            make_all(&egl, jobs, &made);
            // Let go here, on the thread it's current on, before the
            // display it belongs to ends.
            let _ = egl.make_current(share.display, None, None, None);
            let _ = egl.destroy_context(share.display, share.context);
        })
        .map_err(|e| format!("the loader: {e}"))?;
        Ok(Loader { jobs })
    }

    /// Takes a texture the loader made once the GPU has it whole, or gives
    /// it back to wait for a later frame.
    pub fn adopt<T>(&self, made: Made<T>) -> Result<(T, Texture), Made<T>> {
        let status = unsafe { self.gl.client_wait_sync(made.fence.0, 0, 0) };
        if status == glow::ALREADY_SIGNALED || status == glow::CONDITION_SATISFIED {
            unsafe { self.gl.delete_sync(made.fence.0) };
            Ok((made.tag, made.texture))
        } else {
            Err(made)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "a texture forgotten is taken, so nothing draws it after"
    )]
    pub fn forget(&self, texture: Texture) {
        unsafe { self.gl.delete_texture(texture.handle) };
    }

    /// An empty texture to paint again and again, as the prompt is: made
    /// once, since making one is what costs.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "GL takes sizes and places as i32 and f32; a texture or window is a few thousand pixels at most"
    )]
    pub fn canvas(&self, width: u32, height: u32) -> Result<Texture, String> {
        unsafe {
            let texture = self.gl.create_texture()?;
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for (name, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                self.gl
                    .tex_parameter_i32(glow::TEXTURE_2D, name, value as i32);
            }
            self.gl.tex_storage_2d(
                glow::TEXTURE_2D,
                1,
                glow::RGBA8,
                width as i32,
                height as i32,
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            Ok(Texture {
                handle: texture,
                width,
                height,
            })
        }
    }

    /// Fills a canvas with pixels of its own size.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "GL takes sizes and places as i32 and f32; a texture or window is a few thousand pixels at most"
    )]
    pub fn paint(&self, texture: &Texture, pixels: &[u8]) {
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture.handle));
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                texture.width as i32,
                texture.height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(pixels)),
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    /// A texture of one pixel: a blend's source, drawn stretched.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "GL takes sizes and places as i32 and f32; a texture or window is a few thousand pixels at most"
    )]
    pub fn swatch(&self, colour: [f32; 3]) -> Result<Texture, String> {
        let [r, g, b] = colour.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
        let texture = self.canvas(1, 1)?;
        self.paint(&texture, &[r, g, b, 255]);
        Ok(texture)
    }

    /// Draws a blend's swatch over what is drawn, so what lies under it is
    /// recoloured as the blend says.
    #[expect(
        clippy::many_single_char_names,
        reason = "a colour's r, g and b and a place's x and y, as GL names them"
    )]
    pub fn mark(&self, swatch: &Texture, blend: &Blend, x: f32, y: f32, width: f32, height: f32) {
        let [r, g, b] = blend.constant;
        let source = if blend.flipped {
            glow::ONE_MINUS_DST_COLOR
        } else {
            glow::ONE
        };
        unsafe {
            self.gl.enable(glow::BLEND);
            self.gl.blend_color(r, g, b, 1.0);
            self.gl.blend_func(source, glow::CONSTANT_COLOR);
        }
        self.draw(swatch, x, y, width, height);
        unsafe { self.gl.disable(glow::BLEND) };
    }

    /// Starts a frame: the whole buffer in the page's ground.
    pub fn clear(&self, [r, g, b]: [f32; 3]) {
        // GL counts rows from the bottom, and the window shows the top.
        let bottom = self.held.1 - self.height;
        unsafe {
            self.gl.viewport(0, bottom, self.width, self.height);
            self.gl.enable(glow::SCISSOR_TEST);
            self.gl.scissor(0, bottom, self.width, self.height);
            self.gl.clear_color(r, g, b, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    /// Draws a texture into a rectangle given in device pixels, from the
    /// buffer's top left.
    #[expect(
        clippy::cast_precision_loss,
        reason = "GL takes sizes and places as i32 and f32; a texture or window is a few thousand pixels at most"
    )]
    pub fn draw(&self, texture: &Texture, x: f32, y: f32, width: f32, height: f32) {
        let (w, h) = (self.width as f32, self.height as f32);
        let left = -1.0 + 2.0 * x / w;
        let right = -1.0 + 2.0 * (x + width) / w;
        let top = 1.0 - 2.0 * y / h;
        let bottom = 1.0 - 2.0 * (y + height) / h;
        unsafe {
            self.gl.use_program(Some(self.program));
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture.handle));
            self.gl
                .uniform_4_f32(Some(&self.rect), left, top, right, bottom);
            self.gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }

    /// Ends a frame. The swap commits the surface.
    pub fn swap(&self) -> Result<(), String> {
        self.egl
            .swap_buffers(self.display, self.surface)
            .map_err(|e| format!("EGL: {e}"))
    }
}

impl Drop for Gl {
    /// Lets go of the context and its surface while the window and the
    /// Wayland connection they draw to are still there, and ends the
    /// display, so the driver has nothing left to tear down at exit. The
    /// loader is let go first, so no thread is in the driver as it ends.
    fn drop(&mut self) {
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_surface(self.display, self.surface);
        let _ = self.egl.destroy_context(self.display, self.context);
        let _ = self.egl.terminate(self.display);
    }
}

#[cfg(test)]
mod tests {
    use super::Blend;

    const FOREST: [f32; 3] = [0.067, 0.110, 0.094];
    const INK: [f32; 3] = [0.902, 0.875, 0.784];

    fn near(a: [f32; 3], b: [f32; 3], within: f32) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() <= within)
    }

    #[test]
    fn a_lighter_mark_takes_ground_and_text_where_neovim_puts_them() {
        let (fern, cream) = ([0.137, 0.216, 0.169], [0.969, 0.910, 0.698]);
        let blend = Blend::between(FOREST, INK, fern, cream);
        assert!(!blend.flipped);
        assert!(near(blend.over(FOREST), fern, 0.01));
        assert!(near(blend.over(INK), cream, 0.01));
    }

    #[test]
    fn a_mark_under_dark_text_turns_light_text_dark() {
        let (jade, night) = ([0.314, 0.580, 0.459], [0.035, 0.059, 0.051]);
        let blend = Blend::between(FOREST, INK, jade, night);
        assert!(blend.flipped);
        assert!(near(blend.over(FOREST), jade, 0.03));
        // Its text goes as dark as one blend can take it: within a tenth.
        assert!(near(blend.over(INK), night, 0.1));
    }
}
