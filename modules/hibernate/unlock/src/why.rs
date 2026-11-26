//! Why a hibernation rolled back, read from what the kernel logged since
//! the sleep began (kmsg.rs reads it), and told to the rescue as words on
//! its command line.

/// Why a hibernation rolled back, as the kernel's log says.
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum Why {
    /// Memory was too full to make the image.
    Memory,
    /// The swap file the image goes to wasn't there.
    SwapGone,
    /// The swap file had no room left for the image.
    SwapFull,
    /// A device's driver, by its name, refused to stop.
    Device(String),
    /// A program, by its name, wouldn't pause.
    Program(String),
    /// Something refused to stop, and the kernel didn't say what.
    Refused,
    /// The image was made but couldn't be written.
    Unwritten,
    /// Something woke the laptop as it went down.
    Woken,
    /// The kernel didn't say.
    Unsaid,
}

impl Why {
    /// What the kernel's log records, in order, say went wrong. When several
    /// things did, the one that stopped it comes first: a device refusing to
    /// power off is logged as a wakeup too.
    pub(crate) fn from_log(said: &[String]) -> Self {
        let any = |of: &[&str]| {
            said.iter()
                .any(|line| of.iter().any(|it| line.contains(*it)))
        };
        let freezing = |ended: &str| {
            said.iter()
                .any(|line| line.starts_with("Freezing ") && line.contains(ended))
        };
        if any(&[
            "PM: hibernation: Error -12 creating image",
            "PM: hibernation: Image allocation is ",
            "PM: hibernation: Cannot allocate",
        ]) {
            return Self::Memory;
        }
        if any(&["PM: Cannot find swap device", "PM: Swap header not found"]) {
            return Self::SwapGone;
        }
        // Compressed, the image runs out of swap as it's written, which the
        // writer says as ENOSPC's -28.
        if any(&["PM: Not enough free swap", "PM: Image saving failed: -28"]) {
            return Self::SwapFull;
        }
        if let Some(driver) = said.iter().find_map(|line| refusing_driver(line)) {
            return Self::Device(driver);
        }
        if let Some(program) = refusing_program(said) {
            return Self::Program(program);
        }
        if any(&["failed to power down"]) || freezing(" failed after ") {
            return Self::Refused;
        }
        if said.iter().any(|line| unwritten(line)) {
            return Self::Unwritten;
        }
        if any(&["Wakeup event detected during hibernation"]) || freezing(" aborted after ") {
            return Self::Woken;
        }
        Self::Unsaid
    }

    /// As the rescue is told it, one word and a name where there is one.
    pub(crate) fn args(&self) -> Vec<String> {
        let (word, name) = match self {
            Self::Memory => ("memory", None),
            Self::SwapGone => ("swap-gone", None),
            Self::SwapFull => ("swap-full", None),
            Self::Device(driver) => ("device", Some(driver)),
            Self::Program(program) => ("program", Some(program)),
            Self::Refused => ("refused", None),
            Self::Unwritten => ("unwritten", None),
            Self::Woken => ("woken", None),
            Self::Unsaid => ("unsaid", None),
        };
        std::iter::once(word.to_owned())
            .chain(name.cloned())
            .collect()
    }

    pub(crate) fn from_args(args: &[String]) -> Self {
        match args {
            [word] if word == "memory" => Self::Memory,
            [word] if word == "swap-gone" => Self::SwapGone,
            [word] if word == "swap-full" => Self::SwapFull,
            [word, driver] if word == "device" => Self::Device(driver.clone()),
            [word, program] if word == "program" => Self::Program(program.clone()),
            [word] if word == "refused" => Self::Refused,
            [word] if word == "unwritten" => Self::Unwritten,
            [word] if word == "woken" => Self::Woken,
            _ => Self::Unsaid,
        }
    }

    /// Whether hibernating again would fail the same way.
    pub(crate) fn fails_again(&self) -> bool {
        matches!(self, Self::Memory | Self::SwapGone | Self::SwapFull)
    }
}

/// The driver a device's refusal names: the kernel logs a device's line as
/// its driver, its device and the message, and the device is a bus address
/// nobody knows the laptop's parts by.
fn refusing_driver(line: &str) -> Option<String> {
    let (who, said) = line.split_once(": PM: ")?;
    if !(said.starts_with("failed to ") || said.starts_with("dpm_run_callback(): ")) {
        return None;
    }
    who.split_whitespace().next().map(str::to_owned)
}

/// The program that wouldn't pause: the kernel logs each task it couldn't
/// freeze after saying it failed, as `task:<name> state:...`. A kernel
/// thread is no program to close, so only user space's are named.
fn refusing_program(said: &[String]) -> Option<String> {
    let failed = said
        .iter()
        .position(|line| line.contains("Freezing user space processes failed after"))?;
    said[failed + 1..].iter().find_map(|line| {
        let task = line.split_once("task:")?.1;
        let name = task.split_once(" state:")?.0.trim();
        (!name.is_empty()).then(|| name.to_owned())
    })
}

