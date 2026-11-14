//! Starts a Bluetooth controller that stopped answering again.
//!
//! A controller can wedge in its firmware while the kernel still holds it
//! up, and then answers nothing until its driver lets go of it and takes
//! it back, which bluetooth-links' notice tells Joel to type this for.
//! Each Bluetooth controller on PCI, found by its class rather than an
//! address, is let go of through its driver's `unbind` and taken back
//! through its `bind`; one no driver holds, as a restart cut short leaves
//! it, is handed to the bus's `drivers_probe`, which finds its driver.
//! Those files are root's, so each write goes through `sudo tee`, which
//! asks for a password only where a terminal can answer. Once taken back,
//! a controller with no driver again is a failure.
//!
//!   bluetooth-again    starts each Bluetooth controller again
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, its answer as text or JSON, and a failure that says what to try.

use serde::Serialize;
use serde_json::json;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use terminal_kit::{Context, Exit, Failure, Format, Help, Row, layout};

/// PCI's class for a Bluetooth controller: a wireless controller (0d) of
/// Bluetooth's kind (11), whatever its programming interface.
const BLUETOOTH: &str = "0x0d11";

/// The tee its writes go through, built in where the package says.
const TEE: &str = match option_env!("BLUETOOTH_AGAIN_TEE") {
    Some(tee) => tee,
    None => "tee",
};

/// What's left to try once it can't start a controller.
const RESTART: &str = "restart the machine";

/// A Bluetooth controller, by its address on PCI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Controller {
    device: String,
    /// The driver holding it, by name; none where nothing does.
    driver: Option<String>,
}

/// The driver holding a PCI device, by name; none where nothing does.
fn driver_of(device: &Path) -> Option<String> {
    fs::read_link(device.join("driver"))
        .ok()?
        .file_name()?
        .to_str()
        .map(str::to_owned)
}

/// Each Bluetooth controller on PCI under the sysfs root, by address.
fn controllers(sysfs: &Path) -> Vec<Controller> {
    let Ok(devices) = fs::read_dir(sysfs.join("bus/pci/devices")) else {
        return Vec::new();
    };
    let mut found: Vec<Controller> = devices
        .filter_map(Result::ok)
        .filter(|entry| {
            fs::read_to_string(entry.path().join("class"))
                .is_ok_and(|class| class.trim().starts_with(BLUETOOTH))
        })
        .filter_map(|entry| {
            Some(Controller {
                device: entry.file_name().into_string().ok()?,
                driver: driver_of(&entry.path()),
            })
        })
        .collect();
    found.sort_by(|one, other| one.device.cmp(&other.device));
    found
}

/// The controllers to start again, or why there are none.
fn found(sysfs: &Path) -> Result<Vec<Controller>, Failure> {
    let found = controllers(sysfs);
    if found.is_empty() {
        return Err(Failure::new(
            Exit::NotFound,
            "it found no Bluetooth controller to start again",
        )
        .because("no device on PCI is of Bluetooth's class")
        .try_this(RESTART));
    }
    Ok(found)
}

/// Where its address is written, in order, to start a controller again:
/// its driver's `unbind` then `bind`, or the bus's `drivers_probe` for one no
/// driver holds.
fn steps(sysfs: &Path, controller: &Controller) -> Vec<PathBuf> {
    let bus = sysfs.join("bus/pci");
    match &controller.driver {
        Some(driver) => {
            let driver = bus.join("drivers").join(driver);
            vec![driver.join("unbind"), driver.join("bind")]
        }
        None => vec![bus.join("drivers_probe")],
    }
}

/// Starts a controller again, each step written with `write`, and gives it
/// as it is after.
fn start(
    sysfs: &Path,
    controller: &Controller,
    write: &mut dyn FnMut(&Path, &str) -> io::Result<()>,
) -> Result<Controller, Failure> {
    for step in steps(sysfs, controller) {
        write(&step, &controller.device).map_err(|error| {
            Failure::new(
                Exit::Failed,
                format!("it couldn't start {} again", controller.device),
            )
            .because(error.to_string())
            .try_this(RESTART)
        })?;
    }
    let device = sysfs.join("bus/pci/devices").join(&controller.device);
    let Some(driver) = driver_of(&device) else {
        return Err(Failure::new(
            Exit::Failed,
            format!("{} didn't come back", controller.device),
        )
        .because("no driver took it back")
        .try_this(RESTART));
    };
    Ok(Controller {
        device: controller.device.clone(),
        driver: Some(driver),
    })
}

