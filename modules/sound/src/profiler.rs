//! PipeWire's profiler, bound as pw-top binds it, which the pipewire
//! crate has no binding for: its proxy from the registry, and a listener
//! on its one event, each profile, handed on as its pod's bytes. While
//! it's bound, the server measures every cycle for it, so it's bound only
//! while something wants how nodes keep time.

use std::ffi::{CString, c_void};
use std::pin::Pin;

use pipewire as pw;
use pw::registry::RegistryRc;
use pw::spa::pod::Pod;
use pw::spa::spa_interface_call_method;
use pw::spa::sys::{spa_hook, spa_pod};
use pw::sys::{pw_profiler_events, pw_profiler_methods, pw_proxy, pw_registry_methods};
use pw::types::ObjectType;

type Heard = Box<dyn FnMut(&[u8])>;

/// The profiler's proxy and what hears it, kept together, since the
/// server holds the hook and calls the events with the data for as long
/// as they're bound.
pub struct Profiler {
    proxy: *mut pw_proxy,
    hook: Pin<Box<spa_hook>>,
    _events: Pin<Box<pw_profiler_events>>,
    _heard: Box<Heard>,
}

impl Profiler {
    /// Binds the profiler by its global's id, calling `heard` with each
    /// profile's pod on PipeWire's loop; none where the server refuses.
    pub fn bind(
        registry: &RegistryRc,
        id: u32,
        heard: impl FnMut(&[u8]) + 'static,
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
        // SAFETY: the hook was added by bind and is removed once, before
        // the proxy it's on is destroyed, so nothing calls it after.
        unsafe {
            pw::spa::utils::hook::remove(*self.hook.as_mut().get_unchecked_mut());
            pw::sys::pw_proxy_destroy(self.proxy);
        }
    }
}
