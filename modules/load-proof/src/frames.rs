//! Frames: whether niri shows every frame it would show on an idle
//! machine.
//!
//! A surface Joel can't see (one pixel on the overlay layer, too faint to
//! see and changed every frame, taking no input) asks for a new frame at
//! every refresh, as an animation or a video would, and asks niri through `wp_presentation` when each one
//! reached the screen. The display counts its refreshes, so a frame that
//! lands two refreshes after the one before means one was missed. That is
//! niri's own answer, read without privilege and without a debug build.
//!
//! A refresh can be missed by the probe as well as by niri: if the probe
//! itself waited for a CPU and committed its frame after the refresh had
//! passed, niri had nothing new to show. So each frame's commit is timed
//! on the same clock niri reports on, and a refresh that passed before the
//! probe committed is counted as the probe's, not niri's. The probe
//! commits when niri says the last frame is done, so a refresh niri was
//! slow to say so for counts as the probe's too: `missed` is the least
//! niri missed, and `late` holds both.

use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;
use std::time::{Duration, Instant};

use rustix::fs::{MemfdFlags, memfd_create};
use rustix::time::{ClockId, clock_gettime};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::wp::presentation_time::client::{wp_presentation, wp_presentation_feedback};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

/// Linux's number for `CLOCK_MONOTONIC`, as `wp_presentation` names its clock.
const MONOTONIC: u32 = 1;

/// What niri said of one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shown {
    /// It reached the screen at the display's refresh `seq`, at `at_ns`
    /// on the presentation clock, with `refresh_ns` between refreshes.
    Presented {
        seq: u64,
        at_ns: u64,
        refresh_ns: u32,
    },
    /// It was never shown, replaced by a later one.
    Discarded,
}

/// One frame the probe asked for: when it committed it, on the
/// presentation clock, and what niri said of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub committed_ns: u64,
    pub shown: Shown,
}

/// A run's frames, summed.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Tally {
    /// Frames shown.
    pub shown: u64,
    /// Refreshes that passed with a frame of ours ready and none shown:
    /// each one a frame niri missed.
    pub missed: u64,
    /// Refreshes that passed before the probe had committed its frame:
    /// the probe's own lateness, not niri's.
    pub late: u64,
    /// Frames niri dropped for a later one.
    pub discarded: u64,
    /// The longest time between two shown frames, in milliseconds.
    pub longest_ms: f64,
    /// The display's time between refreshes, in milliseconds.
    pub refresh_ms: f64,
}

impl Tally {
    /// Sums what niri said, in the order the frames were committed.
    pub fn of(frames: &[Frame]) -> Tally {
        let mut t = Tally::default();
        let mut last: Option<(u64, u64)> = None;
        for frame in frames {
            match frame.shown {
                Shown::Discarded => t.discarded += 1,
                Shown::Presented {
                    seq,
                    at_ns,
                    refresh_ns,
                } => {
                    t.shown += 1;
                    if refresh_ns > 0 {
                        t.refresh_ms = f64::from(refresh_ns) / 1e6;
                    }
                    if let Some((before_seq, before_at)) = last {
                        let skipped = seq.saturating_sub(before_seq).saturating_sub(1);
                        // Refreshes come every refresh_ns after the last
                        // frame shown; those before the commit were the
                        // probe's to miss.
                        let passed = if refresh_ns > 0 {
                            frame.committed_ns.saturating_sub(before_at) / u64::from(refresh_ns)
                        } else {
                            0
                        };
                        let late = skipped.min(passed);
                        t.late += late;
                        t.missed += skipped - late;
                        let gap = Duration::from_nanos(at_ns.saturating_sub(before_at))
                            .as_secs_f64()
                            * 1e3;
                        t.longest_ms = t.longest_ms.max(gap);
                    }
                    last = Some((seq, at_ns));
                }
            }
        }
        t
    }
}

struct Probe {
    committed: Vec<u64>,
    shown: Vec<Shown>,
    clock: Option<u32>,
    configured: bool,
    closed: bool,
    due: bool,
}

/// What to say when the compositor lacks a protocol the probe needs.
fn bind_err(what: &'static str) -> impl Fn(wayland_client::globals::BindError) -> String {
    move |e| format!("the compositor has no {what}: {e}")
}

fn now_ns() -> u64 {
    let t = clock_gettime(ClockId::Monotonic);
    u64::try_from(t.tv_sec).unwrap_or(0) * 1_000_000_000 + u64::try_from(t.tv_nsec).unwrap_or(0)
}

