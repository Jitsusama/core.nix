//! Every program the guide runs, each by the absolute store path its build
//! gave it (package.nix), so no PATH can put another program in its place:
//! a setuid sudo, a mount, a nix that isn't the one it was built with.

use std::fmt;

macro_rules! tools {
    ($($tool:ident => $name:literal, $variable:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Tool {
            $($tool,)*
        }

        impl Tool {
            pub const ALL: &[Tool] = &[$(Tool::$tool,)*];

            /// The program's own name, as a person would type it.
            pub fn name(self) -> &'static str {
                match self {
                    $(Tool::$tool => $name,)*
                }
            }

            /// Where this build of the guide finds the program, if it was
            /// given one.
            pub fn program(self) -> Option<&'static str> {
                match self {
                    $(Tool::$tool => option_env!($variable),)*
                }
            }
        }
    };
}

tools! {
    Bash => "bash", "GUIDE_BASH";
    Bootctl => "bootctl", "GUIDE_BOOTCTL";
    Cryptsetup => "cryptsetup", "GUIDE_CRYPTSETUP";
    Fido2Assert => "fido2-assert", "GUIDE_FIDO2_ASSERT";
    Fido2Token => "fido2-token", "GUIDE_FIDO2_TOKEN";
    Lsblk => "lsblk", "GUIDE_LSBLK";
    Nix => "nix", "GUIDE_NIX";
    Run0 => "run0", "GUIDE_RUN0";
    NixosEnter => "nixos-enter", "GUIDE_NIXOS_ENTER";
    NixosInstall => "nixos-install", "GUIDE_NIXOS_INSTALL";
    Sbctl => "sbctl", "GUIDE_SBCTL";
    Sgdisk => "sgdisk", "GUIDE_SGDISK";
    SystemdCryptenroll => "systemd-cryptenroll", "GUIDE_SYSTEMD_CRYPTENROLL";
    Systemctl => "systemctl", "GUIDE_SYSTEMCTL";
    Tpm2Getcap => "tpm2_getcap", "GUIDE_TPM2_GETCAP";
    Udevadm => "udevadm", "GUIDE_UDEVADM";
    Umount => "umount", "GUIDE_UMOUNT";
    Wipefs => "wipefs", "GUIDE_WIPEFS";
}

/// A PATH this build of the guide gives a program written to find its
/// tools there, each directory a store path fixed when it was built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path {
    /// What a machine's own disko script runs: what disko's `_packages`
    /// names for core's layout.
    Disk,
    /// What nixos-install takes from its caller: nix and coreutils.
    Install,
    /// Where cryptsetup finds systemd's token plugins, which open a disk
    /// by its FIDO2 and TPM tokens.
    Tokens,
}

impl Path {
    pub const ALL: &[Path] = &[Path::Disk, Path::Install, Path::Tokens];

    pub fn name(self) -> &'static str {
        match self {
            Path::Disk => "the disk tools",
            Path::Install => "the install's tools",
            Path::Tokens => "cryptsetup's token plugins",
        }
    }

    pub fn value(self) -> Option<&'static str> {
        match self {
            Path::Disk => option_env!("GUIDE_DISK_TOOLS"),
            Path::Install => option_env!("GUIDE_INSTALL_TOOLS"),
            Path::Tokens => option_env!("GUIDE_TOKEN_PLUGINS"),
        }
    }

    /// Its value, or the fault of a build that left it out.
    pub fn get(self) -> Result<&'static str, crate::Fault> {
        self.value()
            .ok_or(crate::Fault::Unbuilt { tool: self.name() })
    }
}

/// The packages the disk tools hold, by name, for checking a machine's
/// layout needs nothing else.
pub fn disk_packages() -> Vec<&'static str> {
    option_env!("GUIDE_DISK_PACKAGES")
        .unwrap_or_default()
        .split_whitespace()
        .collect()
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
