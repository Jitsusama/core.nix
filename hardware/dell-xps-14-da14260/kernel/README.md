# The XPS 14's Kernel Patches

Every patch this laptop's kernel carries beyond core's own kernel,
filed by where it stands upstream, so a patch's folder says how
soon it can go. [docs/hardware.md][hardware] says what each one
fixes and why.

| Here                  | Holds                                                   |
| --------------------- | ------------------------------------------------------- |
| `next.nix`, `next/`   | fixes no release has yet, each header saying where from |
| `local.nix`, `local/` | fixes written here and in no tree yet                   |

Four patches came from elsewhere and wait to move at the stack's
next regeneration, since moving them reorders the stack: cvs-wake-irq,
which `local.nix` applies and names, and three of the camera's, which
`../camera/camera.nix` applies (ipu7-isys-releases-firmware from
linux-next, and hm1092-ir-sensor and ipu7-mono-formats from the media
list).

## How a Patch Moves

A patch moves as upstream takes it: from written here, to `next/`
once a maintainer's tree or linux-next has it, to a `backports.nix`
and `backports/` once a release has it, and out once the kernel this
laptop runs has it. Each file's assertion fails when the kernel
reaches the release that holds its patches, which is the sign to
move or drop them. None is carried now: 7.3 holds every fix that
was taken onto 7.2.

Nothing else goes here: a patch to a program rather than the kernel
lives beside the module that builds that program.

[hardware]: ../../../docs/hardware.md
