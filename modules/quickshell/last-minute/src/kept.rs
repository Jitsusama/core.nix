//! What's kept of the stream: MPEG transport stream packets in chunks, each
//! beginning at one of the video's keyframes, the oldest the last to begin
//! at or before the span ago. So what's kept always plays from its start and
//! holds at least the span, and a chunk is forgotten whole once the one
//! after it is old enough to start from.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Every transport stream packet is this long, and opens with the sync byte.
pub const PACKET: usize = 188;
pub const SYNC: u8 = 0x47;

/// The program association table's own packet identifier, which says where
/// the program map table is.
const ASSOCIATION: u16 = 0;

/// The stream types a program map table gives video: MPEG-1 and 2, MPEG-4
/// part 2, H.264 and H.265.
const VIDEO: [u8; 5] = [0x01, 0x02, 0x10, 0x1b, 0x24];

struct Chunk {
    began: Instant,
    bytes: Vec<u8>,
}

pub struct Kept {
    span: Duration,
    // The newest of each table, so a saved stream opens saying what it
    // holds whichever chunk it starts from.
    association: Option<Vec<u8>>,
    map: Option<Vec<u8>>,
    map_pid: Option<u16>,
    video: Option<u16>,
    chunks: VecDeque<Chunk>,
}

impl Kept {
    pub fn new(span: Duration) -> Self {
        Self {
            span,
            association: None,
            map: None,
            map_pid: None,
            video: None,
            chunks: VecDeque::new(),
        }
    }

    /// Takes one packet, heard at `at`. Packets before the first keyframe
    /// are let go, since nothing could play them.
    pub fn take(&mut self, packet: &[u8], at: Instant) {
        let Some(header) = Header::read(packet) else {
            return;
        };
        if header.starts {
            let section = header.section(packet);
            if header.pid == ASSOCIATION {
                if let Some(pid) = section.and_then(program_map) {
                    self.map_pid = Some(pid);
                    self.association = Some(packet.to_vec());
                }
            } else if Some(header.pid) == self.map_pid
                && let Some(pid) = section.and_then(video)
            {
                self.video = Some(pid);
                self.map = Some(packet.to_vec());
            }
        }
        if Some(header.pid) == self.video && header.starts && header.random_access {
            self.chunks.push_back(Chunk {
                began: at,
                bytes: Vec::new(),
            });
            self.forget(at);
        }
        if let Some(chunk) = self.chunks.back_mut() {
            chunk.bytes.extend_from_slice(packet);
        }
    }

    fn forget(&mut self, at: Instant) {
        let Some(since) = at.checked_sub(self.span) else {
            return;
        };
        while self.chunks.get(1).is_some_and(|next| next.began <= since) {
            self.chunks.pop_front();
        }
    }

    /// What's kept as one stream that plays from its start: the tables that
    /// say what it holds, then every chunk; or nothing, before the first
    /// keyframe.
    pub fn stream(&self) -> Option<Vec<u8>> {
        let (Some(association), Some(map)) = (&self.association, &self.map) else {
            return None;
        };
        if self.chunks.is_empty() {
            return None;
        }
        let length = self
            .chunks
            .iter()
            .map(|chunk| chunk.bytes.len())
            .sum::<usize>();
        let mut bytes = Vec::with_capacity(2 * PACKET + length);
        bytes.extend_from_slice(association);
        bytes.extend_from_slice(map);
        for chunk in &self.chunks {
            bytes.extend_from_slice(&chunk.bytes);
        }
        Some(bytes)
    }
}

/// What a packet's header says, and where its payload begins.
struct Header {
    pid: u16,
    starts: bool,
    random_access: bool,
    payload: usize,
}

