//! A machine's own sound card, from ALSA's control device.

use std::fs::{self, File};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::sound::running;
use crate::{Facts, ask, fact, word};

/// A control device is opened without blocking, since its events are read
/// only once poll says they're there.
const O_NONBLOCK: i32 = 0o4000;

/// ALSA's control requests, each as asound.h has it.
const SNDRV_CTL_IOCTL_CARD_INFO: u64 = 0x8178_5501;
const SNDRV_CTL_IOCTL_ELEM_LIST: u64 = 0xC050_5510;
const SNDRV_CTL_IOCTL_ELEM_READ: u64 = 0xC4C8_5512;
const SNDRV_CTL_IOCTL_SUBSCRIBE_EVENTS: u64 = 0xC004_5516;
/// An element's id, `struct snd_ctl_elem_id`: its interface at 4 and its
/// name at 16, 44 bytes.
const ELEMENT_ID: usize = 64;
/// `struct snd_ctl_elem_value`, whose first value is at 72.
const ELEMENT_VALUE: usize = 1224;
/// The card's own interface, which jacks are on.
const IFACE_CARD: u32 = 0;

/// What a card's control device says of it: its jacks, each by its
/// control's name and whether something is plugged into it, the parts its
/// driver names, and its mixer's name.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Control {
    jacks: Vec<(String, bool)>,
    components: String,
    mixer: String,
}

/// The text a C string in a buffer holds, up to its first NUL.
fn c_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Every element id a card's control device has.
fn element_ids(control: &File) -> Vec<[u8; ELEMENT_ID]> {
    let mut list = [0u8; 80];
    if !ask(control, SNDRV_CTL_IOCTL_ELEM_LIST, &mut list) {
        return Vec::new();
    }
    let count = word(&list, 12);
    let mut ids = vec![[0u8; ELEMENT_ID]; count as usize];
    list[4..8].copy_from_slice(&count.to_ne_bytes());
    list[16..24].copy_from_slice(&(ids.as_mut_ptr() as usize as u64).to_ne_bytes());
    // The ids are written where the list's pointer says, which is `ids`,
    // as long as count says.
    if !ask(control, SNDRV_CTL_IOCTL_ELEM_LIST, &mut list) {
        return Vec::new();
    }
    ids.truncate(word(&list, 8) as usize);
    ids
}

/// What a card's control device says of it.
pub(crate) fn control(control: &File) -> Control {
    let mut said = Control::default();
    let mut info = [0u8; 376];
    if ask(control, SNDRV_CTL_IOCTL_CARD_INFO, &mut info) {
        said.mixer = c_text(&info[168..248]);
        said.components = c_text(&info[248..376]);
    }
    for id in element_ids(control) {
        let name = c_text(&id[16..60]);
        if word(&id, 4) != IFACE_CARD || !name.ends_with(" Jack") {
            continue;
        }
        let mut value = [0u8; ELEMENT_VALUE];
        value[..ELEMENT_ID].copy_from_slice(&id);
        if ask(control, SNDRV_CTL_IOCTL_ELEM_READ, &mut value) {
            said.jacks.push((name, word(&value, 72) != 0));
        }
    }
    said
}

/// Which side of a card a jack is on, from its control's name: an SDCA
/// codec says its output and input terminals ("OT 43", "IT 33"), and
/// others say what plugs in. A headset jack with neither is both.
fn jack_sides(name: &str) -> (bool, bool) {
    let words: Vec<&str> = name.split(' ').collect();
    if words.contains(&"OT") {
        return (true, false);
    }
    if words.contains(&"IT") {
        return (false, true);
    }
    let out = ["Headphone", "Line Out", "Speaker"]
        .iter()
        .any(|said| name.contains(said));
    let into = ["Mic", "Line In"].iter().any(|said| name.contains(said));
    let headset = name.contains("Headset");
    (out || headset, into || headset)
}

/// What a jack has plugged into it, by what its control says it takes.
fn plugged_in(name: &str) -> &'static str {
    if name.contains("pcm=") {
        "a display"
    } else if name.contains("Headset") {
        "a headset"
    } else if name.contains("Headphone") {
        "headphones"
    } else if name.contains("Mic") {
        "a microphone"
    } else {
        "a cable"
    }
}

