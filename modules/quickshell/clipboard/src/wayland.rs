//! What the compositor says over its data-control protocol: each offer's
//! types, each new copy read and kept, the copy this program offers read
//! as it's asked for, and the highlighted text held to be read on a get.

use std::fs::File;
use std::io::{self, PipeReader, Read, Write};
use std::os::fd::AsFd;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::history::{Content, Heard, offered};
use crate::{OURS, Readable, State, now, readable};
use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, event_created_child};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::{self, ExtDataControlOfferV1},
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

/// How long a program has to hand over what it copied before it's skipped.
const PATIENCE: Duration = Duration::from_secs(2);

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardManagerV1,
        _: <ZwpVirtualKeyboardManagerV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardV1,
        _: <ZwpVirtualKeyboardV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlSourceV1, ()> for State {
    fn event(
        state: &mut Self,
        source: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            // Read when asked, so a history forgotten since gives nothing.
            ext_data_control_source_v1::Event::Send { fd, .. } => {
                let content = state.history.current.clone();
                thread::spawn(move || {
                    let mut file = File::from(fd);
                    match content {
                        Some(Content::Text(text)) => {
                            let _ = file.write_all(text.as_bytes());
                        }
                        Some(Content::Picture(picture)) => {
                            if let Ok(mut kept) = File::open(&picture.path) {
                                let _ = io::copy(&mut kept, &mut file);
                            }
                        }
                        None => {}
                    }
                });
            }
            // Something else was copied, which holds the clipboard now.
            ext_data_control_source_v1::Event::Cancelled => {
                source.destroy();
                if state.held.as_ref() == Some(source) {
                    state.held = None;
                }
            }
            _ => {}
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

impl Dispatch<ExtDataControlDeviceV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        (): &(),
        connection: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => {
                state.offers.insert(id.id(), Vec::new());
            }
            ext_data_control_device_v1::Event::Selection { id: Some(offer) } => {
                state.emptied = false;
                let types = state.offers.remove(&offer.id()).unwrap_or_default();
                // Its own offer, heard back: the clipboard holds what it did.
                if types.iter().any(|kind| kind == OURS) {
                    offer.destroy();
                    return;
                }
                let heard = match readable(&types, state.pictures) {
                    Some(Readable::Text(kind)) => receive(connection, &offer, kind, state.largest)
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .map(Heard::Text),
                    Some(Readable::Picture(kind)) => {
                        receive(connection, &offer, kind, state.largest_picture)
                            .map(|bytes| Heard::Picture { kind, bytes })
                    }
                    None => None,
                };
                offer.destroy();
                let now = offered(&types, heard.as_ref());
                state.history.copied(heard);
                state.history.holds(now);
            }
            // The program holding the clipboard let go of it: what it held
            // is offered once the events after this are read (run).
            ext_data_control_device_v1::Event::Selection { id: None } => {
                state.emptied = true;
            }
            // The selection a middle click pastes isn't kept, only held to
            // be read as a get asks for it.
            ext_data_control_device_v1::Event::PrimarySelection { id } => {
                let held = id.map(|offer| {
                    let types = state.offers.remove(&offer.id()).unwrap_or_default();
                    (offer, types)
                });
                if let Some((before, _)) = std::mem::replace(&mut state.primary, held) {
                    before.destroy();
                }
                state.primary_at = now();
            }
            ext_data_control_device_v1::Event::Finished => {
                state.taken = true;
            }
            _ => {}
        }
    }

    event_created_child!(State, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

/// Reads an offer as a type, or nothing when it's larger than `largest` or
/// its program doesn't hand it over in time.
fn receive(
    connection: &Connection,
    offer: &ExtDataControlOfferV1,
    kind: &str,
    largest: usize,
) -> Option<Vec<u8>> {
    let bytes = read_in_time(asked_for(connection, offer, kind)?, largest)?;
    (bytes.len() <= largest).then_some(bytes)
}

/// Asks an offer's program for it as a type, and gives where it's read.
pub(crate) fn asked_for(
    connection: &Connection,
    offer: &ExtDataControlOfferV1,
    kind: &str,
) -> Option<PipeReader> {
    let (reader, writer) = io::pipe().ok()?;
    offer.receive(kind.to_string(), writer.as_fd());
    connection.flush().ok()?;
    // The program writes into its own copy of the pipe; once it closes that,
    // the read ends.
    drop(writer);
    Some(reader)
}

/// What a program hands over, up to a byte past `largest`, or nothing when
/// it doesn't in time.
pub(crate) fn read_in_time(reader: PipeReader, largest: usize) -> Option<Vec<u8>> {
    let (sent, heard) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = reader.take(largest as u64 + 1).read_to_end(&mut bytes);
        let _ = sent.send(read.map(|_| bytes));
    });
    heard.recv_timeout(PATIENCE).ok()?.ok()
}
