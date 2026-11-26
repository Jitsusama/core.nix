//! Sound cards, from what ALSA says of them.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use crate::own_sound::{Control, built_in, sound_parts};
use crate::usb::{release, speed};
use crate::{Facts, fact};

/// The Pebble X Plus, as its USB device names itself.
const PEBBLE: &str = "041e:329a";

/// One way a USB card's stream can run: its format, channels and rates.
#[derive(Debug, Default, PartialEq, Eq)]
struct Setting {
    format: String,
    channels: String,
    rates: String,
}

/// One direction of a USB card, as its stream file says.
#[derive(Debug, Default, PartialEq, Eq)]
struct Stream {
    running: bool,
    momentary: String,
    clock: String,
    settings: Vec<Setting>,
}

/// A USB card's playback and capture streams, from
/// /proc/asound/cardN/stream0. Its first line ends in the card's speed, but
/// the kernel cuts that line at 79 characters, and a long enough name cuts
/// the speed ("full spee" for the Pebble X Plus), so the speed is read
/// from the device instead.
fn streams(text: &str) -> (Stream, Stream) {
    let (mut playback, mut capture) = (Stream::default(), Stream::default());
    let mut now: Option<&mut Stream> = None;
    for line in text.lines() {
        let line = line.trim();
        match line {
            "Playback:" => now = Some(&mut playback),
            "Capture:" => now = Some(&mut capture),
            _ => {}
        }
        let Some(stream) = now.as_deref_mut() else {
            continue;
        };
        if let Some(status) = line.strip_prefix("Status: ") {
            stream.running = status == "Running";
        } else if let Some(rate) = line.strip_prefix("Momentary freq = ") {
            rate.split(" (")
                .next()
                .unwrap_or(rate)
                .clone_into(&mut stream.momentary);
        } else if let Some(format) = line.strip_prefix("Format: ") {
            stream.settings.push(Setting {
                format: format.to_owned(),
                ..Setting::default()
            });
        } else if let Some(channels) = line.strip_prefix("Channels: ") {
            if let Some(setting) = stream.settings.last_mut() {
                channels.clone_into(&mut setting.channels);
            }
        } else if let Some(rates) = line.strip_prefix("Rates: ") {
            if let Some(setting) = stream.settings.last_mut() {
                rates.clone_into(&mut setting.rates);
            }
        } else if let Some(endpoint) = line.strip_prefix("Endpoint: ") {
            for (said, clock) in [
                ("(ASYNC)", "asynchronous"),
                ("(ADAPTIVE)", "adaptive"),
                ("(SYNC)", "synchronous"),
            ] {
                if endpoint.contains(said) {
                    clock.clone_into(&mut stream.clock);
                }
            }
        }
    }
    (playback, capture)
}

/// A sample format's bits, as ALSA names it: `S24_3LE` is 24.
fn bits(format: &str) -> String {
    let digits: String = format
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        format.to_owned()
    } else {
        format!("{digits} bit")
    }
}

/// A rate in Hz, in kHz as it's said: 44100 is "44.1 kHz".
fn kilohertz(rate: &str) -> String {
    let Ok(hertz) = rate.trim().parse::<u32>() else {
        return rate.trim().to_owned();
    };
    if hertz % 1000 == 0 {
        format!("{} kHz", hertz / 1000)
    } else {
        format!("{:.1} kHz", f64::from(hertz) / 1000.0)
    }
}

fn channels(count: &str) -> String {
    match count {
        "1" => "mono".to_owned(),
        "2" => "stereo".to_owned(),
        count => format!("{count} channels"),
    }
}

/// Two figures of one value, set apart as the launcher sets a run apart,
/// never with a comma beside its dots (VKCF3Z, 2026-11-07).
const APART: &str = " \u{b7} ";

/// Every way a stream can run, each once: "48 kHz \u{b7} 16 bit \u{b7}
/// stereo".
fn offered(settings: &[Setting]) -> String {
    let mut said: Vec<String> = Vec::new();
    for setting in settings {
        let rates: Vec<String> = setting.rates.split(',').map(kilohertz).collect();
        let each = format!(
            "{}{APART}{}{APART}{}",
            rates.join(" or "),
            bits(&setting.format),
            channels(&setting.channels)
        );
        if !said.contains(&each) {
            said.push(each);
        }
    }
    said.join("; ")
}

