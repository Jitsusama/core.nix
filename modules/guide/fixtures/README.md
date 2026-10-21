# What Real Machines Printed

Each directory is one machine of the hardware core describes, captured on
2026-10-21, and each file is what one command printed there, as it printed
it. The parsers' tests read these, so a tool
that changes what it prints fails a test rather than a person's install.

| File                           | Command                                           |
| ------------------------------ | ------------------------------------------------- |
| `bootctl-list.json`            | `bootctl list --json=short`, as root              |
| `bootctl-status.txt`, `.err`   | `bootctl status`, its stdout and stderr, as a user on the XPS 14 and root on the Surface |
| `efivar-<name>-<guid>`         | `/sys/firmware/efi/efivars/<name>-<guid>`, read whole |
| `fido2-token-L.txt`            | `fido2-token -L`, with no YubiKey plugged in      |
| `lsblk.json`                   | `lsblk -J -O -b`, the short form of the guide's `--json --output-all --bytes` |
| `sbctl-status.json`            | `sbctl status --json`                             |
| `sbctl-verify.json`            | `sbctl verify --json`, as root                    |
| `tpm2-properties-<kind>.txt`   | `tpm2_getcap properties-<kind>`, which prints the same with the guide's `-T device:/dev/tpmrm0` |

A missing file is one the machine doesn't have: the Surface's firmware keeps
no `AuditMode` or `DeployedMode`.

Two things were changed after capture, since they name a person's machine
rather than its hardware: the host name inside each system's store path is
now `machine`, and the Surface's drive serial is now `0123456789ABCDEF`.
