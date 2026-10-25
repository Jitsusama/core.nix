//! The light in the room, heard from an ambient light sensor's IIO buffer.
//!
//! udev turns the buffer on for each light sensor that has one and lets the
//! seat's user read it (nixos.nix). A sensor hub then puts a sample in the
//! buffer only when the light changes past its hysteresis, so the daemon
//! reads the light once when it starts and afterwards wakes only for a
//! change.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

/// Linux's `O_NONBLOCK`, the same on every architecture core builds for.
const O_NONBLOCK: i32 = 0o4000;

/// Where one channel sits in a buffered sample, and how it's written there.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Field {
    signed: bool,
    big_endian: bool,
    bits: u32,
    bytes: usize,
    shift: u32,
}

impl Field {
    /// A scan element's type, as the kernel writes it: le:s32/32>>0 is a
    /// little-endian signed 32-bit number in 32 bits, not shifted.
    fn read(text: &str) -> Option<Field> {
        let (endian, rest) = text.trim().split_once(':')?;
        let big_endian = match endian {
            "le" => false,
            "be" => true,
            _ => return None,
        };
        let (bits, rest) = rest.split_once('/')?;
        let (signed, bits) = match bits.split_at_checked(1)? {
            ("s", bits) => (true, bits),
            ("u", bits) => (false, bits),
            _ => return None,
        };
        let (storage, shift) = rest.split_once(">>")?;
        // A repeated channel says so after the shift, as X2 and the like,
        // and isn't a light's.
        let field = Field {
            signed,
            big_endian,
            bits: bits.parse().ok()?,
            bytes: storage.parse::<usize>().ok()? / 8,
            shift: shift.parse().ok()?,
        };
        let fits = matches!(field.bytes, 1 | 2 | 4 | 8)
            && field.bits > 0
            && u64::from(field.bits + field.shift) <= 8 * field.bytes as u64;
        fits.then_some(field)
    }

    /// The channel's number from its bytes.
    fn value(self, bytes: &[u8]) -> i64 {
        let mut word = [0; 8];
        let raw = if self.big_endian {
            word[8 - self.bytes..].copy_from_slice(bytes);
            u64::from_be_bytes(word)
        } else {
            word[..self.bytes].copy_from_slice(bytes);
            u64::from_le_bytes(word)
        };
        let unused = 64 - self.bits;
        let raw = raw >> self.shift << unused;
        // Read whole, then sign-extended from the channel's own top bit.
        if self.signed {
            i64::from_ne_bytes(raw.to_ne_bytes()) >> unused
        } else {
            i64::try_from(raw >> unused).unwrap_or(i64::MAX)
        }
    }
}

/// Where the illuminance sits in each sample, among the channels turned on.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout {
    size: usize,
    at: usize,
    field: Field,
}

impl Layout {
    /// The layout of the channels, as their index and field, given which of
    /// them is the light. Each sits after the one before at a multiple of
    /// its own size, and a sample is a multiple of the largest.
    fn of(mut channels: Vec<(u32, Field, bool)>) -> Option<Layout> {
        channels.sort_by_key(|&(index, _, _)| index);
        let mut size: usize = 0;
        let mut largest: usize = 1;
        let mut light = None;
        for (_, field, is_light) in channels {
            size = size.next_multiple_of(field.bytes);
            if is_light {
                light = Some((size, field));
            }
            size += field.bytes;
            largest = largest.max(field.bytes);
        }
        let (at, field) = light?;
        Some(Layout {
            size: size.next_multiple_of(largest),
            at,
            field,
        })
    }
}

/// How a sample's illuminance turns into lux.
#[derive(Clone, Copy, Debug)]
struct Units {
    layout: Layout,
    scale: f64,
    offset: f64,
}

impl Units {
    fn lux(self, raw: i64) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a light sensor's readings are far below 2⁵³, so the float is exact"
        )]
        let raw = raw as f64;
        ((raw + self.offset) * self.scale).max(0.0)
    }

    /// The latest light among whole samples, or none when there's none.
    fn latest(self, samples: &[u8]) -> Option<f64> {
        let Layout { size, at, field } = self.layout;
        let last = (samples.len() / size).checked_sub(1)?;
        let at = last * size + at;
        Some(self.lux(field.value(&samples[at..at + field.bytes])))
    }
}

pub struct Sensor {
    dir: PathBuf,
    node: PathBuf,
    units: Units,
}

fn text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path).map(|text| text.trim().to_owned())
}

fn unreadable(path: &Path, text: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{} reads {text:?}", path.display()),
    )
}

