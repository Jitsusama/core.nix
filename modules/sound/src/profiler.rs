//! PipeWire's profiler, bound as pw-top binds it, which the pipewire
//! crate has no binding for: its proxy from the registry, and a listener
//! on its one event, each profile, handed on as its pod's bytes. While
//! it's bound, the server measures every cycle for it, so it's bound only
//! while something wants how nodes keep time. Beside it, a timer on the
//! same loop ticks each second it's bound, since the profiler says
//! nothing while every driver idles, and a second without a word from it
//! has to stand too.

use std::ffi::{CString, c_void};
use std::pin::Pin;

use pipewire as pw;
use pw::context::Context;
use pw::main_loop::MainLoopRc;
use pw::registry::RegistryRc;
use pw::spa::pod::Pod;
use pw::spa::spa_interface_call_method;
use pw::spa::sys::{
    spa_hook, spa_interface, spa_loop_utils_methods, spa_pod, spa_source, timespec,
};
use pw::sys::{pw_profiler_events, pw_profiler_methods, pw_proxy, pw_registry_methods};
use pw::types::ObjectType;

type Heard = Box<dyn FnMut(&[u8])>;
type Tick = Box<dyn FnMut()>;

/// The profiler's proxy and what hears it, kept together, since the
/// server holds the hook and calls the events with the data for as long
/// as they're bound; and the second's timer, with what it calls and the
/// loop it's on, kept so the loop outlives it.
pub struct Profiler {
    proxy: *mut pw_proxy,
    hook: Pin<Box<spa_hook>>,
    _events: Pin<Box<pw_profiler_events>>,
    _heard: Box<Heard>,
    main_loop: MainLoopRc,
    timer: *mut spa_source,
    _tick: Box<Tick>,
}

/// The loop's utilities, where its timers are made, armed and let go.
fn utilities(main_loop: &MainLoopRc) -> *mut spa_interface {
    // SAFETY: a live loop always has its utilities, and the interface is
    // the first thing in them.
    unsafe { (*main_loop.loop_().as_raw_ptr()).utils.cast() }
}

/// A second's timer on the loop, calling `tick` each second from now; none
/// where the loop makes none.
fn ticking(main_loop: &MainLoopRc, tick: &mut Tick) -> Option<*mut spa_source> {
    let utils = utilities(main_loop);
    let data: *mut Tick = tick;
    // SAFETY: the loop is live, and the tick is boxed and kept beside the
    // timer until the timer is destroyed (Drop).
    let timer: *mut spa_source = unsafe {
        spa_interface_call_method!(
            utils,
            spa_loop_utils_methods,
            add_timer,
            Some(ticked),
            data.cast::<c_void>()
        )
    };
    if timer.is_null() {
        return None;
    }
    let second = timespec {
        tv_sec: 1,
        tv_nsec: 0,
    };
    // SAFETY: the timer is the loop's, just made; the times are read
    // during the call.
    unsafe {
        spa_interface_call_method!(
            utils,
            spa_loop_utils_methods,
            update_timer,
            timer,
            (&raw const second).cast_mut(),
            (&raw const second).cast_mut(),
            false
        );
    }
    Some(timer)
}

/// Each second the timer says.
unsafe extern "C" fn ticked(data: *mut c_void, _expirations: u64) {
    if data.is_null() {
        return;
    }
    // SAFETY: the data is the `Tick` made with the timer, alive until the
    // timer is destroyed.
    unsafe { (*data.cast::<Tick>())() }
}

/// Teaches a context the profiler's protocol, as pw-top does before it
/// binds: the marshal that makes its proxy lives in the profiler's
/// module, which a context doesn't load by itself, so without it every
/// bind comes back empty. False where the module isn't there.
pub fn taught(context: &Context) -> bool {
    // SAFETY: the context is live while it's borrowed, and owns the
    // module it loads, which goes when the context does.
    let module = unsafe {
        pw::sys::pw_context_load_module(
            context.as_raw_ptr(),
            c"libpipewire-module-profiler".as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    !module.is_null()
}

impl Profiler {
    /// Binds the profiler by its global's id, calling `heard` with each
    /// profile's pod on PipeWire's loop, and `tick` each second it's
    /// bound; none where the server refuses.
    pub fn bind(
        registry: &RegistryRc,
        main_loop: &MainLoopRc,
        id: u32,
        heard: impl FnMut(&[u8]) + 'static,
        tick: impl FnMut() + 'static,
    ) -> Option<Self> {
        let kind = CString::new(ObjectType::Profiler.to_str()).ok()?;
        // SAFETY: the registry's pointer is its own, live while it is,
        // and bind is the method its interface names for this.
        let proxy: *mut pw_proxy = unsafe {
            spa_interface_call_method!(
                registry.as_raw_ptr(),
                pw_registry_methods,
                bind,
                id,
                kind.as_ptr(),
                pw::sys::PW_VERSION_PROFILER,
                0
            )
        }
        .cast();
        if proxy.is_null() {
            return None;
        }
        let mut tick: Box<Tick> = Box::new(Box::new(tick));
        let Some(timer) = ticking(main_loop, &mut tick) else {
            // SAFETY: the proxy was just bound, and nothing listens on it.
            unsafe { pw::sys::pw_proxy_destroy(proxy) };
            return None;
        };
        let mut heard: Box<Heard> = Box::new(Box::new(heard));
        // SAFETY: both are plain C structs, whole when zeroed, and the
        // events are filled before they're handed over.
        let (mut hook, events) = unsafe {
            let mut events: Pin<Box<pw_profiler_events>> = Box::pin(std::mem::zeroed());
            events.version = pw::sys::PW_VERSION_PROFILER_EVENTS;
            events.profile = Some(profiled);
            let hook: Pin<Box<spa_hook>> = Box::pin(std::mem::zeroed());
            (hook, events)
        };
        // SAFETY: the proxy is the profiler's, just bound; the hook, the
        // events and the data are pinned on the heap and kept beside it
        // until the hook is removed and the proxy destroyed (Drop).
        unsafe {
            let hook: *mut spa_hook = hook.as_mut().get_unchecked_mut();
            let data: *mut Heard = &raw mut *heard;
            spa_interface_call_method!(
                proxy,
                pw_profiler_methods,
                add_listener,
                hook,
                events.as_ref().get_ref(),
                data.cast::<c_void>()
            );
        }
        Some(Self {
            proxy,
            hook,
            _events: events,
            _heard: heard,
            main_loop: main_loop.clone(),
            timer,
            _tick: tick,
        })
    }
}

/// Each profile, as the server says one: the pod handed on whole.
unsafe extern "C" fn profiled(data: *mut c_void, pod: *const spa_pod) {
    if data.is_null() || pod.is_null() {
        return;
    }
    // SAFETY: the data is the `Heard` bound with the hook, alive until the
    // hook is removed, and the pod is the server's for this call.
    unsafe {
        let heard = &mut *data.cast::<Heard>();
        heard(Pod::from_raw(pod).as_bytes());
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        // SAFETY: the timer is the loop's, kept alive here, and destroyed
        // once; the hook was added by bind and is removed once, before the
        // proxy it's on is destroyed, so nothing calls either after.
        unsafe {
            spa_interface_call_method!(
                utilities(&self.main_loop),
                spa_loop_utils_methods,
                destroy_source,
                self.timer
            );
            pw::spa::utils::hook::remove(*self.hook.as_mut().get_unchecked_mut());
            pw::sys::pw_proxy_destroy(self.proxy);
        }
    }
}
