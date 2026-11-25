//! Displays, from their EDID.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use crate::cable::usb_c;
use crate::hearing::IN_Q_OVERFLOW;
use crate::{Facts, fact};

/// The makers a desk is likely to have, by the three letters an EDID gives;
/// any other is shown as its letters.
const MAKERS: [(&str, &str); 14] = [
    ("ACR", "Acer"),
    ("AUO", "AU Optronics"),
    ("BOE", "BOE"),
    ("DEL", "Dell"),
    ("ENC", "EIZO"),
    ("GSM", "LG"),
    ("HPN", "HP"),
    ("LEN", "Lenovo"),
    ("LGD", "LG Display"),
    ("SAM", "Samsung"),
    ("SDC", "Samsung Display"),
    ("SHP", "Sharp"),
    ("BNQ", "BenQ"),
    ("AUS", "ASUS"),
];

/// sRGB's primaries, red, green then blue, as CIE 1931 xy.
const SRGB: [(f64, f64); 3] = [(0.64, 0.33), (0.30, 0.60), (0.15, 0.06)];

/// The three letters an EDID's maker field packs into two bytes.
fn maker(high: u8, low: u8) -> String {
    let code = u16::from_be_bytes([high, low]);
    [10, 5, 0]
        .iter()
        .map(|shift| char::from(b'@' + u8::try_from((code >> shift) & 0x1f).unwrap_or(0)))
        .collect()
}

/// A display descriptor's text: up to 13 bytes, ended by a line feed.
fn text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

/// The area of a triangle of xy points.
fn area(points: [(f64, f64); 3]) -> f64 {
    let [(ax, ay), (bx, by), (cx, cy)] = points;
    ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)).abs() / 2.0
}

/// The red, green, blue and white points an EDID gives, each 10 bits a
/// coordinate.
fn chromaticity(edid: &[u8]) -> [(f64, f64); 4] {
    let low = |byte: u8, shift: u8| u16::from((byte >> shift) & 0b11);
    let value = |high: u8, low: u16| f64::from(u16::from(high) << 2 | low) / 1024.0;
    let (rg, bw) = (edid[25], edid[26]);
    [
        (value(edid[27], low(rg, 6)), value(edid[28], low(rg, 4))),
        (value(edid[29], low(rg, 2)), value(edid[30], low(rg, 0))),
        (value(edid[31], low(bw, 6)), value(edid[32], low(bw, 4))),
        (value(edid[33], low(bw, 2)), value(edid[34], low(bw, 0))),
    ]
}

/// How much wider or narrower its colours' triangle is than sRGB's. One
/// with sRGB's own primaries says nothing, since its Primaries' figures
/// say so beside it (VKCF3Z, 2026-11-09), nor does one giving no colours.
fn gamut(edid: &[u8]) -> String {
    let [red, green, blue, _] = chromaticity(edid);
    let primaries = [red, green, blue];
    let same = primaries
        .iter()
        .zip(SRGB)
        .all(|((x, y), (sx, sy))| (x - sx).abs() < 0.005 && (y - sy).abs() < 0.005);
    let covers = area(primaries) / area(SRGB);
    if same || covers <= 0.0 {
        return String::new();
    }
    format!("{covers:.2} \u{d7} sRGB's area")
}

/// An xy point as figures, x then y, each exactly as its ten bits give it,
/// so the launcher rounds it once, to three places: 0.3134765625, never
/// 0.3135 and then 0.314.
fn figures((x, y): (f64, f64)) -> String {
    format!("{x} {y}")
}

/// `McCamy`'s cubic for a white's correlated colour temperature from its xy,
/// within a few kelvin of the daylight whites a display is set to: its
/// epicentre, then the cubic's terms, highest first.
const EPICENTRE: (f64, f64) = (0.3320, 0.1858);
const MCCAMY: [f64; 4] = [449.0, 3525.0, 6823.3, 5520.33];

/// The coldest and warmest whites a display is set to, in kelvin. Near the
/// cubic's epicentre it gives tens of millions, which no white is, so one
/// past them is said to be none (Y2YBTS, 2026-11-09).
const WHITES: std::ops::RangeInclusive<f64> = 1000.0..=25000.0;

