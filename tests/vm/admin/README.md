# The Admin VM Tests

What a machine lets its administrators do and how it keeps itself in hand,
on machines booted in QEMU: switching and updating its system, tuning its
memory and its slices, asking before firmware is written, tracing niri,
stopping libvirt at once, putting a check's VM ahead of the builds beside
it, giving root only the builders it trusts, and restarting a wedged pi
host. Each test's opening comment says what it proves and what stands in
for what a VM lacks.

| File                                            | Holds                                                           |
| ----------------------------------------------- | --------------------------------------------------------------- |
| `firmware-asks.nix`, `-vm-test.py`              | writing firmware asking an administrator, even at the seat      |
| `joel-first.nix`, `-vm-test.py`                 | Joel's shell drawing on time while agents' work bursts          |
| `joel-first-frames.py`                          | that test's stand-in shell, timing how late each frame comes    |
| `joel-first-work.py`                            | that test's stand-in work, using what it holds over and over    |
| `libvirt.nix`, `-vm-test.py`                    | libvirt's daemon stopping at once while its udev thread waits   |
| `memory-tune.nix`, `-vm-test.py`                | four memory settings an administrator sets with no password     |
| `niri-off-cpu.nix`, `-vm-test.py`               | an account in wheel tracing its own niri, and nobody else       |
| `offload-lines.nix`, `-vm-test.py`              | root's Nix given only the builders nix-offload's lines trust    |
| `offload-lines-gcloud.sh`                       | that test's stand-in gcloud, saying what it lent                |
| `pi-conversations-come-back.nix`, `-vm-test.py` | a pi host's conversations back after a kill -9 and a reboot     |
| `pi-conversations-come-back-ask.mjs`            | that test's request to a pi host's socket                       |
| `pi-hosts-recover.nix`, `-vm-test.py`           | a wedged pi host restarted, one always crashing left down       |
| `pi-hosts-recover-host.sh`                      | that test's stand-in node, ready and pinging as a host is       |
| `slices.nix`, `-vm-test.py`                     | the disk latency and weight the kernel gives each slice         |
| `slices-foreground-wakes.sh`                    | that test's foreground program, timing how late it wakes        |
| `switch-system.nix`, `-vm-test.py`              | an administrator switching to a system it has run, no password  |
| `work-slices.nix`, `-vm-test.py`                | an administrator changing how the work slices share the machine |
| `update-to-main.nix`, `-vm-test.py`             | a machine updating itself to its repository's main when asked   |
| `update-to-main-signer`                         | the stand-in key that test signs main with                      |
| `vm-first.nix`, `-vm-test.py`                   | a check's VM going ahead of the builds beside it                |
| `vm-first-guest.py`                             | that test's stand-in VM, its threads as vm-first sees them      |
| `uid-range.nix`                                 | the build that test asks for a range of user ids with           |

- `niri-off-cpu/`: a stand-in for niri's main thread, waiting and asleep as
  niri does.
