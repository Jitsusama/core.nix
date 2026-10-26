# Programs the XPS 14 Builds or Changes

The programs this laptop needs that nixpkgs doesn't give it as they
are: one it builds, and patches to four others. `../nixos.nix`
applies each, beside a comment saying why and when it can go.

| File                              | Holds                                                    |
| --------------------------------- | -------------------------------------------------------- |
| `npu-compiler.nix`                | the NPU's graph compiler, from Intel's driver release     |
| `level-zero-driver-link.patch`    | Level Zero's loader, made to find the GPU's and NPU's drivers |
| `thermald-joins-its-thread.patch` | thermald, stopping without a two-second sleep            |
| `bolt-panther-lake.patch`         | bolt, knowing Panther Lake's USB4 controllers            |
| `power-profiles-tests-wait.patch` | power-profiles-daemon's tests, waiting for a value to settle |

A patch to the kernel goes in `../kernel/`, and the camera's programs
in `../camera/`. A patch is named for what it fixes, and goes once
the program's own release has the fix.
