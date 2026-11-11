# The Disks VM Tests

How a machine treats its disks and memory as they fill or fail, on machines
booted in QEMU: cargo's filesystem of its own, the disk guard clearing room,
a panic once the disk is gone, oomd ending what filled swap, and coming
back from hibernation. Each test's opening comment says what it fills,
takes away or stands in for.

| File                                 | Holds                                                          |
| ------------------------------------ | -------------------------------------------------------------- |
| `cargo-builds.nix`, `-vm-test.py`    | cargo's own filesystem in an image on Btrfs, kept and remade   |
| `disk-guard.nix`, `-vm-test.py`      | the disk guard taking idle build targets as the disk fills     |
| `disk-loss-panic.nix`, `-vm-test.py` | the kernel panicking once its disk drops off its bus           |
| `full-swap.nix`, `-vm-test.py`       | systemd-oomd ending what filled memory and swap, first         |
| `full-swap-grow.py`                  | that test's program, taking memory as a repack's grows         |
| `hibernate.nix`, `-vm-test.py`       | a machine laid out as the laptops are, hibernating and back    |