/// A white's correlated colour temperature in kelvin, by `McCamy`'s cubic,
/// where it is one a display could be set to.
fn kelvin((x, y): (f64, f64)) -> Option<f64> {
    let n = (x - EPICENTRE.0) / (EPICENTRE.1 - y);
    let [cubed, squared, linear, constant] = MCCAMY;
    let temperature = n.mul_add(n.mul_add(n.mul_add(cubed, squared), linear), constant);
    WHITES.contains(&temperature).then_some(temperature)
}

/// The white an EDID gives, as figures for the launcher to say by Joel's
/// grouping and decimal point: its colour temperature in whole kelvin, then
/// its x and y, "6462 0.3134765625 0.3291015625"; nothing where it gives
/// none.
fn white_point(edid: &[u8]) -> String {
    let [.., white] = chromaticity(edid);
    if white.1 <= 0.0 {
        return String::new();
    }
    kelvin(white).map_or_else(String::new, |temperature| {
        format!("{temperature:.0} {}", figures(white))
    })
}

/// The red, green and blue an EDID gives, as the figures of each point in
/// turn, red's "0.6396484375 0.330078125" first; nothing where it gives
/// none.
fn primaries(edid: &[u8]) -> String {
    let [red, green, blue, _] = chromaticity(edid);
    if [red, green, blue]
        .iter()
        .all(|(x, y)| *x <= 0.0 && *y <= 0.0)
    {
        return String::new();
    }
    [red, green, blue].map(figures).join(" ")
}

/// Its native mode, from the first detailed timing.
fn native(timing: &[u8]) -> Option<String> {
    let clock = u32::from(u16::from_le_bytes([timing[0], timing[1]])) * 10_000;
    if clock == 0 {
        return None;
    }
    let wide = u32::from(timing[2]) | u32::from(timing[4] >> 4) << 8;
    let blank_wide = u32::from(timing[3]) | u32::from(timing[4] & 0xf) << 8;
    let high = u32::from(timing[5]) | u32::from(timing[7] >> 4) << 8;
    let blank_high = u32::from(timing[6]) | u32::from(timing[7] & 0xf) << 8;
    let total = (wide + blank_wide) * (high + blank_high);
    let hertz = f64::from(clock) / f64::from(total.max(1));
    Some(format!("{wide} \u{d7} {high} at {hertz:.0} Hz"))
}

/// What a range limits descriptor says of the refresh rates it takes.
fn refresh_range(descriptor: &[u8]) -> String {
    let flags = descriptor[4];
    let offset = |set: bool| if set { 255 } else { 0 };
    let lowest = u16::from(descriptor[5]) + offset(flags & 0b11 == 0b11);
    let highest = u16::from(descriptor[6]) + offset(flags & 0b10 != 0);
    format!("{lowest} to {highest} Hz")
}

/// A digital input's bits a colour and its interface, from EDID 1.4's input
/// byte, each empty where the EDID doesn't say. An analogue input says no
/// depth.
fn input(byte: u8) -> (&'static str, &'static str) {
    if byte & 0x80 == 0 {
        return ("", "analogue");
    }
    let depth = match (byte >> 4) & 0b111 {
        1 => "6 bit",
        2 => "8 bit",
        3 => "10 bit",
        4 => "12 bit",
        5 => "14 bit",
        6 => "16 bit",
        _ => "",
    };
    let interface = match byte & 0xf {
        1 => "DVI",
        2 | 3 => "HDMI",
        5 => "DisplayPort",
        _ => "",
    };
    (depth, interface)
}

/// Whether a CTA-861 extension's data blocks hold HDR static metadata, and
/// which transfer functions it takes.
fn hdr(extension: &[u8]) -> Option<String> {
    if extension.first() != Some(&0x02) {
        return None;
    }
    let end = usize::from(*extension.get(2)?).min(extension.len());
    let mut at = 4;
    while at < end {
        let header = extension[at];
        let length = usize::from(header & 0x1f);
        let block = extension.get(at + 1..=at + length)?;
        // An extended block, tag 7, whose own tag 6 is HDR static metadata.
        if header >> 5 == 7 && block.first() == Some(&0x06) {
            let eotf = block.get(1).copied().unwrap_or(0);
            let mut takes = Vec::new();
            if eotf & 0b100 != 0 {
                takes.push("HDR10");
            }
            if eotf & 0b1000 != 0 {
                takes.push("HLG");
            }
            return Some(if takes.is_empty() {
                "metadata only".to_owned()
            } else {
                takes.join(" and ")
            });
        }
        at += 1 + length;
    }
    Some("none".to_owned())
}