/// Asks niri for a frame at every refresh for `length`, and returns what
/// it said of each.
pub fn watch(length: Duration) -> Result<Vec<Frame>, String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland display: {e}"))?;
    let (globals, mut queue) =
        registry_queue_init::<Probe>(&conn).map_err(|e| format!("no registry: {e}"))?;
    let qh = queue.handle();
    let compositor: wl_compositor::WlCompositor = globals
        .bind(&qh, 4..=6, ())
        .map_err(bind_err("wl_compositor"))?;
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).map_err(bind_err("wl_shm"))?;
    let layers: zwlr_layer_shell_v1::ZwlrLayerShellV1 = globals
        .bind(&qh, 1..=4, ())
        .map_err(bind_err("layer shell"))?;
    let presentation: wp_presentation::WpPresentation = globals
        .bind(&qh, 1..=1, ())
        .map_err(bind_err("wp_presentation"))?;

    let surface = compositor.create_surface(&qh, ());
    // No input region: a click on the pixel goes through to what's under it.
    let nothing = compositor.create_region(&qh, ());
    surface.set_input_region(Some(&nothing));
    let layer = layers.get_layer_surface(
        &surface,
        None,
        zwlr_layer_shell_v1::Layer::Overlay,
        "load-proof-frames".into(),
        &qh,
        (),
    );
    layer.set_size(1, 1);
    layer.set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
    surface.commit();

    let mut probe = Probe {
        committed: Vec::new(),
        shown: Vec::new(),
        clock: None,
        configured: false,
        closed: false,
        due: false,
    };
    while !probe.configured && !probe.closed {
        queue
            .blocking_dispatch(&mut probe)
            .map_err(|e| format!("Wayland: {e}"))?;
    }
    if probe.closed {
        return Err("niri closed the probe's surface before it showed".into());
    }
    if probe.clock != Some(MONOTONIC) {
        return Err(format!(
            "niri reports frames on clock {:?}, not the monotonic one the probe times its commits on",
            probe.clock
        ));
    }

    // Two pixels of black, a step apart in alpha and both too faint to see,
    // taken in turn. niri draws only what's visible and presents only a
    // frame with something new in it, so a fully transparent pixel, or the
    // same one again, waits for something else on the screen to change,
    // and the probe would time that rather than niri.
    let fd = memfd_create("load-proof-pixel", MemfdFlags::CLOEXEC)
        .map_err(|e| format!("memfd_create: {e}"))?;
    let file = std::fs::File::from(fd);
    // ARGB8888 is little-endian: blue, green, red, then alpha.
    file.write_all_at(&[0, 0, 0, 1, 0, 0, 0, 2], 0)
        .map_err(|e| format!("the probe's pixels: {e}"))?;
    let pool = shm.create_pool(file.as_fd(), 8, &qh, ());
    let buffers =
        [0, 4].map(|offset| pool.create_buffer(offset, 1, 1, 4, wl_shm::Format::Argb8888, &qh, ()));

    let end = Instant::now() + length;
    probe.due = true;
    while Instant::now() < end && !probe.closed {
        if probe.due {
            probe.due = false;
            surface.attach(Some(&buffers[probe.committed.len() % 2]), 0, 0);
            surface.damage_buffer(0, 0, 1, 1);
            presentation.feedback(&surface, &qh, ());
            surface.frame(&qh, ());
            probe.committed.push(now_ns());
            surface.commit();
        }
        queue
            .blocking_dispatch(&mut probe)
            .map_err(|e| format!("Wayland: {e}"))?;
    }
    layer.destroy();
    surface.destroy();
    // The last frames' feedback arrives after the surface goes.
    let _ = queue.roundtrip(&mut probe);
    // niri answers each frame's feedback in the order it was committed.
    Ok(probe
        .committed
        .iter()
        .zip(&probe.shown)
        .map(|(&committed_ns, &shown)| Frame {
            committed_ns,
            shown,
        })
        .collect())
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Probe {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_presentation::WpPresentation, ()> for Probe {
    fn event(
        probe: &mut Self,
        _: &wp_presentation::WpPresentation,
        event: wp_presentation::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_presentation::Event::ClockId { clk_id } = event {
            probe.clock = Some(clk_id);
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for Probe {
    fn event(
        probe: &mut Self,
        layer: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                layer.ack_configure(serial);
                probe.configured = true;
            }
            zwlr_layer_surface_v1::Event::Closed => probe.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for Probe {
    fn event(
        probe: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            probe.due = true;
        }
    }
}

impl Dispatch<wp_presentation_feedback::WpPresentationFeedback, ()> for Probe {
    fn event(
        probe: &mut Self,
        _: &wp_presentation_feedback::WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wp_presentation_feedback::Event::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                seq_hi,
                seq_lo,
                ..
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) | u64::from(tv_sec_lo);
                probe.shown.push(Shown::Presented {
                    seq: (u64::from(seq_hi) << 32) | u64::from(seq_lo),
                    at_ns: secs * 1_000_000_000 + u64::from(tv_nsec),
                    refresh_ns: refresh,
                });
            }
            wp_presentation_feedback::Event::Discarded => probe.shown.push(Shown::Discarded),
            _ => {}
        }
    }
}

delegate_noop!(Probe: ignore wl_compositor::WlCompositor);
delegate_noop!(Probe: ignore wl_shm::WlShm);
delegate_noop!(Probe: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Probe: ignore wl_buffer::WlBuffer);
delegate_noop!(Probe: ignore wl_surface::WlSurface);
delegate_noop!(Probe: ignore wl_region::WlRegion);
delegate_noop!(Probe: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);

#[cfg(test)]
mod tests {
    use super::*;

    const REFRESH: u64 = 8_333_333;

    /// A frame shown at refresh `seq`, committed `committed_after` past
    /// the refresh before the one it was shown on.
    fn shown(seq: u64, committed_after: u64) -> Frame {
        Frame {
            committed_ns: (seq - 1) * REFRESH + committed_after,
            shown: Shown::Presented {
                seq,
                at_ns: seq * REFRESH,
                refresh_ns: 8_333_333,
            },
        }
    }

    /// A frame committed at once after the last one shown, at `last`.
    fn ready(seq: u64, last: u64) -> Frame {
        Frame {
            committed_ns: last * REFRESH + 1_000_000,
            shown: Shown::Presented {
                seq,
                at_ns: seq * REFRESH,
                refresh_ns: 8_333_333,
            },
        }
    }

    #[test]
    fn a_frame_at_every_refresh_misses_none() {
        let t = Tally::of(&[shown(10, 1), shown(11, 1), shown(12, 1), shown(13, 1)]);
        assert_eq!((t.shown, t.missed, t.late, t.discarded), (4, 0, 0, 0));
        assert!((t.longest_ms - 8.333_333).abs() < 0.001);
    }

    #[test]
    fn a_refresh_skipped_with_a_frame_ready_is_niri_missing_it() {
        let t = Tally::of(&[shown(10, 1), shown(11, 1), ready(14, 11), shown(15, 1)]);
        assert_eq!((t.missed, t.late), (2, 0));
        assert!((t.longest_ms - 25.0).abs() < 0.001);
    }

    #[test]
    fn a_refresh_passed_before_the_probe_committed_is_the_probes_own() {
        // Committed 2.5 refreshes after frame 11 was shown: refreshes 12
        // and 13 passed before it, so niri could show it first at 14.
        let late = Frame {
            committed_ns: 11 * REFRESH + 2 * REFRESH + REFRESH / 2,
            shown: Shown::Presented {
                seq: 14,
                at_ns: 14 * REFRESH,
                refresh_ns: 8_333_333,
            },
        };
        let t = Tally::of(&[shown(10, 1), shown(11, 1), late]);
        assert_eq!((t.missed, t.late), (0, 2));
    }

    #[test]
    fn a_late_commit_and_a_miss_after_it_are_told_apart() {
        // Committed after refresh 12 passed; niri then skipped 13 too.
        let both = Frame {
            committed_ns: 11 * REFRESH + REFRESH + 1_000_000,
            shown: Shown::Presented {
                seq: 14,
                at_ns: 14 * REFRESH,
                refresh_ns: 8_333_333,
            },
        };
        let t = Tally::of(&[shown(11, 1), both]);
        assert_eq!((t.missed, t.late), (1, 1));
    }

    #[test]
    fn a_discarded_frame_is_counted_apart() {
        let dropped = Frame {
            committed_ns: 0,
            shown: Shown::Discarded,
        };
        let t = Tally::of(&[shown(1, 1), dropped, shown(2, 1)]);
        assert_eq!((t.shown, t.missed, t.discarded), (2, 0, 1));
    }

    #[test]
    fn the_first_frame_misses_nothing_before_it() {
        assert_eq!(Tally::of(&[shown(1000, 1)]).missed, 0);
    }
}
