# The 14-inch M4 Pro's Kernel

Aurora Silicon's kernel, built by core, since Asahi's own tree has no
disk, USB or GPU for the M4 Pro yet. [docs/hardware.md][hardware] says
what the model has and lacks.

| Here           | Holds                                                         |
| -------------- | ------------------------------------------------------------- |
| `package.nix`  | the kernel: Aurora's commit, and its settings as core's own   |
| `bindgen.nix`  | the bindgen it's built with, with the tree's layout repairs   |
| `nixos.nix`    | what the machine sets beside it                               |
| `asahi.config` | Aurora's shared Apple fragment, as it is at that commit       |

## The Source

aurora-silicon/linux's integration/aurora at d4d267839caa (2026-11-08),
the commit Aurora's NixOS flake, nixos-aurora, pins; its tarball's hash
is the one that flake checks. It holds the J614s's board file,
t6040-j614s.dts, which takes the 16-inch J616s's description whole: the
two boards differ in their panel, their wireless board and their
speakers, not in their ports. So the external display chain the J616s gained in Aurora's
PR 526 is the J614s's too: its two external display coprocessors (one
behind HDMI, one routed to the USB-C ports), the crossbars that route
them, and the HDMI converter's hot-plug and power pins. Each of the
board's values there was checked against the ADT of a J614s, Apple's
own description of the machine, and all of them match.

The kernel enables those coprocessors only once the boot loader has
filled in their firmware's memory, which Aurora's aura does on this
board. A J614s has shown a picture over HDMI at 1920x1080 this way
(aura's PR 53); none has yet over USB-C.

## The Settings

`asahi.config` is Aurora's arch/arm64/configs/asahi.config at the same
commit, unchanged. It says to merge it over the ARM64 defconfig and
check its settings survive, which is what `package.nix` does: the build
merges the source's own copy over the defconfig, and each line of this
copy becomes a structured setting, above nixpkgs' common ones, so
`kernel-settings-hold-apple-macbook-pro-m4-pro-a3401` holds every one.
When the commit moves, this copy is taken again from it.

## The Compiler

The kernel is compiled with Clang 22 and `LLVM=1`, as Aurora's NixOS
and Arch Linux ARM builds are. Its Rust drivers reach C through
bindings bindgen writes, and the kernel checks every binding's size,
alignment and offsets as it compiles them. Stock bindgen gets some
wrong: built with it, the build stops at `efi_signature_data_t`'s
alignment. The tree carries the fix in tools/aurora-bindgen, two
patches to bindgen 0.73.2 and a test of each case they fix.
`bindgen.nix` builds that bindgen from those patches, runs that
test, and puts it where the kernel's Makefile looks for it.

## Licences

The kernel and its device trees are the Linux kernel's: GPL-2.0, with
Aurora's Apple device trees GPL-2.0+ OR MIT. bindgen is BSD-3-Clause,
and the tree's tools/aurora-bindgen, its patches among them, is
GPL-2.0-only; both are taken from their sources as they are, never
copied here. nixos-aurora, whose pin this follows and whose bindgen
package `bindgen.nix` is shaped like, is MIT.

[hardware]: ../../../docs/hardware.md