/// What a display's EDID says of it, or nothing for one that isn't whole.
fn display(edid: &[u8]) -> Option<Facts> {
    let block = edid.get(..128)?;
    if block[..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]
        || block.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) != 0
    {
        return None;
    }
    let mut facts = Facts::new();
    let code = maker(block[8], block[9]);
    let maker = MAKERS
        .iter()
        .find(|(letters, _)| *letters == code)
        .map_or(code.clone(), |(_, name)| (*name).to_owned());
    let mut model = String::new();
    let mut serial = String::new();
    let mut range = String::new();
    for descriptor in block[54..126].as_chunks::<18>().0.iter().skip(1) {
        if descriptor[..3] != [0, 0, 0] {
            continue;
        }
        match descriptor[3] {
            0xFC => model = text(&descriptor[5..]),
            0xFF => serial = text(&descriptor[5..]),
            0xFD => range = refresh_range(descriptor),
            _ => {}
        }
    }
    if serial.is_empty() {
        let number = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
        if number != 0 {
            serial = number.to_string();
        }
    }
    if model.is_empty() {
        // A panel often names nothing but its product code.
        model = format!("product {:04X}", u16::from_le_bytes([block[10], block[11]]));
    }
    fact(&mut facts, "maker", maker);
    fact(&mut facts, "model", model);
    fact(&mut facts, "serial", serial);
    let year = 1990 + u16::from(block[17]);
    fact(
        &mut facts,
        "made",
        match block[16] {
            0 => year.to_string(),
            0xFF => format!("{year} model"),
            week => format!("week {week}, {year}"),
        },
    );
    let (wide, high) = (f64::from(block[21]), f64::from(block[22]));
    if wide > 0.0 && high > 0.0 {
        let inches = wide.hypot(high) / 2.54;
        fact(
            &mut facts,
            "size",
            format!("{wide:.0} \u{d7} {high:.0} cm, {inches:.1} in"),
        );
    }
    fact(
        &mut facts,
        "native",
        native(&block[54..72]).unwrap_or_default(),
    );
    fact(&mut facts, "refresh", range);
    let (depth, link) = input(block[20]);
    fact(&mut facts, "depth", depth);
    fact(&mut facts, "input", link);
    fact(&mut facts, "colours", gamut(block));
    fact(&mut facts, "white", white_point(block));
    fact(&mut facts, "primaries", primaries(block));
    if let Some(extension) = edid.get(128..256) {
        fact(&mut facts, "HDR", hdr(extension).unwrap_or_default());
    }
    fact(&mut facts, "EDID", format!("{}.{}", block[18], block[19]));
    Some(facts)
}

/// Every connected display by niri's name for its connector, from the DRM
/// connectors `sys` lists.
pub(crate) fn displays(sys: &Path, told: &Path) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(sys.join("class/drm")) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Facts)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (_, connector) = name.strip_prefix("card")?.split_once('-')?;
            let status = fs::read_to_string(entry.path().join("status")).ok()?;
            if status.trim() != "connected" {
                return None;
            }
            let edid = fs::read(entry.path().join("edid")).ok()?;
            let mut facts = display(&edid)?;
            facts.extend(monitor_said(told, &edid));
            Some((connector.to_owned(), facts))
        })
        .collect();
    found.sort();
    // The USB-C cable's facts go to the one external display there is; with
    // more, nothing says which is the cable's.
    let mut external = found
        .iter_mut()
        .filter(|(connector, _)| !connector.starts_with("eDP"));
    if let (Some((_, facts)), None) = (external.next(), external.next()) {
        facts.extend(usb_c(sys));
    }
    found
}

