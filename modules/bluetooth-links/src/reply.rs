//! What goes to the controller and what comes back, as bytes, in pure
//! functions a test can hand fixtures to: the kernel's lists of adapters and
//! connections, an HCI command, the event that answers it, and each of the
//! four answers read into a figure. Nothing here touches a socket.
//!
//! The layouts are the Bluetooth Core Specification's (volume 4, part E)
//! and the kernel's `include/net/bluetooth/hci_sock.h`, every field
//! little-endian.

/// A command packet's first byte, and an event packet's.
const COMMAND_PACKET: u8 = 0x01;
const EVENT_PACKET: u8 = 0x04;
/// The events that answer a command: done, with its return parameters, or
/// taken up or refused, with only a status.
pub const COMMAND_COMPLETE: u8 = 0x0e;
pub const COMMAND_STATUS: u8 = 0x0f;

/// An opcode is its group (OGF) in the top six bits and its command (OCF)
/// in the rest.
const fn opcode(group: u16, command: u16) -> u16 {
    (group << 10) | command
}

/// Read RSSI, Read Link Quality and Read AFH Channel Map are status
/// commands; Read Transmit Power Level is the controller's. The kernel lets
/// a socket with no capability send all four.
pub const READ_RSSI: u16 = opcode(0x05, 0x0005);
pub const READ_LINK_QUALITY: u16 = opcode(0x05, 0x0003);
pub const READ_AFH_MAP: u16 = opcode(0x05, 0x0006);
pub const READ_TRANSMIT_POWER: u16 = opcode(0x03, 0x002d);

/// The kernel's names for a connection's kind and its state.
const ACL_LINK: u8 = 0x01;
const LE_LINK: u8 = 0x80;
const CONNECTED: u16 = 1;

/// The classic radio's channels, each a megahertz apart from 2402 MHz.
pub const CHANNELS: usize = 79;

/// How many adapters and connections a list has room for. A laptop has one
/// adapter and a handful of links; past these the rest go unread.
pub const MOST_ADAPTERS: usize = 16;
pub const MOST_CONNECTIONS: usize = 32;

/// The sizes of the kernel's structures: a list's header, an adapter's
/// entry (an id and its options, padded to four bytes) and a connection's.
const LIST_HEADER: usize = 4;
const ADAPTER_ENTRY: usize = 8;
const CONNECTION_ENTRY: usize = 16;

pub const ADAPTER_LIST: usize = LIST_HEADER + MOST_ADAPTERS * ADAPTER_ENTRY;
pub const CONNECTION_LIST: usize = LIST_HEADER + MOST_CONNECTIONS * CONNECTION_ENTRY;

/// One connection the kernel lists: the handle commands name it by, the
/// device at its other end, and whether it's Low Energy rather than the
/// classic radio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub handle: u16,
    pub address: String,
    pub le: bool,
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]))
}

/// An empty list of adapters for `HCIGETDEVLIST` to fill: how many it has
/// room for, first.
pub fn adapter_request() -> [u8; ADAPTER_LIST] {
    let mut request = [0; ADAPTER_LIST];
    request[..2].copy_from_slice(&count(MOST_ADAPTERS).to_le_bytes());
    request
}

/// An empty list of one adapter's connections for `HCIGETCONNLIST` to
/// fill: the adapter, then how many it has room for.
pub fn connection_request(adapter: u16) -> [u8; CONNECTION_LIST] {
    let mut request = [0; CONNECTION_LIST];
    request[..2].copy_from_slice(&adapter.to_le_bytes());
    request[2..4].copy_from_slice(&count(MOST_CONNECTIONS).to_le_bytes());
    request
}

fn count(most: usize) -> u16 {
    u16::try_from(most).unwrap_or(u16::MAX)
}

/// The adapters' ids from a filled list.
pub fn adapters(list: &[u8]) -> Vec<u16> {
    let listed = usize::from(u16_at(list, 0).unwrap_or(0)).min(MOST_ADAPTERS);
    (0..listed)
        .filter_map(|n| u16_at(list, LIST_HEADER + n * ADAPTER_ENTRY))
        .collect()
}

