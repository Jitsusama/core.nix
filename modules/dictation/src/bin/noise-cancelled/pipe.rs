//! PipeWire's side: for each microphone, a source of its own named for it,
//! "‹microphone›, Noise Cancelled", and a capture of the microphone that
//! wakes only while something records from that source.
//!
//! Both streams run in PipeWire's real-time thread, where nothing may wait,
//! so they only move samples through lock-free rings to and from the
//! denoiser's thread (worker.rs). They're one node group, so the
//! microphone's clock drives both and the rings never drift. The source
//! gives silence until two frames are cleaned, a cushion against a frame
//! that cleans late, and gives silence again for any it's still short of.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::Thread;

use pipewire as pw;
use pw::context::ContextRc;
use pw::main_loop::MainLoopRc;
use pw::properties::PropertiesBox;
use pw::spa;
use pw::spa::param::audio::{AudioFormat, AudioInfoRaw, MAX_CHANNELS};
use pw::spa::pod::Pod;
use pw::spa::utils::dict::DictRef;
use pw::stream::{StreamFlags, StreamListener, StreamRc};
use pw::types::ObjectType;
use rtrb::{Consumer, Producer, RingBuffer};

use crate::denoise::{FRAME, RATE};
use crate::worker::{Ends, Shared, Worker};

/// What each source's node name starts with, so they're never taken for
/// microphones of their own.
const OURS: &str = "noise-cancelled.";

/// How many samples each ring holds: 16 frames, 160 ms.
const RING: usize = 16 * FRAME;

/// How much is cleaned before the source gives any of it: two frames.
const CUSHION: usize = 2 * FRAME;

/// A microphone as the registry names it.
struct Microphone {
    name: String,
    description: String,
}

impl Microphone {
    /// The microphone a node is, when it's a device's source and not ours.
    /// The registry gives only a few of a node's properties, and a
    /// device's nodes carry its id among them where a client's, as a
    /// filter's, carry none.
    fn of(props: &DictRef) -> Option<Self> {
        let name = props.get("node.name")?;
        let hardware = props.get("media.class") == Some("Audio/Source")
            && props.get("device.id").is_some()
            && !name.starts_with(OURS);
        hardware.then(|| Self {
            name: name.to_owned(),
            description: props
                .get("node.description")
                .or_else(|| props.get("node.nick"))
                .unwrap_or(name)
                .to_owned(),
        })
    }
}

/// Serves until PipeWire goes away.
pub fn serve(models: &Path) -> Result<(), String> {
    pw::init();
    let main_loop = MainLoopRc::new(None).map_err(|why| format!("PipeWire's loop: {why}"))?;
    let context =
        ContextRc::new(&main_loop, None).map_err(|why| format!("PipeWire's context: {why}"))?;
    let core = context
        .connect_rc(None)
        .map_err(|why| format!("reaching PipeWire: {why}"))?;
    let registry = core
        .get_registry_rc()
        .map_err(|why| format!("PipeWire's registry: {why}"))?;
    let worker = Rc::new(Worker::start(models.to_path_buf())?);
    let pairs: Rc<RefCell<HashMap<u32, Pair>>> = Rc::default();
    // Each link from a source, by its id, to the microphone whose pair
    // the source is, so its going is known by its id alone.
    let links: Rc<RefCell<HashMap<u32, u32>>> = Rc::default();

    let quit = main_loop.downgrade();
    let _core_listener = core
        .add_listener_local()
        .error(move |id, _, _, message| {
            if id == pw::core::PW_ID_CORE {
                eprintln!("noise-cancelled: PipeWire went away: {message}");
                if let Some(main_loop) = quit.upgrade() {
                    main_loop.quit();
                }
            }
        })
        .register();
    let _registry_listener = registry
        .add_listener_local()
        .global({
            let (core, worker) = (core.clone(), Rc::clone(&worker));
            let (pairs, links) = (Rc::clone(&pairs), Rc::clone(&links));
            move |global| {
                if global.type_ == ObjectType::Link {
                    let from = global
                        .props
                        .and_then(|props| props.get("link.output.node"))
                        .and_then(|node| node.parse::<u32>().ok());
                    let mut pairs = pairs.borrow_mut();
                    let found = pairs
                        .iter_mut()
                        .find(|(_, pair)| Some(pair.source.node_id()) == from);
                    if let Some((&microphone, pair)) = found {
                        links.borrow_mut().insert(global.id, microphone);
                        pair.linked();
                    }
                    return;
                }
                if global.type_ != ObjectType::Node {
                    return;
                }
                let Some(microphone) = global.props.and_then(Microphone::of) else {
                    return;
                };
                match Pair::new(&core, &worker, global.id, &microphone) {
                    Ok(pair) => {
                        eprintln!("noise-cancelled: offering {}", pair.description);
                        pairs.borrow_mut().insert(global.id, pair);
                    }
                    Err(why) => eprintln!(
                        "noise-cancelled: {} won't be offered: {why}",
                        microphone.description
                    ),
                }
            }
        })
        .global_remove(move |id| {
            if let Some(microphone) = links.borrow_mut().remove(&id) {
                if let Some(pair) = pairs.borrow_mut().get_mut(&microphone) {
                    pair.unlinked();
                }
                return;
            }
            if let Some(pair) = pairs.borrow_mut().remove(&id) {
                links.borrow_mut().retain(|_, microphone| *microphone != id);
                worker.remove(id);
                eprintln!("noise-cancelled: {} is gone", pair.description);
            }
        })
        .register();

    main_loop.run();
    Ok(())
}

