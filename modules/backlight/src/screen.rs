//! The screen's backlight: which one, how bright it is, and hearing when
//! that changes.
//!
//! The kernel marks a backlight's `actual_brightness` changed whenever its
//! brightness is written, by the keys, by this daemon or by anything else,
//! and a poll on that file wakes for it. So the daemon waits on the file
//! rather than reading it every so often.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

pub struct Screen {
    /// The backlight's name, as logind wants it.
    pub name: String,
    dir: PathBuf,
    max: u32,
}

fn number(path: &Path) -> io::Result<u32> {
    let text = fs::read_to_string(path)?;
    text.trim().parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} reads {text:?}", path.display()),
        )
    })
}

impl Screen {
    /// The backlight logind and brightnessctl would pick under `root`: the
    /// firmware's first, then the platform's, then a raw one.
    pub fn find(root: &Path) -> io::Result<Option<Screen>> {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut found = Vec::new();
        for entry in entries {
            let dir = entry?.path();
            let rank = match fs::read_to_string(dir.join("type"))
                .unwrap_or_default()
                .trim()
            {
                "firmware" => 0,
                "platform" => 1,
                "raw" => 2,
                _ => continue,
            };
            let max = number(&dir.join("max_brightness"))?;
            let Some(name) = dir
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            if max > 0 {
                found.push((rank, name, dir, max));
            }
        }
        found.sort();
        Ok(found
            .into_iter()
            .next()
            .map(|(_, name, dir, max)| Screen { name, dir, max }))
    }

    /// The connector of the panel this backlight lights, such as eDP-1, when
    /// the backlight is the graphics driver's own: its device is the
    /// connector, named for its card, as card0-eDP-1.
    pub fn connector(&self) -> Option<String> {
        let device = fs::read_link(self.dir.join("device")).ok()?;
        let name = device.file_name()?.to_str()?;
        let (card, connector) = name.split_once('-')?;
        card.starts_with("card").then(|| connector.to_owned())
    }

    /// The brightness now, in the backlight's own steps.
    pub fn brightness(&self) -> io::Result<u32> {
        number(&self.dir.join("brightness"))
    }

    /// A brightness in steps as a share of the brightest.
    pub fn level_of(&self, steps: u32) -> f64 {
        f64::from(steps) / f64::from(self.max)
    }

    /// The steps nearest a share of the brightest, and never none, which on
    /// some panels turns the light off.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a share of the brightest is between 0 and max"
    )]
    pub fn steps_for(&self, level: f64) -> u32 {
        let steps = (level.clamp(0.0, 1.0) * f64::from(self.max)).round();
        (steps as u32).max(1)
    }

    /// A watch that wakes whenever the brightness is written.
    pub fn watch(&self) -> io::Result<Watch> {
        let mut file = File::open(self.dir.join("actual_brightness"))?;
        // The kernel wakes a poll only for changes after the file was read.
        file.read_to_end(&mut Vec::new())?;
        // SAFETY: the File is moved into the AsyncFd, which owns it until
        // it's dropped, and a File keeps one descriptor open for its life.
        let file = unsafe { AsyncFd::register_with_interest(file, Interest::PRIORITY) }
            .map_err(io::Error::from)?;
        Ok(Watch { file })
    }
}

pub struct Watch {
    file: AsyncFd<File>,
}

impl Watch {
    /// Waits for the next time the brightness is written.
    pub async fn changed(&mut self) -> io::Result<()> {
        let mut ready = self.file.ready(Interest::PRIORITY).await?;
        let mut file = ready.get_inner();
        file.seek(SeekFrom::Start(0))?;
        file.read_to_end(&mut Vec::new())?;
        ready.clear_ready();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of backlights, gone when the test is.
    struct Backlights(PathBuf);

    impl Backlights {
        fn new(test: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("backlight-{}-{test}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Backlights(root)
        }

        fn add(&self, name: &str, kind: &str, max: u32, brightness: u32) {
            let dir = self.0.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("type"), format!("{kind}\n")).unwrap();
            fs::write(dir.join("max_brightness"), format!("{max}\n")).unwrap();
            fs::write(dir.join("brightness"), format!("{brightness}\n")).unwrap();
        }
    }

    impl Drop for Backlights {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_firmware_s_backlight_is_picked_over_a_raw_one() {
        let backlights = Backlights::new("firmware");
        backlights.add("intel_backlight", "raw", 512, 208);
        backlights.add("acpi_video0", "firmware", 100, 40);
        let screen = Screen::find(&backlights.0).unwrap().unwrap();
        assert_eq!(screen.name, "acpi_video0");
        assert_eq!(screen.brightness().unwrap(), 40);
    }

    #[test]
    fn a_backlight_of_no_known_kind_or_with_no_steps_is_passed_over() {
        let backlights = Backlights::new("passed-over");
        backlights.add("odd", "strange", 100, 50);
        backlights.add("none", "raw", 0, 0);
        assert!(Screen::find(&backlights.0).unwrap().is_none());
    }

    #[test]
    fn no_backlights_is_none_found() {
        let backlights = Backlights::new("missing");
        assert!(Screen::find(&backlights.0.join("gone")).unwrap().is_none());
    }

    #[test]
    fn shares_and_steps_turn_into_each_other() {
        let backlights = Backlights::new("steps");
        backlights.add("intel_backlight", "raw", 512, 208);
        let screen = Screen::find(&backlights.0).unwrap().unwrap();
        assert!((screen.level_of(256) - 0.5).abs() < 1e-9);
        assert_eq!(screen.steps_for(0.5), 256);
        assert_eq!(screen.steps_for(2.0), 512);
    }

    #[test]
    fn the_panel_is_named_by_its_connector_only_when_the_driver_lights_it() {
        let backlights = Backlights::new("connector");
        backlights.add("intel_backlight", "raw", 512, 208);
        let screen = Screen::find(&backlights.0).unwrap().unwrap();
        assert_eq!(screen.connector(), None, "no device");
        std::os::unix::fs::symlink(
            "../../devices/pci0000:00/0000:00:02.0/drm/card0/card0-eDP-1",
            backlights.0.join("intel_backlight/device"),
        )
        .unwrap();
        assert_eq!(screen.connector().as_deref(), Some("eDP-1"));

        backlights.add("dell_backlight", "firmware", 15, 7);
        std::os::unix::fs::symlink(
            "../../devices/platform/dell-laptop",
            backlights.0.join("dell_backlight/device"),
        )
        .unwrap();
        let firmware = Screen::find(&backlights.0).unwrap().unwrap();
        assert_eq!(
            firmware.connector(),
            None,
            "a platform's light names no panel"
        );
    }

    #[test]
    fn the_dimmest_share_still_has_a_step() {
        let backlights = Backlights::new("dimmest");
        backlights.add("intel_backlight", "raw", 512, 208);
        let screen = Screen::find(&backlights.0).unwrap().unwrap();
        assert_eq!(screen.steps_for(0.0), 1);
    }
}