impl Header {
    fn read(packet: &[u8]) -> Option<Self> {
        if packet.len() != PACKET || packet[0] != SYNC {
            return None;
        }
        let starts = packet[1] & 0x40 != 0;
        let pid = (u16::from(packet[1] & 0x1f) << 8) | u16::from(packet[2]);
        let control = (packet[3] >> 4) & 0x3;
        let adapted = control & 0x2 != 0;
        let length = if adapted { usize::from(packet[4]) } else { 0 };
        // The adaptation field's flags follow its length, the random
        // access indicator among them, which the muxer sets on a keyframe.
        let random_access = adapted && length > 0 && packet[5] & 0x40 != 0;
        let payload = if control & 0x1 == 0 {
            PACKET
        } else if adapted {
            5 + length
        } else {
            4
        };
        Some(Self {
            pid,
            starts,
            random_access,
            payload: payload.min(PACKET),
        })
    }

    /// The table section a packet starting one holds, past its pointer.
    fn section<'a>(&self, packet: &'a [u8]) -> Option<&'a [u8]> {
        let payload = packet.get(self.payload..)?;
        let pointer = usize::from(*payload.first()?);
        payload.get(1 + pointer..)
    }
}

/// A section's entries: past its first `head` bytes and up to its checksum.
fn entries(section: &[u8], table: u8, head: usize) -> Option<&[u8]> {
    if *section.first()? != table {
        return None;
    }
    let length = (usize::from(section.get(1)? & 0x0f) << 8) | usize::from(*section.get(2)?);
    section.get(head..(3 + length).checked_sub(4)?)
}

fn pid(high: u8, low: u8) -> u16 {
    (u16::from(high & 0x1f) << 8) | u16::from(low)
}

/// Where the program association table says the first program's map is.
fn program_map(section: &[u8]) -> Option<u16> {
    entries(section, 0x00, 8)?
        .as_chunks::<4>()
        .0
        .iter()
        .find(|&&[high, low, _, _]| high != 0 || low != 0)
        .map(|&[_, _, high, low]| pid(high, low))
}

