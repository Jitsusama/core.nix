# VM Tests

Each VM test boots one or more machines under QEMU and proves what
only a running system shows: a boot, a stick written and installed, a
desktop driven. A test is two files of one name, its `.nix` (the
machines and how the driver runs) and its `-vm-test.py` (what it
checks, through pytest), and the root `checks.nix` names it as a
`vm-` check.

They are grouped by what they prove, a folder an area:

| Folder   | Holds                                                                |
| -------- | -------------------------------------------------------------------- |
| `guide/` | the guide's sticks, installs and recovery, and Secure Boot           |

The VM tests not yet grouped are still in `../`, and move into a folder
here as their owners free them. Shared helpers are in `../lib/`, and
stand-ins several tests use are in `../`. A check that needs no VM goes
beside its module instead.
