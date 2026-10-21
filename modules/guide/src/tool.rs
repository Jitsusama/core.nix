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
    Bootctl => "bootctl", "GUIDE_BOOTCTL";
    Fido2Token => "fido2-token", "GUIDE_FIDO2_TOKEN";
    Lsblk => "lsblk", "GUIDE_LSBLK";
    Sbctl => "sbctl", "GUIDE_SBCTL";
    Tpm2Getcap => "tpm2_getcap", "GUIDE_TPM2_GETCAP";
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