/// What a device's jacks have plugged into them, or nothing where it has
/// none: a display's jack names its device ("HDMI/DP,pcm=5 Jack"), and a
/// headphone or microphone jack belongs to the device of that side that
/// says it's a jack or a headset's. A headset wins over what else is
/// plugged in, since it takes both.
fn jack(jacks: &[(String, bool)], device: u32, name: &str, playing: bool) -> String {
    let display = format!("pcm={device} ");
    let mine: Vec<&(String, bool)> = if jacks.iter().any(|(jack, _)| jack.contains(&display)) {
        jacks
            .iter()
            .filter(|(jack, _)| jack.contains(&display))
            .collect()
    } else if ["Jack", "Headphone", "Headset"]
        .iter()
        .any(|said| name.contains(said))
    {
        jacks
            .iter()
            .filter(|(jack, _)| !jack.contains("pcm="))
            .filter(|(jack, _)| {
                let (out, into) = jack_sides(jack);
                if playing { out } else { into }
            })
            .collect()
    } else {
        Vec::new()
    };
    if mine.is_empty() {
        return String::new();
    }
    let plugged = mine
        .iter()
        .filter(|(_, plugged)| *plugged)
        .map(|(jack, _)| plugged_in(jack))
        .min_by_key(|said| *said != "a headset");
    plugged.unwrap_or("nothing").to_owned()
}

/// A `SoundWire` device on the machine: its driver, maker and part, from
/// its name ("sdw:0:2:01fa:3557:01:2").
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Part {
    driver: String,
    maker: String,
    model: String,
}

/// Every `SoundWire` device `sys` lists.
pub(crate) fn sound_parts(sys: &Path) -> Vec<Part> {
    let Ok(entries) = fs::read_dir(sys.join("bus/soundwire/devices")) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let fields: Vec<&str> = name.split(':').collect();
        let ["sdw", _, _, maker, part, ..] = fields[..] else {
            continue;
        };
        let driver = fs::read_link(entry.path().join("driver"))
            .ok()
            .and_then(|link| Some(link.file_name()?.to_string_lossy().into_owned()))
            .unwrap_or_default();
        parts.push(Part {
            driver,
            maker: maker.to_owned(),
            model: part.to_owned(),
        });
    }
    parts.sort_by(|a, b| (&a.maker, &a.model).cmp(&(&b.maker, &b.model)));
    parts
}

/// A `SoundWire` part as its maker names it: Cirrus Logic's 3557 is the
/// CS35L57, Realtek's 0722 the RT722.
fn part_name(part: &Part) -> String {
    let digits = part.model.to_uppercase();
    match part.maker.as_str() {
        "01fa" if digits.len() == 4 => {
            format!("Cirrus Logic CS{}L{}", &digits[..2], &digits[2..])
        }
        "025d" => format!("Realtek RT{}", digits.trim_start_matches('0')),
        maker => format!("{maker}:{digits}"),
    }
}

/// The chips a card's driver names a part by ("cs35l56"), as the parts
/// themselves say, with how many there are where more than one: a driver
/// can name a family whose part is another of it, as cs35l56 drives the
/// CS35L57. A chip no `SoundWire` device is found for is said as it's
/// named.
fn chips(named: &str, parts: &[Part]) -> String {
    let found: Vec<String> = parts
        .iter()
        .filter(|part| {
            let model = part_name(part).to_lowercase();
            part.driver == named || model.ends_with(&format!(" {named}"))
        })
        .map(part_name)
        .collect();
    let Some(first) = found.first() else {
        return named.to_uppercase();
    };
    if found.len() > 1 && found.iter().all(|each| each == first) {
        format!("{} \u{d7} {first}", found.len())
    } else {
        found.join(", ")
    }
}

/// The chip behind a device, from the parts the card's driver names in its
/// components ("spk:cs35l56 hs:cs42l45 mic:cs42l45-dmic
/// iec61937-pcm:7,6,5"): the speakers' amplifiers, the jacks' codec, the
/// microphones' codec, and for a display's device the mixer's.
fn chip(said: &Control, parts: &[Part], device: u32, name: &str, playing: bool) -> Facts {
    let mut facts = Facts::new();
    let component = |key: &str| {
        said.components
            .split(' ')
            .find_map(|each| each.strip_prefix(key)?.strip_prefix(':'))
    };
    let displays = component("iec61937-pcm").unwrap_or_default();
    if displays.split(',').any(|each| each == device.to_string()) {
        fact(&mut facts, "codec", said.mixer.clone());
    } else if playing && name.starts_with("Speaker") {
        if let Some(amplifiers) = component("spk") {
            fact(&mut facts, "amplifiers", chips(amplifiers, parts));
        }
    } else if name.contains("Jack") {
        if let Some(codec) = component("hs") {
            fact(&mut facts, "codec", chips(codec, parts));
        }
    } else if !playing
        && name.contains("Mic")
        && let Some(codec) = component("mic")
    {
        let (chip, digital) = codec
            .strip_suffix("-dmic")
            .map_or((codec, false), |chip| (chip, true));
        let named = chips(chip, parts);
        let said = if digital {
            format!("{named}, digital microphones")
        } else {
            named
        };
        fact(&mut facts, "codec", said);
    }
    facts
}