/// A number from an optional attribute, or the default when it isn't there.
fn number_or(path: &Path, default: f64) -> io::Result<f64> {
    match text(path) {
        Ok(text) => text.parse().map_err(|_| unreadable(path, &text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(default),
        Err(error) => Err(error),
    }
}

/// The channels turned on in a device's buffer, as their index, field and
/// whether each is the light, or none when one isn't understood.
fn channels(dir: &Path) -> io::Result<Option<Vec<(u32, Field, bool)>>> {
    let scan = dir.join("scan_elements");
    let mut channels = Vec::new();
    for entry in fs::read_dir(&scan)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let Some(channel) = name.strip_suffix("_en") else {
            continue;
        };
        if text(&scan.join(&name))? != "1" {
            continue;
        }
        let index = text(&scan.join(format!("{channel}_index")))?.parse().ok();
        let field = Field::read(&text(&scan.join(format!("{channel}_type")))?);
        let (Some(index), Some(field)) = (index, field) else {
            eprintln!(
                "<4>{} has a channel {channel} not understood",
                dir.display()
            );
            return Ok(None);
        };
        channels.push((index, field, channel == "in_illuminance"));
    }
    Ok(Some(channels))
}

impl Sensor {
    /// The first light sensor under `root` whose buffer is on with its
    /// illuminance in it, and its device node under `dev`.
    pub fn find(root: &Path, dev: &Path) -> io::Result<Option<Sensor>> {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut dirs = entries
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        dirs.sort();
        for dir in dirs {
            let on = |path: &str| text(&dir.join(path)).is_ok_and(|text| text == "1");
            if !on("buffer/enable") || !on("scan_elements/in_illuminance_en") {
                continue;
            }
            let Some(layout) = channels(&dir)?.and_then(Layout::of) else {
                continue;
            };
            let Some(name) = dir.file_name() else {
                continue;
            };
            let units = Units {
                layout,
                scale: number_or(&dir.join("in_illuminance_scale"), 1.0)?,
                offset: number_or(&dir.join("in_illuminance_offset"), 0.0)?,
            };
            return Ok(Some(Sensor {
                node: dev.join(name),
                dir,
                units,
            }));
        }
        Ok(None)
    }

    /// The sensor's name, to say which is heard.
    pub fn name(&self) -> String {
        self.dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Opens the buffer, emptied of what it held while nobody read it, and
    /// the light now, read once.
    pub fn open(&self) -> io::Result<(Readings, f64)> {
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK)
            .open(&self.node)?;
        let mut samples = vec![0; self.units.layout.size * 32];
        loop {
            match file.read(&mut samples) {
                Ok(0) => break,
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        let raw = self.dir.join("in_illuminance_raw");
        let now = text(&raw)?;
        let now = self
            .units
            .lux(now.parse().map_err(|_| unreadable(&raw, &now))?);
        // SAFETY: the File is moved into the AsyncFd, which owns it until
        // it's dropped, and a File keeps one descriptor open for its life.
        let file = unsafe { AsyncFd::register_with_interest(file, Interest::READABLE) }
            .map_err(io::Error::from)?;
        Ok((
            Readings {
                file,
                units: self.units,
                samples,
            },
            now,
        ))
    }
}

pub struct Readings {
    file: AsyncFd<File>,
    units: Units,
    samples: Vec<u8>,
}

impl Readings {
    /// Waits for the light to change, and says what it changed to.
    pub async fn changed(&mut self) -> io::Result<f64> {
        loop {
            let mut ready = self.file.readable_mut().await?;
            let samples = &mut self.samples;
            match ready.try_io(|file| file.get_mut().read(samples)) {
                Ok(Ok(0)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the light sensor went away",
                    ));
                }
                Ok(Ok(read)) => {
                    if let Some(lux) = self.units.latest(&self.samples[..read]) {
                        return Ok(lux);
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sensor_hub_s_type_reads() {
        let field = Field::read("le:s32/32>>0\n").unwrap();
        assert_eq!(
            field,
            Field {
                signed: true,
                big_endian: false,
                bits: 32,
                bytes: 4,
                shift: 0
            }
        );
    }

    #[test]
    fn a_type_not_understood_reads_as_none() {
        for text in [
            "",
            "le:s32",
            "xe:s32/32>>0",
            "le:q32/32>>0",
            "le:s32/24>>0",
            "le:s12/16>>8",
        ] {
            assert_eq!(Field::read(text), None, "{text}");
        }
    }

    #[test]
    fn a_little_endian_number_reads() {
        let field = Field::read("le:s32/32>>0").unwrap();
        assert_eq!(field.value(&25_000_i32.to_le_bytes()), 25_000);
        assert_eq!(field.value(&(-5_i32).to_le_bytes()), -5);
    }

    #[test]
    fn a_shifted_big_endian_number_is_sign_extended_from_its_own_bits() {
        let field = Field::read("be:s12/16>>4").unwrap();
        // -3 in 12 bits, shifted up 4, with junk in the bits below.
        let word: u16 = (0xffd << 4) | 0x7;
        assert_eq!(field.value(&word.to_be_bytes()), -3);
        let unsigned = Field::read("be:u12/16>>4").unwrap();
        assert_eq!(unsigned.value(&word.to_be_bytes()), 0xffd);
    }

    #[test]
    fn the_light_sits_where_its_index_and_the_alignment_put_it() {
        let int = Field::read("le:s32/32>>0").unwrap();
        let time = Field::read("le:s64/64>>0").unwrap();
        let short = Field::read("le:u16/16>>0").unwrap();
        let hub = Layout::of(vec![(5, time, false), (1, int, true), (0, int, false)]).unwrap();
        assert_eq!(
            hub,
            Layout {
                size: 16,
                at: 4,
                field: int
            }
        );
        let alone = Layout::of(vec![(1, int, true)]).unwrap();
        assert_eq!(
            alone,
            Layout {
                size: 4,
                at: 0,
                field: int
            }
        );
        let padded =
            Layout::of(vec![(0, short, false), (1, int, true), (2, short, false)]).unwrap();
        assert_eq!(
            padded,
            Layout {
                size: 12,
                at: 4,
                field: int
            }
        );
    }

    #[test]
    fn of_several_samples_the_latest_is_the_light_now() {
        let int = Field::read("le:s32/32>>0").unwrap();
        let units = Units {
            layout: Layout::of(vec![(0, int, false), (1, int, true)]).unwrap(),
            scale: 0.5,
            offset: 2.0,
        };
        let samples: Vec<u8> = [7, 10, 7, 40, 7]
            .iter()
            .flat_map(|n: &i32| n.to_le_bytes())
            .collect();
        // Two whole samples and part of a third, which isn't one yet.
        assert_eq!(units.latest(&samples), Some(21.0));
        assert_eq!(units.latest(&samples[..4]), None);
    }

    #[test]
    fn channels_without_the_light_have_no_layout() {
        let int = Field::read("le:s32/32>>0").unwrap();
        assert_eq!(Layout::of(vec![(0, int, false)]), None);
    }

    /// A folder of IIO devices, gone when the test is.
    struct Devices(PathBuf);

    impl Devices {
        fn new(test: &str) -> Self {
            let root = std::env::temp_dir().join(format!("light-{}-{test}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Devices(root)
        }

        fn add(&self, name: &str, buffer: &str, channels: &[(&str, &str, u32, &str)]) -> PathBuf {
            let dir = self.0.join(name);
            fs::create_dir_all(dir.join("buffer")).unwrap();
            fs::create_dir_all(dir.join("scan_elements")).unwrap();
            fs::write(dir.join("buffer/enable"), format!("{buffer}\n")).unwrap();
            for (channel, on, index, kind) in channels {
                let scan = dir.join("scan_elements");
                fs::write(scan.join(format!("in_{channel}_en")), format!("{on}\n")).unwrap();
                fs::write(
                    scan.join(format!("in_{channel}_index")),
                    format!("{index}\n"),
                )
                .unwrap();
                fs::write(scan.join(format!("in_{channel}_type")), format!("{kind}\n")).unwrap();
            }
            dir
        }
    }

    impl Drop for Devices {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_first_sensor_with_its_buffer_on_is_found_with_its_scale() {
        let devices = Devices::new("found");
        let s32 = "le:s32/32>>0";
        devices.add("iio:device0", "0", &[("illuminance", "1", 1, s32)]);
        devices.add("iio:device1", "1", &[("proximity", "1", 0, s32)]);
        let light = devices.add(
            "iio:device2",
            "1",
            &[
                ("intensity_both", "0", 0, s32),
                ("illuminance", "1", 1, s32),
            ],
        );
        fs::write(light.join("in_illuminance_scale"), "0.001000\n").unwrap();
        let sensor = Sensor::find(&devices.0, Path::new("/dev"))
            .unwrap()
            .unwrap();
        assert_eq!(sensor.name(), "iio:device2");
        assert_eq!(sensor.node, Path::new("/dev/iio:device2"));
        assert_eq!(sensor.units.layout.at, 0);
        assert!((sensor.units.lux(25_000) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn a_sensor_with_a_channel_not_understood_is_passed_over() {
        let devices = Devices::new("passed-over");
        devices.add(
            "iio:device0",
            "1",
            &[
                ("illuminance", "1", 0, "le:s32/32>>0"),
                ("odd", "1", 1, "le:s32/32>>0X2"),
            ],
        );
        assert!(
            Sensor::find(&devices.0, Path::new("/dev"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn no_sensors_is_none_found() {
        let devices = Devices::new("missing");
        assert!(
            Sensor::find(&devices.0.join("gone"), Path::new("/dev"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn the_light_is_never_below_none() {
        let devices = Devices::new("dark");
        devices.add(
            "iio:device0",
            "1",
            &[("illuminance", "1", 0, "le:s32/32>>0")],
        );
        let sensor = Sensor::find(&devices.0, Path::new("/dev"))
            .unwrap()
            .unwrap();
        assert!(sensor.units.lux(-40).abs() < f64::EPSILON);
    }
}