/// One microphone's capture and source, and what listens to them.
struct Pair {
    description: String,
    capture: StreamRc,
    source: StreamRc,
    shared: Arc<Shared>,
    waker: Thread,
    /// How many links take what the source gives: whatever records it.
    links: usize,
    _heard: StreamListener<Capturing>,
    _given: StreamListener<Giving>,
}

/// What the capture's real-time callback holds.
struct Capturing {
    heard: Producer<f32>,
    shared: Arc<Shared>,
    waker: Thread,
}

/// What the source's real-time callback holds.
struct Giving {
    cleaned: Consumer<f32>,
    shared: Arc<Shared>,
    started: u32,
    primed: bool,
}

impl Pair {
    fn new(
        core: &pw::core::CoreRc,
        worker: &Worker,
        id: u32,
        microphone: &Microphone,
    ) -> Result<Self, String> {
        let description = format!("{}, Noise Cancelled", microphone.description);
        let group = format!("{OURS}{}", microphone.name);
        let (heard, heard_here) = RingBuffer::new(RING);
        let (cleaned_here, cleaned) = RingBuffer::new(RING);
        let shared = Arc::new(Shared::default());

        // A stream's own name is only its media's, so its node is named
        // here, or it would be the program's, the same for every pair.
        let capture_name = format!("{group}.capture");
        let capture = StreamRc::new(core.clone(), &capture_name, {
            let mut props = grouped(&group);
            props.insert(*pw::keys::NODE_NAME, capture_name.as_str());
            props.insert(*pw::keys::MEDIA_CATEGORY, "Capture");
            props.insert(*pw::keys::TARGET_OBJECT, microphone.name.as_str());
            // Gone with its microphone, rather than moved to another one.
            props.insert(*pw::keys::NODE_DONT_RECONNECT, "true");
            props
        })
        .map_err(|why| format!("its capture: {why}"))?;
        let source = StreamRc::new(core.clone(), &group, {
            let mut props = grouped(&group);
            props.insert(*pw::keys::MEDIA_CLASS, "Audio/Source");
            props.insert(*pw::keys::NODE_NAME, group.as_str());
            props.insert(*pw::keys::NODE_DESCRIPTION, description.as_str());
            props
        })
        .map_err(|why| format!("its source: {why}"))?;

        let heard_listener = capture
            .add_local_listener_with_user_data(Capturing {
                heard,
                shared: Arc::clone(&shared),
                waker: worker.waker(),
            })
            .process(hear)
            .register()
            .map_err(|why| format!("hearing its capture: {why}"))?;
        let given_listener = source
            .add_local_listener_with_user_data(Giving {
                cleaned,
                shared: Arc::clone(&shared),
                started: 0,
                primed: false,
            })
            .process(give)
            .register()
            .map_err(|why| format!("giving its source: {why}"))?;
        worker.add(
            id,
            Ends {
                heard: heard_here,
                cleaned: cleaned_here,
                shared: Arc::clone(&shared),
            },
        );
        let format = mono()?;
        let format = Pod::from_bytes(&format).ok_or("its format wouldn't serialize")?;
        capture
            .connect(
                spa::utils::Direction::Input,
                None,
                StreamFlags::AUTOCONNECT
                    | StreamFlags::MAP_BUFFERS
                    | StreamFlags::RT_PROCESS
                    | StreamFlags::INACTIVE,
                &mut [format],
            )
            .map_err(|why| format!("connecting its capture: {why}"))?;
        source
            .connect(
                spa::utils::Direction::Output,
                None,
                StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
                &mut [format],
            )
            .map_err(|why| format!("connecting its source: {why}"))?;

        Ok(Self {
            description,
            capture,
            source,
            shared,
            waker: worker.waker(),
            links: 0,
            _heard: heard_listener,
            _given: given_listener,
        })
    }

    /// Something began to record from the source.
    fn linked(&mut self) {
        self.links += 1;
        if self.links == 1 {
            self.recorded(true);
        }
    }

    /// Something stopped recording from the source.
    fn unlinked(&mut self) {
        self.links = self.links.saturating_sub(1);
        if self.links == 0 {
            self.recorded(false);
        }
    }