/// Whether a line is the swap writer's, or its compressor's, failing.
fn unwritten(line: &str) -> bool {
    let Some(said) = line.split_once("PM: ").map(|(_, said)| said) else {
        return false;
    };
    [
        "Image saving failed",
        "Cannot get swap writer",
        "Could not allocate comp",
        "Cannot start compression threads",
        "Cannot start CRC32 thread",
        "Failed to allocate ",
    ]
    .iter()
    .any(|start| said.starts_with(start))
        || said.contains(" compression failed")
        || said.contains(" compressed length")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::Why;

    pub(crate) fn lines(said: &[&str]) -> Vec<String> {
        said.iter().map(|&line| line.to_owned()).collect()
    }

    /// Every case, with the kernel's own lines for it (7.3.1's
    /// `kernel/power` and `drivers/base/power/main.c`), those around them
    /// included.
    pub(crate) fn cases() -> Vec<(Why, Vec<String>)> {
        vec![
            (
                Why::Memory,
                // The Surface Laptop 4's, on 4 November 2026.
                lines(&[
                    "PM: hibernation: Allocated 745783 pages for snapshot",
                    "PM: hibernation: Normal pages needed: 1089703 + 1024, available pages: 863209",
                    "PM: hibernation: Error -12 creating image",
                ]),
            ),
            (
                Why::Memory,
                lines(&["PM: hibernation: Image allocation is 56721 pages short"]),
            ),
            (
                Why::Memory,
                lines(&["PM: hibernation: Cannot allocate copy bitmap"]),
            ),
            (
                Why::SwapGone,
                lines(&[
                    "PM: Cannot find swap device, try swapon -a",
                    "PM: Cannot get swap writer",
                ]),
            ),
            (Why::SwapGone, lines(&["PM: Swap header not found!"])),
            (Why::SwapFull, lines(&["PM: Not enough free swap"])),
            (
                Why::SwapFull,
                lines(&[
                    "PM: Compressing and saving image data (120034 pages)...",
                    "PM: Image saving failed: -28",
                ]),
            ),
            (
                Why::Device("btintel_pcie".to_owned()),
                // The XPS 14's Bluetooth refusing to power off, which the
                // kernel then logs as a wakeup.
                lines(&[
                    "btintel_pcie 0000:00:14.7: PM: failed to hibernate async: error -16",
                    "PM: hibernation: Wakeup event detected during hibernation, rolling back.",
                ]),
            ),
            (
                Why::Device("amdgpu".to_owned()),
                lines(&[
                    "amdgpu 0000:03:00.0: PM: dpm_run_callback(): pci_pm_freeze returns -16",
                    "amdgpu 0000:03:00.0: PM: failed to freeze async: error -16",
                ]),
            ),
            (
                Why::Program("fuse-thing".to_owned()),
                lines(&[
                    "Freezing user space processes failed after 20.006 seconds (1 tasks \
                     refusing to freeze, wq_busy=0):",
                    "task:fuse-thing      state:D stack:0     pid:4242  tgid:4242  ppid:1",
                ]),
            ),
            (
                Why::Refused,
                lines(&[
                    "Freezing remaining freezable tasks failed after 20.004 seconds (1 tasks \
                     refusing to freeze, wq_busy=0):",
                    "task:kworker/2:1     state:D stack:0     pid:99    tgid:99    ppid:2",
                ]),
            ),
            (
                Why::Refused,
                lines(&["PM: hibernation: Some devices failed to power down, aborting"]),
            ),
            (
                Why::Unwritten,
                lines(&["PM: lz4 compression failed", "PM: Image saving failed: -5"]),
            ),
            (
                Why::Unwritten,
                lines(&["PM: Cannot start compression threads"]),
            ),
            (
                Why::Woken,
                lines(&[
                    "PM: hibernation: Wakeup event detected during hibernation, rolling back.",
                ]),
            ),
            (
                Why::Woken,
                lines(&[
                    "Freezing user space processes aborted after 0.012 seconds (3 tasks \
                     refusing to freeze, wq_busy=0):",
                ]),
            ),
            (
                Why::Unsaid,
                lines(&[
                    "PM: hibernation: hibernation entry",
                    "PM: hibernation: hibernation exit",
                ]),
            ),
            (Why::Unsaid, Vec::new()),
        ]
    }

    #[test]
    fn the_kernels_log_says_why_a_hibernation_rolled_back() {
        for (why, said) in cases() {
            assert_eq!(Why::from_log(&said), why, "{said:?}");
        }
    }

    #[test]
    fn the_rescue_is_told_why_as_it_was_found() {
        for (why, _) in cases() {
            assert_eq!(Why::from_args(&why.args()), why);
        }
        assert_eq!(Why::from_args(&[]), Why::Unsaid);
    }
}
