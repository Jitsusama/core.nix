# The XPS 14's Own Kernel Patches

Kernel patches for the XPS 14, one fix a file, each named for what it makes
true and opening with the message that says why. Most were written here and
are in no tree yet. Three came from elsewhere and wait to move to `../next/`
at the stack's next regeneration, as [the kernel's README][kernel] names them.

Two files apply them, with a comment on each saying what went wrong:
[`../local.nix`][local] most of them, and [`../../camera/camera.nix`][camera]
the camera's. A patch written here moves to `../next/` once a maintainer's
tree or linux-next takes it.

[kernel]: ../README.md
[local]: ../local.nix
[camera]: ../../camera/camera.nix