/// What a running stream's `hw_params` say it runs at: its period and
/// buffer, each the time it holds at its rate, which is the card's own
/// share of the delay, then in frames: "48 kHz \u{b7} 32 bit \u{b7} period
/// 21.3 ms (1024) \u{b7} buffer 682.7 ms (32768)". Where the rate isn't
/// known, it isn't said, and only the frames are.
pub(crate) fn running(hw_params: &str) -> String {
    let field = |name: &str| {
        hw_params
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix(": "))
            .map(|value| value.split(' ').next().unwrap_or(value).to_owned())
    };
    let (Some(format), Some(rate)) = (field("format"), field("rate")) else {
        return String::new();
    };
    // A rate the stream doesn't know yet is no rate, so it isn't said.
    let hertz = rate.parse::<u32>().ok().filter(|hertz| *hertz > 0);
    let mut said = match hertz {
        Some(_) => format!("{}{APART}{}", kilohertz(&rate), bits(&format)),
        None => bits(&format),
    };
    for (name, size) in [
        ("period", field("period_size")),
        ("buffer", field("buffer_size")),
    ] {
        let Some(size) = size else { continue };
        let _ = match (hertz, size.parse::<u32>()) {
            (Some(hertz), Ok(frames)) => write!(
                said,
                "{APART}{name} {} ({size})",
                milliseconds(frames, hertz)
            ),
            _ => write!(said, "{APART}{name} {size}"),
        };
    }
    said
}

/// The time some frames hold at a rate, to a tenth of a millisecond.
fn milliseconds(frames: u32, hertz: u32) -> String {
    let tenths = (u64::from(frames) * 10_000 + u64::from(hertz) / 2) / u64::from(hertz);
    format!("{}.{} ms", tenths / 10, tenths % 10)
}

/// One direction of a card's facts.
fn direction(stream: &Stream, speed: &str, firmware: &str, now: &str, playing: bool) -> Facts {
    let mut facts = Facts::new();
    if stream.running {
        fact(
            &mut facts,
            if playing { "playing" } else { "recording" },
            now,
        );
        fact(&mut facts, "measured rate", stream.momentary.clone());
    }
    fact(&mut facts, "offers", offered(&stream.settings));
    fact(&mut facts, "clock", stream.clock.clone());
    fact(&mut facts, "USB", speed);
    fact(&mut facts, "firmware", firmware);
    facts
}