/// What a monitor says of itself over DDC/CI, as the backlight daemon keeps
/// it in `told` (modules/backlight's facts.rs): a file named for the EDID's
/// bytes 8 to 17, a fact a line, its name and value split by a tab.
fn monitor_said(told: &Path, edid: &[u8]) -> Facts {
    let Some(identity) = edid.get(8..18) else {
        return Facts::new();
    };
    let name = identity
        .iter()
        .fold(String::from("ddc-"), |mut name, byte| {
            let _ = write!(name, "{byte:02x}");
            name
        });
    let text = fs::read_to_string(told.join(name)).unwrap_or_default();
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// Whether inotify's events name a monitor's DDC facts written whole, or
/// lost some that might have.
pub(crate) fn names_a_monitor(events: &[u8]) -> bool {
    let mut rest = events;
    while let Some((head, tail)) = rest.split_at_checked(16) {
        let word =
            |at: usize| u32::from_ne_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let (mask, length) = (word(4), word(12) as usize);
        let Some((name, after)) = tail.split_at_checked(length) else {
            return true;
        };
        let name = name.split(|byte| *byte == 0).next().unwrap_or(name);
        if mask & IN_Q_OVERFLOW != 0 || (name.starts_with(b"ddc-") && !name.ends_with(b".new")) {
            return true;
        }
        rest = after;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    use crate::hearing::{IN_DELETE, IN_MOVED_TO};

    /// The HP Z27k G3's EDID, as it gave it on 2026-10-24.
    const Z27K_G3: &str = "\
        00ffffffffffff00220ed336000000001c1f0104b53c22783eee95a3544c99260f5054a10800d1c0a9c0a940d100b300\
        9500810081804dd000a0f0703e803020350055502100001a000000fd001d3c1c873c010a202020202020000000fc0048\
        50205a32376b2047330a2020000000ff00434e34313238324743310a202001c8020313b14b61100403021f1312115f01\
        e2006ba36600a0f0701f803020350055502100001a023a801871382d40582c450055502100001e283c80a070b0234030\
        20360055502100001a565e00a0a0a029503020350055502100001a000000000000000000000000000000000000000000\
        000000000000000000000000000000ed";

    #[test]
    fn the_z27k_g3_says_who_made_it_and_what_it_is() {
        let edid = bytes(Z27K_G3);
        assert_eq!(edid.len(), 256);
        let facts = display(&edid).unwrap();
        assert_eq!(named(&facts, "maker"), Some("HP"));
        assert_eq!(named(&facts, "model"), Some("HP Z27k G3"));
        assert_eq!(named(&facts, "serial"), Some("CN41282GC1"));
        assert_eq!(named(&facts, "made"), Some("week 28, 2021"));
        assert_eq!(named(&facts, "size"), Some("60 \u{d7} 34 cm, 27.2 in"));
        assert_eq!(named(&facts, "native"), Some("3840 \u{d7} 2160 at 60 Hz"));
        assert_eq!(named(&facts, "refresh"), Some("29 to 60 Hz"));
        assert_eq!(named(&facts, "depth"), Some("10 bit"));
        assert_eq!(named(&facts, "input"), Some("DisplayPort"));
        // sRGB's own primaries, which its Primaries' figures say.
        assert_eq!(named(&facts, "colours"), None);
        assert_eq!(
            named(&facts, "white"),
            Some("6462 0.3134765625 0.3291015625")
        );
        assert_eq!(
            named(&facts, "primaries"),
            Some("0.6396484375 0.330078125 0.2998046875 0.599609375 0.150390625 0.0595703125")
        );
        assert_eq!(named(&facts, "HDR"), Some("none"));
        assert_eq!(named(&facts, "EDID"), Some("1.4"));
    }

    #[test]
    fn an_input_says_its_depth_apart_from_its_link() {
        assert_eq!(input(0xB5), ("10 bit", "DisplayPort"));
        assert_eq!(input(0x80), ("", ""));
        assert_eq!(input(0x0F), ("", "analogue"));
    }

    #[test]
    fn wider_colours_are_said_against_srgb_s_area() {
        let mut edid = bytes(Z27K_G3);
        // DCI-P3's primaries in 1024ths, about 1.36 times sRGB's area:
        // red 0.680, 0.320; green 0.265, 0.690; blue 0.150, 0.060.
        for (at, value) in [
            (27, 696u16),
            (28, 328),
            (29, 271),
            (30, 707),
            (31, 154),
            (32, 61),
        ] {
            edid[at] = u8::try_from(value >> 2).unwrap();
        }
        edid[25] = 0;
        edid[26] &= 0x0f;
        let said = gamut(&edid);
        assert!(said.ends_with("sRGB's area"), "{said}");
        assert!(said.starts_with("1.3"), "{said}");
    }

    #[test]
    fn an_edid_without_its_colours_says_nothing_of_them() {
        let mut edid = bytes(Z27K_G3);
        let given = edid[25..35]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        edid[25..35].fill(0);
        edid[127] = edid[127].wrapping_add(given);
        let facts = display(&edid).unwrap();
        assert_eq!(named(&facts, "colours"), None);
        assert_eq!(named(&facts, "white"), None);
        assert_eq!(named(&facts, "primaries"), None);
    }

    #[test]
    fn a_white_is_one_a_display_could_be_set_to() {
        // D65, then one beside the cubic's epicentre, tens of millions.
        let d65 = kelvin((0.3127, 0.3290)).unwrap();
        assert!((d65 - 6505.0).abs() < 1.0, "{d65}");
        assert_eq!(kelvin((0.4, 0.185)), None);
    }

    #[test]
    fn a_display_that_names_nothing_is_named_by_its_product_code() {
        let mut edid = bytes(Z27K_G3);
        // The name descriptor, the third, made a dummy one, the sum kept.
        let at = 54 + 2 * 18 + 3;
        assert_eq!(edid[at], 0xFC);
        edid[at] = 0x10;
        edid[127] = edid[127].wrapping_add(0xFC - 0x10);
        let facts = display(&edid).unwrap();
        assert_eq!(named(&facts, "model"), Some("product 36D3"));
    }

    #[test]
    fn what_isn_t_a_whole_edid_says_nothing() {
        let mut edid = bytes(Z27K_G3);
        edid[127] ^= 1;
        assert_eq!(display(&edid), None);
        assert_eq!(display(&[0; 64]), None);
    }

    #[test]
    fn an_edid_maker_code_is_three_letters() {
        assert_eq!(maker(0x22, 0x0E), "HPN");
        assert_eq!(maker(0x30, 0xE4), "LGD");
    }

    #[test]
    fn hdr_is_found_in_its_extended_block() {
        // A CTA block with one HDR static metadata block taking HDR10 and HLG.
        let mut extension = vec![0x02, 0x03, 0x08, 0x00, 0xE3, 0x06, 0x0C, 0x01];
        extension.resize(128, 0);
        assert_eq!(hdr(&extension).as_deref(), Some("HDR10 and HLG"));
        assert_eq!(hdr(&[0x70, 0x00]), None);
    }

    #[test]
    fn a_monitor_s_ddc_facts_are_found_by_its_edid() {
        let told = std::env::temp_dir().join(format!("desk-told-{}", std::process::id()));
        fs::create_dir_all(&told).unwrap();
        fs::write(
            told.join("ddc-220ed336000000001c1f"),
            "brightness\t40%\nnot a fact\nfirmware level\t1.0\n",
        )
        .unwrap();
        let said = monitor_said(&told, &bytes(Z27K_G3));
        assert_eq!(
            said,
            vec![
                ("brightness".to_owned(), "40%".to_owned()),
                ("firmware level".to_owned(), "1.0".to_owned()),
            ]
        );
        assert!(
            monitor_said(&told, &[0; 128]).is_empty(),
            "another monitor's"
        );
        fs::remove_dir_all(&told).unwrap();
    }

    #[test]
    fn a_monitor_s_facts_written_whole_are_heard_and_their_copy_is_not() {
        assert!(
            names_a_monitor(&event(IN_MOVED_TO, "ddc-220ed336000000001c1f")),
            "a monitor's facts written whole"
        );
        assert!(
            names_a_monitor(&event(IN_DELETE, "ddc-220ed336000000001c1f")),
            "a monitor's facts removed"
        );
        assert!(
            !names_a_monitor(&event(IN_MOVED_TO, "ddc-220ed336000000001c1f.new")),
            "a copy still being written"
        );
        assert!(
            !names_a_monitor(&event(IN_MOVED_TO, "displays.json")),
            "the desk's own file"
        );
    }
}
