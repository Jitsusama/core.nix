//! The keyboard's light: which one, and how bright it is.
//!
//! A laptop's keyboard light is an LED named for it, `<device>::kbd_backlight`,
//! whose levels are a few steps, set through logind as the screen's are.
//! Unlike the screen's backlight, its brightness file wakes no poll when the
//! light key changes it, and on some laptops `brightness_hw_changed` never
//! says so either, so the daemon reads the level whenever it's about to act.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct Keyboard {
    /// The light's name, as logind wants it.
    pub name: String,
    /// Its brightest level.
    pub max: u32,
    dir: PathBuf,
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

impl Keyboard {
    /// The keyboard light under `root`, the first by name when there are
    /// several, or none.
    pub fn find(root: &Path) -> io::Result<Option<Keyboard>> {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut found = Vec::new();
        for entry in entries {
            let dir = entry?.path();
            let Some(name) = dir
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            if !name.ends_with("::kbd_backlight") {
                continue;
            }
            let max = number(&dir.join("max_brightness"))?;
            if max > 0 {
                found.push((name, dir, max));
            }
        }
        found.sort();
        Ok(found
            .into_iter()
            .next()
            .map(|(name, dir, max)| Keyboard { name, max, dir }))
    }

    /// The level now.
    pub fn level(&self) -> io::Result<u32> {
        number(&self.dir.join("brightness"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn led(root: &Path, name: &str, max: &str, brightness: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("max_brightness"), max).unwrap();
        fs::write(dir.join("brightness"), brightness).unwrap();
    }

    #[test]
    fn the_keyboards_light_is_found_among_the_other_leds_and_read() {
        let root = std::env::temp_dir().join(format!("keyboard-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        led(&root, "input3::capslock", "1\n", "0\n");
        led(&root, "platform::micmute", "1\n", "0\n");
        led(&root, "dell::kbd_backlight", "2\n", "1\n");
        let keyboard = Keyboard::find(&root).unwrap().unwrap();
        assert_eq!(keyboard.name, "dell::kbd_backlight");
        assert_eq!(keyboard.max, 2);
        assert_eq!(keyboard.level().unwrap(), 1);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_machine_with_no_keyboard_light_has_none() {
        let root = std::env::temp_dir().join(format!("no-keyboard-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        led(&root, "input3::capslock", "1\n", "0\n");
        assert!(Keyboard::find(&root).unwrap().is_none());
        assert!(Keyboard::find(&root.join("gone")).unwrap().is_none());
        fs::remove_dir_all(&root).unwrap();
    }
}