/// A card's devices that aren't on USB, each keyed as PipeWire's path to it
/// is ("sofsoundwire,2:out" for `hw:sofsoundwire,2`), with what it runs at
/// while it plays or records, what its jack has plugged in, and its chip.
pub(crate) fn built_in(
    card: &Path,
    id: &str,
    said: &Control,
    parts: &[Part],
) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(card) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let folder = entry.file_name().to_string_lossy().into_owned();
        let Some(device) = folder.strip_prefix("pcm") else {
            continue;
        };
        let (number, playing) = match device.split_at_checked(device.len().saturating_sub(1)) {
            Some((number, "p")) => (number, true),
            Some((number, "c")) => (number, false),
            _ => continue,
        };
        let Ok(device) = number.parse::<u32>() else {
            continue;
        };
        let read = |name: &str| fs::read_to_string(entry.path().join(name)).unwrap_or_default();
        let name = read("info")
            .lines()
            .find_map(|line| line.strip_prefix("id: "))
            .map(|id| id.trim_end_matches(" (*)").trim().to_owned())
            .unwrap_or_default();
        let mut facts = Facts::new();
        if read("sub0/status").starts_with("state: RUNNING") {
            fact(
                &mut facts,
                if playing { "playing" } else { "recording" },
                running(&read("sub0/hw_params")),
            );
        }
        fact(
            &mut facts,
            "plugged in",
            jack(&said.jacks, device, &name, playing),
        );
        facts.extend(chip(said, parts, device, &name, playing));
        let direction = if playing { "out" } else { "in" };
        found.push((format!("{id},{device}:{direction}"), facts));
    }
    found
}

/// Opens a card's control device to read and hear its jacks, asking it to
/// say when a value changes.
pub(crate) fn open_control(dev: &Path, number: &str) -> Option<File> {
    let control = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(dev.join(format!("snd/controlC{number}")))
        .ok()?;
    let mut on = [0u8; 4];
    on.copy_from_slice(&1i32.to_ne_bytes());
    ask(&control, SNDRV_CTL_IOCTL_SUBSCRIBE_EVENTS, &mut on).then_some(control)
}