/// Writes an address to a file of root's, through sudo, which asks for a
/// password where it may.
fn through_sudo(asks: bool) -> impl FnMut(&Path, &str) -> io::Result<()> {
    move |path, device| {
        let mut sudo = Command::new("sudo");
        if !asks {
            sudo.arg("--non-interactive");
        }
        let mut child = sudo
            .arg(TEE)
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?;
        // tee reads to its end, so its stdin closes once the address is in.
        child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("sudo took no input"))?
            .write_all(device.as_bytes())?;
        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "sudo tee {} {status}",
                path.display()
            )))
        }
    }
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "bluetooth-again",
        "starts a Bluetooth controller that stopped answering again",
    )
    .example(
        "bluetooth-again",
        "lets each controller's driver go of it and take it back, through sudo",
    )
    .exit(Exit::NotFound, "the machine has no Bluetooth controller on PCI")
    .exit(
        Exit::Failed,
        "sudo refused, or a controller didn't come back",
    )
}

/// The controllers started again, as the answer's form asks.
fn answer(context: &Context, started: &[Controller]) -> io::Result<()> {
    if context.format() != &Format::Text {
        return context.answer(&json!({
            "schema": "bluetooth-again.started/1",
            "controllers": started,
        }));
    }
    let rows: Vec<Row> = started
        .iter()
        .map(|controller| {
            Row::named(&controller.device)
                .marked("done", "success")
                .holding(controller.driver.clone().unwrap_or_default())
                .side("started again")
        })
        .collect();
    context.say(&layout::joined(vec![layout::list(
        &rows,
        &context.out,
        context.room,
    )]))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        if !context.args.is_empty() {
            return Err(
                Failure::new(Exit::Usage, "it takes no words").try_this("bluetooth-again")
            );
        }
        let sysfs = Path::new("/sys");
        let mut write = through_sudo(context.can_ask());
        let mut started = Vec::new();
        for controller in found(sysfs)? {
            let through = controller.driver.as_deref().unwrap_or("its bus");
            let _ = context.tell(
                "running",
                "accent",
                &format!(
                    "starting {} again through {through}; sudo may ask for your password",
                    controller.device
                ),
            );
            started.push(start(sysfs, &controller, &mut write)?);
        }
        answer(context, &started).map_err(|error| {
            Failure::new(Exit::Failed, "it couldn't say what it started")
                .because(error.to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    /// A sysfs root of its own for each test, gone after.
    struct Sysfs(PathBuf);

    impl Sysfs {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "bluetooth-again-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("bus/pci/devices")).expect("a stand-in sysfs");
            fs::create_dir_all(root.join("bus/pci/drivers")).expect("a stand-in sysfs");
            Self(root)
        }

        /// A PCI device of a class, held by a driver or none.
        fn device(&self, address: &str, class: &str, driver: Option<&str>) {
            let device = self.0.join("bus/pci/devices").join(address);
            fs::create_dir_all(&device).expect("a stand-in device");
            fs::write(device.join("class"), format!("{class}\n")).expect("its class");
            if let Some(driver) = driver {
                self.hold(address, driver);
            }
        }

        /// A driver taking a device, as the kernel links them both ways.
        fn hold(&self, address: &str, driver: &str) {
            let folder = self.0.join("bus/pci/drivers").join(driver);
            let device = self.0.join("bus/pci/devices").join(address);
            fs::create_dir_all(&folder).expect("a stand-in driver");
            let _ = symlink(&device, folder.join(address));
            let _ = symlink(&folder, device.join("driver"));
        }

        /// A driver letting a device go.
        fn release(&self, address: &str) {
            let device = self.0.join("bus/pci/devices").join(address);
            if let Ok(driver) = fs::read_link(device.join("driver")) {
                let _ = fs::remove_file(driver.join(address));
            }
            let _ = fs::remove_file(device.join("driver"));
        }
    }

    impl Drop for Sysfs {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn intel(driver: Option<&str>) -> Controller {
        Controller {
            device: "0000:00:14.7".to_owned(),
            driver: driver.map(str::to_owned),
        }
    }

    #[test]
    fn a_bluetooth_controller_is_found_by_its_class_with_its_driver() {
        let sysfs = Sysfs::new("found");
        sysfs.device("0000:00:14.7", "0x0d1100", Some("btintel_pcie"));
        sysfs.device("0000:00:14.3", "0x028000", Some("iwlwifi"));
        sysfs.device("0000:01:00.0", "0x0d1100", None);
        assert_eq!(
            controllers(&sysfs.0),
            vec![
                intel(Some("btintel_pcie")),
                Controller {
                    device: "0000:01:00.0".to_owned(),
                    driver: None,
                },
            ],
            "only Bluetooth's class is a controller, each with its driver or none"
        );
    }

    #[test]
    fn no_controller_on_pci_is_not_found() {
        let sysfs = Sysfs::new("none");
        sysfs.device("0000:00:14.3", "0x028000", Some("iwlwifi"));
        let failure = found(&sysfs.0).expect_err("a machine with none has none to start");
        assert_eq!(
            (failure.exit, failure.then.as_deref()),
            (Exit::NotFound, Some(RESTART)),
            "it says none is there, and what's left"
        );
    }

    #[test]
    fn a_held_controller_is_let_go_of_then_taken_back_by_its_driver() {
        let sysfs = Sysfs::new("held");
        sysfs.device("0000:00:14.7", "0x0d1100", Some("btintel_pcie"));
        let mut written = Vec::new();
        let after = start(&sysfs.0, &intel(Some("btintel_pcie")), &mut |path, device| {
            written.push((path.to_path_buf(), device.to_owned()));
            match path.file_name().and_then(|name| name.to_str()) {
                Some("unbind") => sysfs.release(device),
                Some("bind") => sysfs.hold(device, "btintel_pcie"),
                _ => {}
            }
            Ok(())
        })
        .expect("it starts again");
        let driver = sysfs.0.join("bus/pci/drivers/btintel_pcie");
        assert_eq!(
            written,
            vec![
                (driver.join("unbind"), "0000:00:14.7".to_owned()),
                (driver.join("bind"), "0000:00:14.7".to_owned()),
            ],
            "its driver lets it go, then takes it back"
        );
        assert_eq!(after, intel(Some("btintel_pcie")), "it's held again");
    }

    #[test]
    fn a_controller_no_driver_holds_is_handed_to_the_bus_to_find_one() {
        let sysfs = Sysfs::new("loose");
        sysfs.device("0000:00:14.7", "0x0d1100", None);
        let mut written = Vec::new();
        let after = start(&sysfs.0, &intel(None), &mut |path, device| {
            written.push(path.to_path_buf());
            sysfs.hold(device, "btintel_pcie");
            Ok(())
        })
        .expect("the bus finds its driver");
        assert_eq!(
            written,
            vec![sysfs.0.join("bus/pci/drivers_probe")],
            "the bus is asked to find its driver"
        );
        assert_eq!(after, intel(Some("btintel_pcie")), "it's held again");
    }

    #[test]
    fn a_controller_that_doesnt_come_back_fails() {
        let sysfs = Sysfs::new("gone");
        sysfs.device("0000:00:14.7", "0x0d1100", Some("btintel_pcie"));
        let failure = start(&sysfs.0, &intel(Some("btintel_pcie")), &mut |path, device| {
            if path.ends_with("unbind") {
                sysfs.release(device);
            }
            Ok(())
        })
        .expect_err("no driver took it back");
        assert_eq!(
            (failure.exit, failure.then.as_deref()),
            (Exit::Failed, Some(RESTART)),
            "it says it failed, and what's left"
        );
    }

    #[test]
    fn a_write_sudo_refuses_stops_it_there() {
        let sysfs = Sysfs::new("refused");
        sysfs.device("0000:00:14.7", "0x0d1100", Some("btintel_pcie"));
        let mut tries = 0;
        let failure = start(&sysfs.0, &intel(Some("btintel_pcie")), &mut |_, _| {
            tries += 1;
            Err(io::Error::other("sudo tee exit status: 1"))
        })
        .expect_err("sudo refused");
        assert_eq!(
            (tries, failure.exit, failure.why.as_deref()),
            (1, Exit::Failed, Some("sudo tee exit status: 1")),
            "nothing is written after a refusal, and it says why"
        );
    }
}
