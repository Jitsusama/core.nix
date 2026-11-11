//! What Wayland tells the reader: the compositor's frames and scale, the seat
//! and its keyboard, the outputs, the window's configures, and the
//! viewporter's, fractional scale's and presentation's events.

use super::{
    ZOOM_UNTIL, current_marks, cursor_marks, ground, page_ink, search_marks, selection_marks,
};
use crate::{App, Feedback, Presentation, Scaling, Viewporting, gl, keys, keys_logged, theme};
use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers,
};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::window::{Window, WindowConfigure, WindowHandler};
use smithay_client_toolkit::{delegate_registry, registry_handlers};
use wayland_client::protocol::{wl_keyboard, wl_output, wl_seat, wl_surface};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
};
use wayland_protocols::wp::presentation_time::client::wp_presentation::WpPresentation;
use wayland_protocols::wp::presentation_time::client::wp_presentation_feedback::WpPresentationFeedback;
use wayland_protocols::wp::presentation_time::client::{wp_presentation, wp_presentation_feedback};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        // Under a fractional scale the compositor says the scale exactly,
        // and this whole one is only rounded.
        if self.fractional.is_some() || surface != self.window.wl_surface() {
            return;
        }
        self.rescale(f64::from(factor));
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.frame_pending = false;
        self.draw();
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let repeat = Box::new(
                |app: &mut App, _: &wl_keyboard::WlKeyboard, event: KeyEvent| {
                    app.key(&event, true);
                },
            );
            match self.seats.get_keyboard_with_repeat(
                qh,
                &seat,
                None,
                self.loop_handle.clone(),
                repeat,
            ) {
                Ok(keyboard) => {
                    if keys_logged() {
                        eprintln!("keys: a keyboard");
                    }
                    self.keyboard = Some(keyboard);
                }
                Err(error) => eprintln!("no keys: {error}"),
            }
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            keyboard.release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        if keys_logged() {
            eprintln!("keys: focused");
        }
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        // A key waiting for its second is dropped when focus goes, as vi's
        // would be by anything else typed.
        self.keys = keys::Machine::default();
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(&event, false);
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(&event, true);
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.ctrl = modifiers.ctrl;
        self.shift = modifiers.shift;
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl WindowHandler for App {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _: u32,
    ) {
        let (width, height) = configure.new_size;
        let at_end = self.shown && self.at_end();
        self.size = (
            width.map_or(960, std::num::NonZero::get),
            height.map_or(900, std::num::NonZero::get),
        );
        self.fit_window();
        let (w, h) = self.buffer_size();
        if let Some(measure) = self.measure.as_mut()
            && measure.configured_ms.is_none()
        {
            measure.configured_ms = Some(self.opened.elapsed().as_secs_f64() * 1000.0);
        }
        if let Some(measure) = self.measure.as_mut()
            && measure
                .start
                .is_some_and(|s| s.elapsed().as_secs_f64() >= ZOOM_UNTIL)
        {
            measure.configures += 1;
        }
        if let Some(gl) = self.gl.as_mut() {
            gl.resize(w, h);
        } else {
            window.wl_surface().set_buffer_scale(self.buffer_scale());
            // SAFETY: the display is the connection's, which outlives
            // the window and its GL.
            let cut = self.viewport.is_some();
            match unsafe { gl::Gl::new(self.display, window.wl_surface(), w, h, cut) } {
                Ok(gl) => {
                    match gl.loader(self.made_to.clone()) {
                        Ok(loader) => self.loader = Some(loader),
                        Err(error) => {
                            eprintln!("can't draw: {error}");
                            self.exit = true;
                            return;
                        }
                    }
                    let make = |(ground, ink): ([f32; 3], [f32; 3])| {
                        let blend = gl::Blend::between(self::ground(), page_ink(), ground, ink);
                        gl.swatch(blend.source).ok().map(|swatch| (swatch, blend))
                    };
                    self.lit = make(search_marks())
                        .zip(make(current_marks()))
                        .map(|(a, b)| [a, b]);
                    // The cursor always stands on a character the
                    // selection holds, so it recolours the selection's.
                    let page = (self::ground(), page_ink());
                    self.chosen = gl
                        .chosen(page, selection_marks())
                        .ok()
                        .zip(gl.chosen(selection_marks(), cursor_marks()).ok())
                        .map(|(a, b)| [a, b]);
                    self.rule = gl.swatch(theme::get().accent.gl()).ok();
                    self.bare = gl.swatch(self::ground()).ok();
                    self.gl = Some(gl);
                }
                Err(error) => {
                    eprintln!("can't draw: {error}");
                    self.exit = true;
                    return;
                }
            }
        }
        // The view's end moves with the buffer's height, so a view at the
        // end keeps to it once the buffer is the window's new size.
        if at_end {
            self.keep_to_end();
        }
        // A key list showing is set again for the window's new width.
        if self.keys_shown {
            self.show_keys(true);
        }
        self.cut();
        if !self.frame_pending {
            self.draw();
        }
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

delegate_registry!(App);

smithay_client_toolkit::delegate_dispatch2!(App);

impl Dispatch2<WpViewporter, App> for Viewporting {
    fn event(
        &self,
        _: &mut App,
        _: &WpViewporter,
        _: wp_viewporter::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch2<WpViewport, App> for Viewporting {
    fn event(
        &self,
        _: &mut App,
        _: &WpViewport,
        _: wp_viewport::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch2<WpFractionalScaleManagerV1, App> for Scaling {
    fn event(
        &self,
        _: &mut App,
        _: &WpFractionalScaleManagerV1,
        _: wp_fractional_scale_manager_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch2<WpFractionalScaleV1, App> for Scaling {
    fn event(
        &self,
        app: &mut App,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            app.rescale(f64::from(scale) / 120.0);
        }
    }
}

impl Dispatch2<WpPresentation, App> for Presentation {
    fn event(
        &self,
        _: &mut App,
        _: &WpPresentation,
        _: wp_presentation::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

impl Dispatch2<WpPresentationFeedback, App> for Feedback {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    fn event(
        &self,
        app: &mut App,
        _: &WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        let Some(measure) = app.measure.as_mut() else {
            return;
        };
        match event {
            wp_presentation_feedback::Event::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                ..
            } => {
                let seconds = (u64::from(tv_sec_hi) << 32) | u64::from(tv_sec_lo);
                if let Some(drawn) = measure.drawn.get_mut(self.0 as usize) {
                    drawn.presented_ns = Some(seconds * 1_000_000_000 + u64::from(tv_nsec));
                }
                if refresh > 0 {
                    measure.refresh_ns = refresh;
                }
            }
            wp_presentation_feedback::Event::Discarded => measure.discarded += 1,
            _ => (),
        }
    }
}
