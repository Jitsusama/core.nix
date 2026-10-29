# The Guide's VM Tests

What the guide does on real machines, booted in QEMU: writing a stick
on a host, installing from it, recovering a machine, and core's Secure
Boot install that the guide's installs build on. [The guide's own
guide][agents] says which test proves what.

| File                               | Holds                                                          |
| ---------------------------------- | -------------------------------------------------------------- |
| `guide-host.nix`, `-vm-test.py`    | a stick written on a host, as the person writes one            |
| `guide-install.nix`, `-vm-test.py` | a machine installed from its stick, by the guide's keys        |
| `guide-recover.nix`, `-vm-test.py` | a machine recovered from a repair stick                        |
| `guide-one-stick.nix`, `-vm-test.py` | one stick signed for two machines, booted by each one's firmware |
| `secure-boot.nix`, `-vm-test.py`   | core's disk and Secure Boot installed, opened by TPM and PIN   |
| `secure-boot-machine.nix`          | that test's machine, which tests of an installed laptop share  |
| `canokeys.nix`, `set-pin.py`       | the security keys these tests plug in, made before QEMU starts |
| `make-recipient.py`                | a key's age recipient, made as a person makes one              |

The guide's Rust tests, which need no VM, are in
`../../../modules/guide/tests/`.

[agents]: ../../../modules/guide/AGENTS.md
