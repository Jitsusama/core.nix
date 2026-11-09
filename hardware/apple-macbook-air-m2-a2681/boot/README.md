# The MacBook Air M2's Boot Chain

How the Air gets from Apple's firmware to systemd-boot, and which of it
core builds.

| Stage | What it is | Where it comes from |
| --- | --- | --- |
| 1 | m1n1, in macOS's place in Apple's boot picker | Asahi's installer, once |
| 2 | `m1n1/boot.bin` on the EFI system partition | core: `nixos.nix`, on each switch |
| 3 | U-Boot, a plain UEFI with no Secure Boot | core: `u-boot.nix`, inside stage 2 |
| 4 | systemd-boot, then core's kernel | NixOS, and `../kernel/` |

Stage 2 joins nixpkgs' m1n1 (2.0.0), the Air's own device tree from core's
kernel (`t8112-j413.dtb`) and U-Boot, so the device tree a switch installs
is always the kernel's own, fairydust's screen included. m1n1 stays at
nixpkgs' release: m1n1's main between cb42485b and 70757dc7 breaks the
M2's second display controller, which drives the external screen.

Stage 1 is Asahi's installer's, and core never writes it. systemd-boot
keeps no older `boot.bin`, so a stage 2 that fails to start is replaced
from macOS, which still starts from Apple's boot picker.

Both files follow nixos-apple-silicon's `boot-m1n1` module and
`uboot-asahi` package, which are MIT, under the notice in
`../LICENSE.nixos-apple-silicon`. U-Boot stays at Asahi's tag
`asahi-v2026.07-2`.
