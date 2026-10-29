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
    Btrfs => "btrfs", "GUIDE_BTRFS";
    Cat => "cat", "GUIDE_CAT";
    Chown => "chown", "GUIDE_CHOWN";
    Cryptsetup => "cryptsetup", "GUIDE_CRYPTSETUP";
    Curl => "curl", "GUIDE_CURL";
    Df => "df", "GUIDE_DF";
    Dmsetup => "dmsetup", "GUIDE_DMSETUP";
    Du => "du", "GUIDE_DU";
    Env => "env", "GUIDE_ENV";
    Fido2Assert => "fido2-assert", "GUIDE_FIDO2_ASSERT";
    Fido2Token => "fido2-token", "GUIDE_FIDO2_TOKEN";
    Gh => "gh", "GUIDE_GH";
    Losetup => "losetup", "GUIDE_LOSETUP";
    Lsblk => "lsblk", "GUIDE_LSBLK";
    Mkdir => "mkdir", "GUIDE_MKDIR";
    Mount => "mount", "GUIDE_MOUNT";
    Mv => "mv", "GUIDE_MV";
    Nix => "nix", "GUIDE_NIX";
    Run0 => "run0", "GUIDE_RUN0";
    NixosEnter => "nixos-enter", "GUIDE_NIXOS_ENTER";
    NixosInstall => "nixos-install", "GUIDE_NIXOS_INSTALL";
    Pcrlock => "systemd-pcrlock", "GUIDE_PCRLOCK";
    Rsync => "rsync", "GUIDE_RSYNC";
    Runuser => "runuser", "GUIDE_RUNUSER";
    Sbctl => "sbctl", "GUIDE_SBCTL";
    Sgdisk => "sgdisk", "GUIDE_SGDISK";
    SshKeygen => "ssh-keygen", "GUIDE_SSH_KEYGEN";
    SshTpmKeygen => "ssh-tpm-keygen", "GUIDE_SSH_TPM_KEYGEN";
    SystemdCryptenroll => "systemd-cryptenroll", "GUIDE_SYSTEMD_CRYPTENROLL";
    Systemctl => "systemctl", "GUIDE_SYSTEMCTL";
    Tpm2Createprimary => "tpm2_createprimary", "GUIDE_TPM2_CREATEPRIMARY";
    Tpm2Getcap => "tpm2_getcap", "GUIDE_TPM2_GETCAP";
    Tpm2Load => "tpm2_load", "GUIDE_TPM2_LOAD";
    Tpm2Nvreadpublic => "tpm2_nvreadpublic", "GUIDE_TPM2_NVREADPUBLIC";
    Tpm2Nvundefine => "tpm2_nvundefine", "GUIDE_TPM2_NVUNDEFINE";
    Udevadm => "udevadm", "GUIDE_UDEVADM";
    Umount => "umount", "GUIDE_UMOUNT";
    Veritysetup => "veritysetup", "GUIDE_VERITYSETUP";
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
    /// Where age finds the plugin a YubiKey seals and opens the vault's
    /// key through: the guide's own PATH, set as it starts.
    AgePlugins,
}

impl Path {
    pub const ALL: &[Path] = &[Path::Disk, Path::Install, Path::Tokens, Path::AgePlugins];

    pub fn name(self) -> &'static str {
        match self {
            Path::Disk => "the disk tools",
            Path::Install => "the install's tools",
            Path::Tokens => "cryptsetup's token plugins",
            Path::AgePlugins => "age's plugins",
        }
    }

    pub fn value(self) -> Option<&'static str> {
        match self {
            Path::Disk => option_env!("GUIDE_DISK_TOOLS"),
            Path::Install => option_env!("GUIDE_INSTALL_TOOLS"),
            Path::Tokens => option_env!("GUIDE_TOKEN_PLUGINS"),
            Path::AgePlugins => option_env!("GUIDE_AGE_PLUGINS"),
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