    /// Wakes the capture while something records from the source, and
    /// rests it otherwise, so the microphone is only heard while it's
    /// wanted. It goes by the source's links rather than its state: the
    /// two streams are one node group, so a running capture would keep
    /// the source running with nothing recording it, and never rest.
    fn recorded(&self, wanted: bool) {
        let (shared, description) = (&self.shared, &self.description);
        if wanted {
            shared.late.store(0, Ordering::Relaxed);
            shared.lost.store(0, Ordering::Relaxed);
            shared.started.fetch_add(1, Ordering::Release);
            shared.wanted.store(true, Ordering::Release);
            eprintln!("noise-cancelled: cleaning {description}");
        } else {
            shared.wanted.store(false, Ordering::Release);
            eprintln!(
                "noise-cancelled: resting {description}, {} late and {} frames lost",
                shared.late.load(Ordering::Relaxed),
                shared.lost.load(Ordering::Relaxed)
            );
        }
        if let Err(why) = self.capture.set_active(wanted) {
            eprintln!("noise-cancelled: {description}'s capture wouldn't change: {why}");
        }
        self.waker.unpark();
    }
}

/// The properties both of a microphone's streams share: one node group, so
/// one clock drives both, and one link group, so neither is ever linked to
/// the other.
fn grouped(group: &str) -> PropertiesBox {
    let mut props = PropertiesBox::new();
    props.insert(*pw::keys::MEDIA_TYPE, "Audio");
    props.insert(*pw::keys::MEDIA_ROLE, "Communication");
    props.insert(*pw::keys::NODE_GROUP, group);
    props.insert(*pw::keys::NODE_LINK_GROUP, group);
    props
}

/// The capture's real-time callback: hands what the microphone heard to the
/// denoiser's thread and wakes it.
fn hear(stream: &pw::stream::Stream, capturing: &mut Capturing) {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return;
    };
    let Some(data) = buffer.datas_mut().first_mut() else {
        return;
    };
    let chunk = data.chunk();
    let (offset, size) = (chunk.offset() as usize, chunk.size() as usize);
    let Some(bytes) = data
        .data()
        .and_then(|bytes| bytes.get(offset..offset + size))
    else {
        return;
    };
    let mut dropped = false;
    for sample in bytes.as_chunks::<4>().0 {
        dropped |= capturing.heard.push(f32::from_le_bytes(*sample)).is_err();
    }
    if dropped {
        capturing.shared.lost.fetch_add(1, Ordering::Relaxed);
    }
    capturing.waker.unpark();
}

/// The source's real-time callback: gives what's been cleaned, after the
/// cushion, and silence for whatever it's short of.
fn give(stream: &pw::stream::Stream, giving: &mut Giving) {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return;
    };
    let requested = usize::try_from(buffer.requested()).unwrap_or(usize::MAX);
    let Some(data) = buffer.datas_mut().first_mut() else {
        return;
    };
    let started = giving.shared.started.load(Ordering::Acquire);
    if started != giving.started {
        giving.started = started;
        giving.primed = false;
        let stale = giving.cleaned.slots();
        if let Ok(chunk) = giving.cleaned.read_chunk(stale) {
            chunk.commit_all();
        }
    }
    if !giving.primed && giving.cleaned.slots() >= CUSHION {
        giving.primed = true;
    }
    let mut written = 0;
    if let Some(bytes) = data.data() {
        let room = bytes.len() / 4;
        let wanted = if requested == 0 {
            room
        } else {
            requested.min(room)
        };
        for out in bytes.as_chunks_mut::<4>().0.iter_mut().take(wanted) {
            let sample = if giving.primed {
                giving.cleaned.pop().unwrap_or_else(|_| {
                    giving.primed = false;
                    giving.shared.late.fetch_add(1, Ordering::Relaxed);
                    0.0
                })
            } else {
                0.0
            };
            *out = sample.to_le_bytes();
        }
        written = wanted;
    }
    let chunk = data.chunk_mut();
    *chunk.offset_mut() = 0;
    *chunk.stride_mut() = 4;
    *chunk.size_mut() = u32::try_from(written * 4).unwrap_or(u32::MAX);
}

/// One channel of 32-bit floats at the denoiser's rate, the only format
/// either stream takes; PipeWire converts the microphone's own to it.
fn mono() -> Result<Vec<u8>, String> {
    let mut info = AudioInfoRaw::new();
    info.set_format(AudioFormat::F32LE);
    info.set_rate(RATE);
    info.set_channels(1);
    let mut position = [0; MAX_CHANNELS];
    position[0] = spa::sys::SPA_AUDIO_CHANNEL_MONO;
    info.set_position(position);
    let object = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    };
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(object),
    )
    .map(|(cursor, _)| cursor.into_inner())
    .map_err(|why| format!("its format wouldn't serialize: {why:?}"))
}