/// The video's packet identifier, as the program map table gives it.
fn video(section: &[u8]) -> Option<u16> {
    let info = (usize::from(section.get(10)? & 0x0f) << 8) | usize::from(*section.get(11)?);
    let mut streams = entries(section, 0x02, 12 + info)?;
    while streams.len() >= 5 {
        let described = (usize::from(streams[3] & 0x0f) << 8) | usize::from(streams[4]);
        if VIDEO.contains(&streams[0]) {
            return Some(pid(streams[1], streams[2]));
        }
        streams = streams.get(5 + described..)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: u16 = 0x1000;
    const PICTURES: u16 = 0x100;
    const SOUND: u16 = 0x101;

    /// A packet starting a table section, padded as a muxer pads one.
    fn table(pid: u16, section: &[u8]) -> Vec<u8> {
        let [high, low] = pid.to_be_bytes();
        let mut packet = vec![SYNC, 0x40 | high, low, 0x10, 0];
        packet.extend_from_slice(section);
        packet.resize(PACKET, 0xff);
        packet
    }

    /// A section with its length filled in and room for a checksum.
    fn sectioned(table: u8, body: &[u8]) -> Vec<u8> {
        let length = u16::try_from(body.len() + 4).expect("a section too long");
        let [high, low] = length.to_be_bytes();
        let mut section = vec![table, 0xb0 | high, low];
        section.extend_from_slice(body);
        section.extend_from_slice(&[0; 4]);
        section
    }

    fn association() -> Vec<u8> {
        let [high, low] = MAP.to_be_bytes();
        let body = [0, 1, 0xc1, 0, 0, 0, 1, 0xe0 | high, low];
        table(ASSOCIATION, &sectioned(0x00, &body))
    }

    /// A map whose sound comes before its pictures, so the video is found
    /// by its type rather than its place.
    fn map() -> Vec<u8> {
        let body = [
            0, 1, 0xc1, 0, 0, // the program, its version and section numbers
            0xe1, 0x00, 0xf0, 0, // the clock's packets, and no program info
            0x0f, 0xe1, 0x01, 0xf0, 0, // AAC sound
            0x1b, 0xe1, 0x00, 0xf0, 0, // H.264 pictures
        ];
        table(MAP, &sectioned(0x02, &body))
    }

    /// A packet of a stream's, starting a frame or carrying one on, said to
    /// be where playing can start or not.
    fn frame(pid: u16, starts: bool, key: bool) -> Vec<u8> {
        let start = if starts { 0x40 } else { 0 };
        let flags = if key { 0x40 } else { 0 };
        let [high, low] = pid.to_be_bytes();
        let mut packet = vec![SYNC, start | high, low, 0x30, 1, flags];
        packet.resize(PACKET, 0xaa);
        packet
    }

    fn heard(kept: &mut Kept, at: Instant, packets: &[Vec<u8>]) {
        for packet in packets {
            kept.take(packet, at);
        }
    }

    /// How many keyframes a stream holds, and so where it may begin.
    fn keyframes(stream: &[u8]) -> usize {
        stream
            .chunks(PACKET)
            .filter(|packet| packet[1] & 0x40 != 0 && packet[5] & 0x40 != 0)
            .filter(|packet| pid(packet[1], packet[2]) == PICTURES)
            .count()
    }

    #[test]
    fn nothing_is_kept_before_the_tables_and_a_keyframe() {
        let now = Instant::now();
        let mut kept = Kept::new(Duration::from_secs(60));
        heard(&mut kept, now, &[frame(PICTURES, true, true)]);
        assert!(
            kept.stream().is_none(),
            "kept a frame before knowing it was video"
        );
        heard(
            &mut kept,
            now,
            &[association(), map(), frame(PICTURES, true, false)],
        );
        assert!(kept.stream().is_none(), "kept frames before a keyframe");
    }

    #[test]
    fn what_is_kept_opens_with_its_tables_then_a_keyframe() {
        let now = Instant::now();
        let mut kept = Kept::new(Duration::from_secs(60));
        heard(
            &mut kept,
            now,
            &[association(), map(), frame(PICTURES, false, false)],
        );
        heard(
            &mut kept,
            now,
            &[frame(PICTURES, true, true), frame(SOUND, true, true)],
        );
        let stream = kept.stream().expect("nothing kept after a keyframe");
        assert_eq!(&stream[..PACKET], association().as_slice());
        assert_eq!(&stream[PACKET..2 * PACKET], map().as_slice());
        assert_eq!(
            &stream[2 * PACKET..3 * PACKET],
            frame(PICTURES, true, true).as_slice()
        );
        assert_eq!(
            stream.len(),
            4 * PACKET,
            "kept what came before the keyframe"
        );
    }

    #[test]
    fn the_sounds_own_starts_never_start_a_chunk() {
        let now = Instant::now();
        let mut kept = Kept::new(Duration::from_secs(60));
        heard(
            &mut kept,
            now,
            &[association(), map(), frame(SOUND, true, true)],
        );
        assert!(kept.stream().is_none(), "sound was taken for a keyframe");
    }

    #[test]
    fn it_keeps_from_the_last_keyframe_at_or_before_a_minute_ago() {
        let start = Instant::now();
        let at = |seconds| start + Duration::from_secs(seconds);
        let mut kept = Kept::new(Duration::from_secs(60));
        heard(&mut kept, at(0), &[association(), map()]);
        for seconds in [0, 30, 61, 90] {
            heard(
                &mut kept,
                at(seconds),
                &[frame(PICTURES, true, true), frame(SOUND, true, true)],
            );
        }
        // At 90 s, the minute began at 30 s: that keyframe's chunk stays,
        // the one at 0 s goes.
        let stream = kept.stream().expect("nothing kept");
        assert_eq!(keyframes(&stream), 3);
        assert_eq!(stream.len(), 2 * PACKET + 3 * 2 * PACKET);
    }
}