/// The connections from a filled list that carry data and are up: classic
/// ACL links and Low Energy ones, but not a headset's voice (SCO) links,
/// which ride an ACL link already listed, nor one still being made.
pub fn connections(list: &[u8]) -> Vec<Connection> {
    let listed = usize::from(u16_at(list, 2).unwrap_or(0)).min(MOST_CONNECTIONS);
    (0..listed)
        .filter_map(|n| {
            let entry = list.get(LIST_HEADER + n * CONNECTION_ENTRY..)?;
            let entry = entry.get(..CONNECTION_ENTRY)?;
            let kind = entry[8];
            let up = u16_at(entry, 10)? == CONNECTED;
            let handle = u16_at(entry, 0)?;
            (up && (kind == ACL_LINK || kind == LE_LINK)).then(|| Connection {
                handle,
                address: address(&entry[2..8]),
                le: kind == LE_LINK,
            })
        })
        .collect()
}

/// A device's address as `BlueZ` writes it, the kernel keeping its bytes
/// the other way round.
pub fn address(bytes: &[u8]) -> String {
    bytes
        .iter()
        .rev()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// A command as it's written to the socket.
pub fn command(opcode: u16, parameters: &[u8]) -> Vec<u8> {
    let mut packet = vec![COMMAND_PACKET];
    packet.extend_from_slice(&opcode.to_le_bytes());
    packet.push(u8::try_from(parameters.len()).unwrap_or(u8::MAX));
    packet.extend_from_slice(parameters);
    packet
}

/// The parameters of each command: the connection's handle, and for the
/// transmit power, which of its levels (0, as it is now).
pub fn asking(opcode: u16, handle: u16) -> Vec<u8> {
    let mut parameters = handle.to_le_bytes().to_vec();
    if opcode == READ_TRANSMIT_POWER {
        parameters.push(0);
    }
    parameters
}

/// The socket's filter, as `HCI_FILTER` takes it: event packets only, and
/// of those only the two that answer a command. Every command's answer
/// comes through it, so `answer` picks out the one asked for.
pub fn filter() -> [u8; 16] {
    let mut filter = [0; 16];
    let packets: u32 = 1 << EVENT_PACKET;
    let events: u32 = (1 << COMMAND_COMPLETE) | (1 << COMMAND_STATUS);
    filter[..4].copy_from_slice(&packets.to_le_bytes());
    filter[4..8].copy_from_slice(&events.to_le_bytes());
    filter
}

/// What an event packet says of the command asked.
#[derive(Debug, PartialEq, Eq)]
pub enum Heard<'a> {
    /// It's done, and these are its return parameters, status first.
    Done(&'a [u8]),
    /// The controller wouldn't do it, saying why by this status.
    Refused(u8),
    /// Something else: another command's answer, or this one taken up,
    /// its answer still to come.
    Other,
}

/// Reads an event packet for the answer to `opcode`.
pub fn answer(packet: &[u8], opcode: u16) -> Heard<'_> {
    let [EVENT_PACKET, event, length, body @ ..] = packet else {
        return Heard::Other;
    };
    let Some(body) = body.get(..usize::from(*length)) else {
        return Heard::Other;
    };
    match *event {
        COMMAND_COMPLETE if u16_at(body, 1) == Some(opcode) => Heard::Done(&body[3..]),
        COMMAND_STATUS if u16_at(body, 2) == Some(opcode) && body[0] != 0 => {
            Heard::Refused(body[0])
        }
        _ => Heard::Other,
    }
}

/// A command's return parameters past its status and handle, when it
/// succeeded for this connection.
fn succeeded(parameters: &[u8], handle: u16) -> Option<&[u8]> {
    let [0, _, _, rest @ ..] = parameters else {
        return None;
    };
    ((u16_at(parameters, 1)? & 0x0fff) == handle).then_some(rest)
}

/// What Read RSSI says, in dB: on a classic link, how far the signal is
/// above (positive) or below (negative) the range its radio hears best,
/// 0 inside it; on a Low Energy link, the signal itself in dBm.
pub fn rssi(parameters: &[u8], handle: u16) -> Option<i8> {
    succeeded(parameters, handle)?
        .first()
        .map(|&byte| i8::from_le_bytes([byte]))
}

/// What Read Link Quality says: how well the link carries, from 0 to 255,
/// as the controller's maker judges it.
pub fn quality(parameters: &[u8], handle: u16) -> Option<u8> {
    succeeded(parameters, handle)?.first().copied()
}

/// What Read Transmit Power Level says: how loud this end sends, in dBm.
pub fn power(parameters: &[u8], handle: u16) -> Option<i8> {
    rssi(parameters, handle)
}

/// What Read AFH Channel Map says: which of the 79 channels the link hops
/// over, as 1, and which it skips as noisy, as 0, channel 0 first. With
/// adaptive hopping off, it hops over them all.
pub fn channel_map(parameters: &[u8], handle: u16) -> Option<String> {
    let [mode, map @ ..] = succeeded(parameters, handle)? else {
        return None;
    };
    let map = map.get(..CHANNELS.div_ceil(8))?;
    Some(
        (0..CHANNELS)
            .map(|channel| {
                let hops = *mode == 0 || map[channel / 8] & (1 << (channel % 8)) != 0;
                if hops { '1' } else { '0' }
            })
            .collect(),
    )
}

