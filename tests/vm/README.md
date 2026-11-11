# VM Tests

Each VM test boots one or more machines under QEMU and proves what
only a running system shows: a boot, a stick written and installed, a
desktop driven. A test is two files of one name, its `.nix` (the
machines and how the driver runs) and its `-vm-test.py` (what it
checks, through pytest), and its area's `checks.nix` names it as a
`vm-` check, given `vmHostPkgs` by the root `checks.nix`, which keeps
the tests not yet in an area.

They are grouped by what they prove, a folder an area:

| Folder     | Holds                                                              |
| ---------- | ------------------------------------------------------------------ |
| `guide/`   | the guide's sticks, installs and recovery, and Secure Boot         |
| `sound/`   | calls, dictation, and each microphone's and speaker's tuning       |
| `devices/` | desk speakers and lights, lids, idle sleep, USB wake and cameras   |
| `network/` | Wi-Fi, the clock, names, mDNS, a network coming back, the shaper   |
| `admin/`   | switching and updating, memory and slices, niri traces, VMs first  |
| `keys/`    | the keyring, passkeys and security keys in the TPM, signed commits |
| `disks/`   | disks and memory filling or failing, the disk guard, hibernation   |
| `boot/`    | the boot theme's ask, a Bluetooth keyboard there, greeter, reboots |

The VM tests not yet grouped are still in `../`, and move into a folder
here as their owners free them. Shared helpers are in `../lib/`, and
stand-ins several tests use are in `../`, each saying what it stands in
for. A check that needs no VM goes
beside its module instead.
