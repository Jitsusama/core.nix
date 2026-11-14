//! The desktop's clipboard, set and read through the compositor's
//! data-control protocol (ext-data-control-v1).
//!
//! data-control is what clipboard managers use: it sets and reads both
//! selections with no surface, so a copy or a paste never takes the keys
//! or the focus, and never opens a window. Each request opens a
//! connection of its own and reads the selection as it stands, so a paste
//! checks the types of the very offer it reads. A copy keeps its
//! connection, on a thread of its own, serving every program that pastes
//! it until something else is copied, or once when it's asked to be
//! served once, as wl-copy does in the background.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use wayland_client::backend::ObjectId;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, event_created_child};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::{self, ExtDataControlOfferV1},
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};

use crate::kinds;
use crate::wire::{Selection, Wanted, Why};

/// How long the program holding a selection has to hand it over, whole,
/// before a paste gives up on it.
const PATIENCE: Duration = Duration::from_secs(10);

/// What a selection is read in, at a time.
const CHUNK: usize = 64 << 10;

/// A selection read: the type it was read as, and its bytes.
#[derive(Debug)]
pub struct Pasted {
    pub kind: String,
    pub bytes: Vec<u8>,
}

/// The types `selection` is offered as now, or why they can't be said.
pub fn types(selection: Selection) -> Result<Vec<String>, Why> {
    let desk = Desk::open()?;
    desk.held(selection).map(|(_, types)| types.to_vec())
}

/// What `selection` holds, as the type `wanted`, up to `largest` bytes;
/// refused when a password manager marked it secret.
pub fn paste(selection: Selection, wanted: &Wanted, largest: usize) -> Result<Pasted, Why> {
    let desk = Desk::open()?;
    let (offer, types) = desk.held(selection)?;
    if kinds::secret(types) {
        return Err(Why::Secret);
    }
    let kind = kinds::chosen(types, wanted)
        .ok_or(Why::NoSuchType)?
        .to_owned();
    let (reader, writer) = io::pipe().map_err(|_| Why::NoDisplay)?;
    offer.receive(kind.clone(), writer.as_fd());
    desk.connection.flush().map_err(|_| Why::NoDisplay)?;
    // The program writes into its own copy of the pipe; once it closes
    // that, the read ends.
    drop(writer);
    let bytes = read_in_time(reader, largest)?;
    Ok(Pasted { kind, bytes })
}

/// Empty `selection`.
pub fn clear(selection: Selection) -> Result<(), Why> {
    let mut desk = Desk::open()?;
    match selection {
        Selection::Clipboard => desk.device.set_selection(None),
        Selection::Primary => desk.device.set_primary_selection(None),
    }
    desk.settle()
}

/// Put `bytes` on `selection`, offered as `types`, and serve them on a
/// thread of their own until something else is copied, or to one paste
/// when `once`.
pub fn copy(selection: Selection, types: &[String], bytes: Vec<u8>, once: bool) -> Result<(), Why> {
    let mut desk = Desk::open()?;
    let handle = desk.queue.handle();
    let source = desk.manager.create_data_source(&handle, ());
    for kind in types {
        source.offer(kind.clone());
    }
    match selection {
        Selection::Clipboard => desk.device.set_selection(Some(&source)),
        Selection::Primary => desk.device.set_primary_selection(Some(&source)),
    }
    desk.state.serving = Some(Serving {
        bytes: bytes.into(),
        once,
        served: false,
        cancelled: false,
    });
    desk.settle()?;
    thread::spawn(move || desk.serve(&source));
    Ok(())
}

/// One connection to the compositor, with the seat's data-control device.
struct Desk {
    connection: Connection,
    queue: EventQueue<State>,
    manager: ExtDataControlManagerV1,
    device: ExtDataControlDeviceV1,
    state: State,
}

impl Desk {
    /// A connection to the compositor `WAYLAND_DISPLAY` names, its first
    /// seat's device made and both selections, as they stand, read.
    fn open() -> Result<Self, Why> {
        let connection = Connection::connect_to_env().map_err(|_| Why::NoDisplay)?;
        let (globals, queue) =
            registry_queue_init::<State>(&connection).map_err(|_| Why::NoDisplay)?;
        let handle = queue.handle();
        let manager: ExtDataControlManagerV1 = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| Why::NoDisplay)?;
        let seat: wl_seat::WlSeat = globals
            .bind(&handle, 1..=1, ())
            .map_err(|_| Why::NoDisplay)?;
        let device = manager.get_data_device(&seat, &handle, ());
        let mut desk = Desk {
            connection,
            queue,
            manager,
            device,
            state: State::default(),
        };
        desk.settle()?;
        Ok(desk)
    }

    /// Wait for the compositor to answer everything asked so far.
    fn settle(&mut self) -> Result<(), Why> {
        self.queue
            .roundtrip(&mut self.state)
            .map_err(|_| Why::NoDisplay)?;
        if self.state.finished {
            return Err(Why::NoDisplay);
        }
        Ok(())
    }

    /// The offer `selection` holds and its types, or nothing copied.
    fn held(&self, selection: Selection) -> Result<(&ExtDataControlOfferV1, &[String]), Why> {
        let offer = match selection {
            Selection::Clipboard => self.state.selection.as_ref(),
            Selection::Primary => self.state.primary.as_ref(),
        }
        .ok_or(Why::NothingCopied)?;
        let types = self
            .state
            .offers
            .get(&offer.id())
            .ok_or(Why::NothingCopied)?;
        Ok((offer, types.as_slice()))
    }

    /// Serve the copy `source` offers until it's replaced or, when it's
    /// served once, until it has been.
    fn serve(mut self, source: &ExtDataControlSourceV1) {
        while self.state.still_serving() {
            if self.queue.blocking_dispatch(&mut self.state).is_err() {
                break;
            }
        }
        // Destroyed when served once, which empties the selection, as
        // wl-copy leaving does, and when cancelled, as the protocol asks.
        source.destroy();
        let _ = self.connection.flush();
    }
}

