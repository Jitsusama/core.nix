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

## In a Virtual Machine

`qemu-canokey/` is what a NixOS test's QEMU printed with canokey FIDO2
keys: the first header in the two-key spike, made as the guide makes a
disk (a LUKS2 volume formatted with a random key, each key enrolled with
`systemd-cryptenroll --fido2-device`, then a recovery key), and the rest
on the stick in core's `vm-guide-installs`, its disk `/dev/vda` and the
stick `/dev/sda`.

| File                           | Command                                           |
| ------------------------------ | ------------------------------------------------- |
| `luks-dump-before-wipe.json`   | `cryptsetup luksDump --dump-json-metadata`, with the random key's slot still there |
| `luks-dump-installed.json`     | the same on the machine's disk once the guide's install took the random key's slot off |
| `luks-dump-sealed.json`        | the same once the guide sealed it to the TPM and preferred that slot, parsed and written again by the test's Python |
| `lsblk-blank.json`             | `lsblk --json --output-all --bytes`, the machine's disk blank |
| `lsblk-laid-out.json`          | the same once the machine's layout made its volume and mounted it at /mnt |
| `fido2-token-L.txt`            | `fido2-token -L`, one canokey plugged in          |
