# The XPS 14's Kernel Patches

Every patch this laptop's kernel carries beyond core's own kernel,
filed by where it stands upstream, so a patch's folder says how
soon it can go. [docs/hardware.md][hardware] says what each one
fixes and why.

| Here                          | Holds                                                   |
| ----------------------------- | ------------------------------------------------------- |
| `backports.nix`, `backports/` | fixes a later release has, taken onto this kernel       |
| `next.nix`, `next/`           | fixes no release has yet, each header saying where from |
| `local.nix`, `local/`         | fixes written here and in no tree yet                   |

Four patches in `local/` came from elsewhere and wait to move at the
stack's next regeneration, as `local.nix` says; the camera's own kernel
patches are applied from `../camera.nix`.

## How a Patch Moves

A patch moves as upstream takes it: from written here, to `next/`
once a maintainer's tree or linux-next has it, to `backports/` once
a release has it, and out once the kernel this laptop runs has it.
Each file's assertion fails when the kernel reaches the release
that holds its patches, which is the sign to move or drop them.

Nothing else goes here: a patch to a program rather than the kernel
lives beside the module that builds that program.

[hardware]: ../../../docs/hardware.md