/// Whether a control device's events say a jack changed: each is 72
/// bytes, its element's id at 8 and so its name at 24.
pub(crate) fn a_jack_changed(events: &[u8]) -> bool {
    events
        .as_chunks::<72>()
        .0
        .iter()
        .any(|event| c_text(&event[24..68]).ends_with(" Jack"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    use crate::sound::sound;

    /// The jacks the XPS 14's card has, as its control device names them,
    /// a headset plugged in where asked.
    fn xps_jacks(headset: bool) -> Vec<(String, bool)> {
        [
            ("HDMI/DP,pcm=5 Jack", false),
            ("HDMI/DP,pcm=6 Jack", true),
            ("cs42l45 IT 31 Microphone Jack", false),
            ("cs42l45 IT 33 Headset Jack", headset),
            ("cs42l45 OT 43 Headphone Jack", headset),
            ("cs42l45 OT 45 Headset Jack", headset),
        ]
        .into_iter()
        .map(|(name, plugged)| (name.to_owned(), plugged))
        .collect()
    }

    #[test]
    fn a_jack_says_what_s_plugged_into_its_own_device() {
        let jacks = xps_jacks(false);
        assert_eq!(jack(&jacks, 5, "HDMI1", true), "nothing");
        assert_eq!(jack(&jacks, 6, "HDMI2", true), "a display");
        assert_eq!(jack(&jacks, 0, "Jack Out", true), "nothing");
        assert_eq!(jack(&jacks, 1, "Jack In", false), "nothing");
        assert_eq!(jack(&jacks, 2, "Speaker", true), "");
        assert_eq!(jack(&jacks, 4, "Microphone", false), "");
        let headset = xps_jacks(true);
        assert_eq!(jack(&headset, 0, "Jack Out", true), "a headset");
        assert_eq!(jack(&headset, 1, "Jack In", false), "a headset");
        let headphones = [("Headphone Jack".to_owned(), true)];
        assert_eq!(jack(&headphones, 0, "Headphones", true), "headphones");
        assert_eq!(jack(&headphones, 1, "Headset Mic", false), "");
    }

    /// The XPS 14's `SoundWire` devices: its codec, then its four amplifiers.
    fn xps_parts() -> Vec<Part> {
        let part = |driver: &str, part: &str| Part {
            driver: driver.to_owned(),
            maker: "01fa".to_owned(),
            model: part.to_owned(),
        };
        let mut parts = vec![part("sdca_class", "4245")];
        parts.extend((0..4).map(|_| part("cs35l56", "3557")));
        parts
    }

    #[test]
    fn a_device_names_its_chip_as_the_parts_themselves_say() {
        let said = Control {
            jacks: Vec::new(),
            components: "HDA:80862822,80860101,00100000  cfg-amp:4 iec61937-pcm:7,6,5 \
                         hs:cs42l45 mic:cs42l45-dmic spk:cs35l56"
                .to_owned(),
            mixer: "Intel Panther Lake HDMI".to_owned(),
        };
        let parts = xps_parts();
        let chip_of = |device, name, playing| chip(&said, &parts, device, name, playing);
        let one = |name: &str, value: &str| vec![(name.to_owned(), value.to_owned())];
        assert_eq!(
            chip_of(2, "Speaker", true),
            one("amplifiers", "4 \u{d7} Cirrus Logic CS35L57")
        );
        assert_eq!(
            chip_of(0, "Jack Out", true),
            one("codec", "Cirrus Logic CS42L45")
        );
        assert_eq!(
            chip_of(4, "Microphone", false),
            one("codec", "Cirrus Logic CS42L45, digital microphones")
        );
        assert_eq!(
            chip_of(6, "HDMI2", true),
            one("codec", "Intel Panther Lake HDMI")
        );
        assert_eq!(chips("rt1320", &parts), "RT1320");
        let realtek = Part {
            driver: "rt722".to_owned(),
            maker: "025d".to_owned(),
            model: "0722".to_owned(),
        };
        assert_eq!(part_name(&realtek), "Realtek RT722");
    }

    #[test]
    fn a_built_in_card_says_each_device_by_its_path_and_what_it_runs_at() {
        let sys = Sys::new("built-in");
        let card = sys.0.join("asound/card0");
        put(&card, "id", "sofsoundwire\n");
        put(
            &card,
            "pcm2p/info",
            "card: 0\ndevice: 2\nid: Speaker (*)\nname: \n",
        );
        put(
            &card,
            "pcm2p/sub0/status",
            "state: RUNNING\nowner_pid   : 1\n",
        );
        put(
            &card,
            "pcm2p/sub0/hw_params",
            "access: MMAP_INTERLEAVED\nformat: S32_LE\nsubformat: STD\nchannels: 2\n\
             rate: 48000 (48000/1)\nperiod_size: 1024\nbuffer_size: 32768\n",
        );
        put(&card, "pcm1c/info", "id: Jack In (*)\n");
        put(&card, "pcm1c/sub0/status", "closed\n");
        put(&card, "pcm1c/sub0/hw_params", "closed\n");
        put(&card, "codec#2", "");
        let said = Control {
            jacks: xps_jacks(true),
            components: "hs:cs42l45 spk:cs35l56".to_owned(),
            mixer: String::new(),
        };
        let mut found = built_in(&card, "sofsoundwire", &said, &xps_parts());
        found.sort();
        let pairs = |facts: &Facts| {
            facts
                .iter()
                .map(|(name, value)| format!("{name}: {value}"))
                .collect::<Vec<_>>()
        };
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, "sofsoundwire,1:in");
        assert_eq!(
            pairs(&found[0].1),
            ["plugged in: a headset", "codec: Cirrus Logic CS42L45"]
        );
        assert_eq!(found[1].0, "sofsoundwire,2:out");
        assert_eq!(
            pairs(&found[1].1),
            [
                "playing: 48 kHz \u{b7} 32 bit \u{b7} period 21.3 ms (1024) \u{b7} buffer 682.7 ms (32768)",
                "amplifiers: 4 \u{d7} Cirrus Logic CS35L57"
            ]
        );
        // A card on USB is left to its own reading.
        put(&sys.0, "asound/card1/id", "MV7\n");
        put(&sys.0, "asound/card1/usbid", "14ed:1012\n");
        let all = sound(&sys.0.join("asound"), &sys.0, None, &|_| Control::default());
        assert_eq!(
            all.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(),
            ["sofsoundwire,1:in", "sofsoundwire,2:out"],
            "the built-in card's devices, and nothing of the USB card's"
        );
    }

    #[test]
    fn only_a_jack_s_change_is_heard_from_a_control_device() {
        let event = |name: &str| {
            let mut event = [0u8; 72];
            event[4..8].copy_from_slice(&1u32.to_ne_bytes());
            event[24..24 + name.len()].copy_from_slice(name.as_bytes());
            event
        };
        let volume = event("Speaker Playback Volume");
        let jack = event("cs42l45 OT 43 Headphone Jack");
        assert!(!a_jack_changed(&volume), "a volume's change is no jack's");
        assert!(
            a_jack_changed(&[volume, jack].concat()),
            "a jack's change is heard among others"
        );
    }
}