/// Every sound card's facts, from the cards `asound` lists and their
/// devices in `sys`: a USB card's by its ALSA id and direction, and the
/// Pebble's lighting from what its file says; another card's by each of
/// its devices (`built_in`), with what `controls` says of the card by its
/// number.
pub(crate) fn sound(
    asound: &Path,
    sys: &Path,
    lighting: Option<&str>,
    controls: &dyn Fn(&str) -> Control,
) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(asound) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(number) = name.strip_prefix("card") else {
            continue;
        };
        let card = entry.path();
        let Ok(id) = fs::read_to_string(card.join("id")) else {
            continue;
        };
        if !card.join("usbid").exists() {
            let parts = sound_parts(sys);
            found.extend(built_in(&card, id.trim(), &controls(number), &parts));
            continue;
        }
        let (Ok(stream0), Ok(usbid)) = (
            fs::read_to_string(card.join("stream0")),
            fs::read_to_string(card.join("usbid")),
        ) else {
            continue;
        };
        let (id, usbid) = (id.trim(), usbid.trim());
        let usb = sys.join(format!("class/sound/card{number}/device/.."));
        let firmware = fs::read_to_string(usb.join("bcdDevice"))
            .map(|bcd| format!("release {}", release(&bcd)))
            .unwrap_or_default();
        let speed = speed(&fs::read_to_string(usb.join("speed")).unwrap_or_default());
        let (playback, capture) = streams(&stream0);
        let now =
            |sub: &str| fs::read_to_string(card.join(sub)).map_or(String::new(), |hw| running(&hw));
        if !playback.settings.is_empty() {
            let mut facts = direction(
                &playback,
                &speed,
                &firmware,
                &now("pcm0p/sub0/hw_params"),
                true,
            );
            if usbid == PEBBLE {
                let said = match lighting {
                    Some("on") => "on",
                    Some("off") => "off while unused",
                    Some("off by hand") => "off at its button",
                    _ => "",
                };
                if !said.is_empty() {
                    facts.insert(0, ("lighting".to_owned(), said.to_owned()));
                }
            }
            found.push((format!("{id}:out"), facts));
        }
        if !capture.settings.is_empty() {
            found.push((
                format!("{id}:in"),
                direction(
                    &capture,
                    &speed,
                    &firmware,
                    &now("pcm0c/sub0/hw_params"),
                    false,
                ),
            ));
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn a_usb_card_s_streams_say_what_they_offer_and_how_they_run() {
        let (playback, capture) = streams(MV7);
        assert!(!playback.running, "the MV7's playback is stopped");
        assert_eq!(playback.clock, "adaptive");
        assert_eq!(
            offered(&playback.settings),
            "44.1 kHz or 48 kHz \u{b7} 24 bit \u{b7} stereo"
        );
        assert!(capture.running, "the MV7's capture is running");
        assert_eq!(capture.momentary, "48000 Hz");
        assert_eq!(capture.clock, "asynchronous");
        assert_eq!(
            offered(&capture.settings),
            "44.1 kHz or 48 kHz \u{b7} 24 bit \u{b7} mono"
        );
    }

    #[test]
    fn a_running_stream_says_its_format_period_and_buffer() {
        let hw = "access: MMAP_INTERLEAVED\nformat: S24_3LE\nsubformat: STD\nchannels: 1\nrate: 48000 (48000/1)\nperiod_size: 256\nbuffer_size: 16384\n";
        assert_eq!(
            running(hw),
            "48 kHz \u{b7} 24 bit \u{b7} period 5.3 ms (256) \u{b7} buffer 341.3 ms (16384)"
        );
        // Without a rate, the frames are all there is to say.
        assert_eq!(
            running("format: S16_LE\nrate: 0\nperiod_size: 256\n"),
            "16 bit \u{b7} period 256"
        );
        assert_eq!(milliseconds(1024, 44_100), "23.2 ms");
        assert_eq!(milliseconds(0, 48_000), "0.0 ms");
        assert_eq!(running("closed\n"), "");
    }

    #[test]
    fn a_direction_says_what_plays_first_and_only_while_it_does() {
        let (playback, capture) = streams(MV7);
        let speed = speed("12\n");
        let recording = direction(
            &capture,
            &speed,
            "release 2.03",
            "48 kHz \u{b7} 24 bit",
            false,
        );
        assert_eq!(
            recording[0],
            ("recording".to_owned(), "48 kHz \u{b7} 24 bit".to_owned())
        );
        assert_eq!(named(&recording, "measured rate"), Some("48000 Hz"));
        let idle = direction(&playback, &speed, "release 2.03", "", true);
        assert_eq!(named(&idle, "playing"), None);
        assert_eq!(named(&idle, "USB"), Some("12 Mbit/s"));
        assert_eq!(named(&idle, "firmware"), Some("release 2.03"));
    }

    /// The Pebble X Plus as card1, stopped.
    fn pebble(name: &str) -> Sys {
        let sys = Sys::new(name);
        put(&sys.0, "asound/card1/id", "Plus\n");
        put(&sys.0, "asound/card1/usbid", "041e:329a\n");
        put(
            &sys.0,
            "asound/card1/stream0",
            "Creative Technology Ltd Creative Pebble X Plus at usb-0000:00:14.0-4, full spee : USB Audio

Playback:
  Status: Stop
  Interface 1
    Altset 1
    Format: S16_LE
    Channels: 2
    Endpoint: 0x01 (1 OUT) (ADAPTIVE)
    Rates: 48000
",
        );
        fs::create_dir_all(sys.0.join("class/sound/card1/device")).unwrap();
        put(&sys.0, "class/sound/card1/speed", "12\n");
        put(&sys.0, "class/sound/card1/bcdDevice", "1310\n");
        sys
    }

    #[test]
    fn a_usb_card_with_a_long_name_still_says_its_whole_speed() {
        // The kernel cuts the stream file's first line at 79 characters,
        // and the Pebble X Plus's name is long enough to cut its speed.
        let sys = pebble("long-name");
        let all = sound(&sys.0.join("asound"), &sys.0, None, &|_| Control::default());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "Plus:out");
        assert_eq!(named(&all[0].1, "USB"), Some("12 Mbit/s"));
        assert_eq!(named(&all[0].1, "firmware"), Some("release 13.10"));
    }

    #[test]
    fn the_pebble_says_its_lighting_first_as_its_file_has_it() {
        let sys = pebble("lighting");
        let lit = |said: Option<&str>| {
            let all = sound(&sys.0.join("asound"), &sys.0, said, &|_| Control::default());
            let first = all[0].1.first().cloned();
            first
                .filter(|(name, _)| name == "lighting")
                .map(|(_, value)| value)
        };
        assert_eq!(lit(Some("on")).as_deref(), Some("on"));
        assert_eq!(lit(Some("off")).as_deref(), Some("off while unused"));
        assert_eq!(
            lit(Some("off by hand")).as_deref(),
            Some("off at its button")
        );
        assert_eq!(
            lit(Some("dim")),
            None,
            "a word the file doesn't use was said"
        );
        assert_eq!(lit(None), None, "lighting was said with no file");
    }
}