/// How many channels a map hops over.
pub fn in_use(map: &str) -> usize {
    map.chars().filter(|&channel| channel == '1').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A connection's entry as the kernel lays it out: its handle, its
    /// address backwards, its kind, whether this end made it, its state and
    /// its link mode.
    fn entry(handle: u16, address: [u8; 6], kind: u8, state: u16) -> Vec<u8> {
        let mut entry = handle.to_le_bytes().to_vec();
        entry.extend_from_slice(&address);
        entry.extend_from_slice(&[kind, 1]);
        entry.extend_from_slice(&state.to_le_bytes());
        entry.extend_from_slice(&0u32.to_le_bytes());
        entry
    }

    fn listed(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut list = connection_request(0).to_vec();
        list[2..4].copy_from_slice(&u16::try_from(entries.len()).unwrap().to_le_bytes());
        for (n, entry) in entries.iter().enumerate() {
            let at = LIST_HEADER + n * CONNECTION_ENTRY;
            list[at..at + CONNECTION_ENTRY].copy_from_slice(entry);
        }
        list
    }

    #[test]
    fn the_adapters_are_read_from_their_list() {
        let mut list = adapter_request();
        assert_eq!(
            u16_at(&list, 0),
            Some(16),
            "it says how many it has room for"
        );
        list[..2].copy_from_slice(&2u16.to_le_bytes());
        list[4..6].copy_from_slice(&0u16.to_le_bytes());
        list[12..14].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(adapters(&list), vec![0, 3]);
    }

    #[test]
    fn a_list_saying_more_than_it_holds_is_read_as_far_as_it_goes() {
        let mut list = [0u8; 12];
        list[..2].copy_from_slice(&5u16.to_le_bytes());
        list[4..6].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(adapters(&list), vec![1]);
    }

    #[test]
    fn the_connections_asked_for_name_their_adapter_and_room() {
        let request = connection_request(2);
        assert_eq!(u16_at(&request, 0), Some(2));
        assert_eq!(u16_at(&request, 2), Some(32));
    }

    #[test]
    fn only_data_links_that_are_up_are_kept() {
        let headphones = [0xcc, 0xbb, 0xaa, 0x66, 0x1b, 0x00];
        let mouse = [0x6e, 0xf3, 0x2a, 0xe3, 0x9c, 0xc3];
        let list = listed(&[
            entry(0x000b, headphones, ACL_LINK, CONNECTED),
            // The headset's voice link beside it.
            entry(0x000c, headphones, 0x02, CONNECTED),
            entry(0x0040, mouse, LE_LINK, CONNECTED),
            // One still being made.
            entry(0x0041, mouse, LE_LINK, 2),
        ]);
        assert_eq!(
            connections(&list),
            vec![
                Connection {
                    handle: 0x000b,
                    address: "00:1B:66:AA:BB:CC".into(),
                    le: false
                },
                Connection {
                    handle: 0x0040,
                    address: "C3:9C:E3:2A:F3:6E".into(),
                    le: true
                },
            ]
        );
    }

    #[test]
    fn an_empty_list_has_no_connections() {
        assert!(connections(&connection_request(0)).is_empty());
        assert!(connections(&[]).is_empty());
    }

    #[test]
    fn a_command_is_its_packet_type_opcode_length_and_parameters() {
        assert_eq!(
            command(READ_RSSI, &asking(READ_RSSI, 0x000b)),
            [0x01, 0x05, 0x14, 2, 0x0b, 0]
        );
        assert_eq!(
            command(READ_TRANSMIT_POWER, &asking(READ_TRANSMIT_POWER, 0x000b)),
            [0x01, 0x2d, 0x0c, 3, 0x0b, 0, 0]
        );
        assert_eq!(READ_LINK_QUALITY, 0x1403);
        assert_eq!(READ_AFH_MAP, 0x1406);
    }

    #[test]
    fn the_filter_lets_through_only_what_answers_a_command() {
        let filter = filter();
        assert_eq!(filter[..4], 0x10u32.to_le_bytes());
        assert_eq!(filter[4..8], 0xc000u32.to_le_bytes());
        assert_eq!(filter[8..], [0; 8]);
    }

    #[test]
    fn a_completed_command_gives_its_return_parameters() {
        // Command Complete for Read RSSI on handle 0x000b: -6 dB.
        let packet = [0x04, 0x0e, 0x07, 0x01, 0x05, 0x14, 0x00, 0x0b, 0x00, 0xfa];
        assert_eq!(
            answer(&packet, READ_RSSI),
            Heard::Done(&[0x00, 0x0b, 0x00, 0xfa])
        );
        assert_eq!(
            answer(&packet, READ_LINK_QUALITY),
            Heard::Other,
            "another command's"
        );
    }

    #[test]
    fn a_refused_command_says_why() {
        // Command Status: unknown connection, for Read AFH Channel Map.
        let refused = [0x04, 0x0f, 0x04, 0x02, 0x01, 0x06, 0x14];
        assert_eq!(answer(&refused, READ_AFH_MAP), Heard::Refused(0x02));
        // A status of 0 only says it was taken up.
        let taken = [0x04, 0x0f, 0x04, 0x00, 0x01, 0x06, 0x14];
        assert_eq!(answer(&taken, READ_AFH_MAP), Heard::Other);
    }

    #[test]
    fn a_cut_short_or_foreign_packet_says_nothing() {
        assert_eq!(answer(&[0x04, 0x0e, 0x07, 0x01], READ_RSSI), Heard::Other);
        assert_eq!(answer(&[0x02, 0x0e, 0x00], READ_RSSI), Heard::Other);
        assert_eq!(answer(&[], READ_RSSI), Heard::Other);
    }

    #[test]
    fn the_signal_is_read_signed() {
        assert_eq!(rssi(&[0x00, 0x0b, 0x00, 0xfa], 0x000b), Some(-6));
        assert_eq!(rssi(&[0x00, 0x40, 0x00, 0xc6], 0x0040), Some(-58));
        assert_eq!(rssi(&[0x00, 0x0b, 0x00, 0x04], 0x000b), Some(4));
    }

    #[test]
    fn a_failed_or_misdirected_answer_is_no_figure() {
        // Unknown connection.
        assert_eq!(rssi(&[0x02, 0x0b, 0x00, 0x00], 0x000b), None);
        assert_eq!(
            rssi(&[0x00, 0x0c, 0x00, 0xfa], 0x000b),
            None,
            "another link's"
        );
        assert_eq!(rssi(&[0x00, 0x0b], 0x000b), None, "cut short");
        assert_eq!(quality(&[], 0x000b), None);
    }

    #[test]
    fn a_handle_is_read_without_its_flags() {
        // The top four bits of a handle field are flags, not the handle.
        assert_eq!(quality(&[0x00, 0x0b, 0x20, 0xe6], 0x000b), Some(230));
    }

    #[test]
    fn the_link_quality_and_power_are_read() {
        assert_eq!(quality(&[0x00, 0x0b, 0x00, 0xff], 0x000b), Some(255));
        assert_eq!(power(&[0x00, 0x0b, 0x00, 0x04], 0x000b), Some(4));
        assert_eq!(power(&[0x00, 0x0b, 0x00, 0xf6], 0x000b), Some(-10));
    }

    #[test]
    fn the_channel_map_is_read_channel_0_first() {
        // Hopping over all but channels 22 to 39, where a Wi-Fi network on
        // its channel 6 sits, and with the 80th bit, which is reserved, set.
        let mut map = [0xffu8; 10];
        for channel in 22..40 {
            map[channel / 8] &= !(1 << (channel % 8));
        }
        let mut parameters = vec![0x00, 0x0b, 0x00, 0x01];
        parameters.extend_from_slice(&map);
        let read = channel_map(&parameters, 0x000b).unwrap();
        assert_eq!(read.len(), CHANNELS);
        assert_eq!(&read[..22], "1".repeat(22));
        assert_eq!(&read[22..40], "0".repeat(18));
        assert_eq!(&read[40..], "1".repeat(39));
        assert_eq!(in_use(&read), 61);
    }

    #[test]
    fn with_adaptive_hopping_off_every_channel_is_used() {
        let mut parameters = vec![0x00, 0x0b, 0x00, 0x00];
        parameters.extend_from_slice(&[0; 10]);
        assert_eq!(in_use(&channel_map(&parameters, 0x000b).unwrap()), CHANNELS);
    }

    #[test]
    fn a_map_cut_short_is_no_map() {
        assert_eq!(
            channel_map(&[0x00, 0x0b, 0x00, 0x01, 0xff, 0xff], 0x000b),
            None
        );
        assert_eq!(channel_map(&[0x00, 0x0b, 0x00], 0x000b), None);
    }
}
