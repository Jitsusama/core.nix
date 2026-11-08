# The MacBook Air M2's Kernel

Asahi Linux's kernel, built by core rather than taken from
nixos-apple-silicon, so core chooses its tag and its patches.
[docs/hardware.md][hardware] says what the model has and lacks.

| Here              | Holds                                                     |
| ----------------- | --------------------------------------------------------- |
| `package.nix`     | the kernel: Asahi's tag, its settings, and the patches    |
| `nixos.nix`       | what the machine sets beside it: initrd, governor, flags  |
| `fairydust.patch` | Asahi's fairydust branch, as one patch on the tag         |

`package.nix` and `nixos.nix` follow nixos-apple-silicon's
packages/linux-asahi and modules/kernel at the commit core pins,
which are MIT; the hardware module leaves that kernel module out.
The kernel and the patch are the Linux kernel's, GPL-2.0.

## fairydust

Janne Grunau's fairydust branch of AsahiLinux/linux, at
432e09dca5d8 (2026-11-05): its 11 commits beyond `asahi-7.2.15-2`,
from GitHub's comparison of the two. It routes the second display
coprocessor to the front-left USB-C port and passes the port
controller's hot-plug signal on, so one external screen works on
that port. Asahi ships it to try, without support. What's known to
fail:

- Unplugging and plugging back in leaves the screen black until a
  restart.
- A screen plugged in before the display driver starts isn't seen:
  plug it in after signing in.
- Hubs and docks fall back to about 1440p, or drop out; a plain
  USB-C to DisplayPort cable, or a single-purpose USB-C to HDMI 2.0
  adapter, holds.
- One screen, on the front-left port only.

The patch is cut for this tag. When the tag moves, it's cut again
from the branch, or goes once Asahi's tag carries it.

[hardware]: ../../../docs/hardware.md