/// A copy being served.
struct Serving {
    bytes: Arc<[u8]>,
    once: bool,
    served: bool,
    cancelled: bool,
}

/// What the compositor has said on one connection.
#[derive(Default)]
struct State {
    offers: HashMap<ObjectId, Vec<String>>,
    selection: Option<ExtDataControlOfferV1>,
    primary: Option<ExtDataControlOfferV1>,
    serving: Option<Serving>,
    finished: bool,
}

impl State {
    /// Whether a copy is still to be served: not replaced, not served
    /// already when it's served once, and the compositor still there.
    fn still_serving(&self) -> bool {
        self.serving.as_ref().is_some_and(|serving| {
            !(serving.cancelled || serving.once && serving.served || self.finished)
        })
    }
}

/// What a program hands over, up to `largest` bytes, within
/// [`PATIENCE`]: too large past it, unanswered when it takes longer.
fn read_in_time(mut reader: io::PipeReader, largest: usize) -> Result<Vec<u8>, Why> {
    let deadline = Instant::now() + PATIENCE;
    let mut bytes = Vec::new();
    let mut chunk = vec![0; CHUNK];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let mut ready = libc::pollfd {
            fd: reader.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let wait = libc::c_int::try_from(left.as_millis()).unwrap_or(libc::c_int::MAX);
        // SAFETY: one pollfd, alive for the call, naming a pipe open for
        // as long as `reader` is.
        let polled = unsafe { libc::poll(&raw mut ready, 1, wait) };
        if polled == 0 {
            return Err(Why::Unanswered);
        }
        if polled < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(Why::Unanswered);
        }
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(bytes),
            Ok(read) => {
                bytes.extend_from_slice(&chunk[..read]);
                if bytes.len() > largest {
                    return Err(Why::TooLarge);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return Err(Why::Unanswered),
        }
    }
}

impl Dispatch<ExtDataControlSourceV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(serving) = state.serving.as_mut() else {
            return;
        };
        match event {
            // Written on a thread of its own, so a program slow to read
            // never holds up the next.
            ext_data_control_source_v1::Event::Send { fd, .. } => {
                if serving.once && serving.served {
                    return;
                }
                serving.served = true;
                let bytes = Arc::clone(&serving.bytes);
                thread::spawn(move || {
                    let _ = std::fs::File::from(fd).write_all(&bytes);
                });
            }
            ext_data_control_source_v1::Event::Cancelled => serving.cancelled = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtDataControlDeviceV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => {
                state.offers.insert(id.id(), Vec::new());
            }
            ext_data_control_device_v1::Event::Selection { id } => {
                if let Some(before) = std::mem::replace(&mut state.selection, id) {
                    before.destroy();
                }
            }
            ext_data_control_device_v1::Event::PrimarySelection { id } => {
                if let Some(before) = std::mem::replace(&mut state.primary, id) {
                    before.destroy();
                }
            }
            ext_data_control_device_v1::Event::Finished => state.finished = true,
            _ => {}
        }
    }

    event_created_child!(State, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ExtDataControlOfferV1, ()> for State {
    fn event(
        state: &mut Self,
        offer: &ExtDataControlOfferV1,
        event: ext_data_control_offer_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
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

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtDataControlManagerV1,
        _: <ExtDataControlManagerV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_handed_over_in_time_is_read_whole() {
        let (reader, mut writer) = io::pipe().unwrap();
        writer.write_all(b"hello").unwrap();
        drop(writer);
        assert_eq!(read_in_time(reader, 5).unwrap(), b"hello");
    }

    #[test]
    fn more_than_is_carried_is_too_large() {
        let (reader, mut writer) = io::pipe().unwrap();
        writer.write_all(b"hello").unwrap();
        drop(writer);
        assert_eq!(read_in_time(reader, 4).unwrap_err(), Why::TooLarge);
    }

    #[test]
    fn with_no_display_nothing_is_reached() {
        // SAFETY: tests touching the environment run in this one only.
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", "/nonexistent/wayland-none");
            std::env::remove_var("WAYLAND_SOCKET");
        }
        assert_eq!(types(Selection::Clipboard).unwrap_err(), Why::NoDisplay);
    }
}
