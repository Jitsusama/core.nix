# What Real Machines Printed

Each directory is one machine of the hardware core describes, captured on
2026-10-21, and each file is what one command printed there, as it printed
it. The parsers' tests read these, so a tool
that changes what it prints fails a test rather than a person's install.

- `dell-xps-14-da14260/`: a Dell XPS 14, model DA14260.
- `microsoft-surface-laptop-4-1958/`: a Surface Laptop 4, model 1958.

| File                           | Command                                           |
| ------------------------------ | ------------------------------------------------- |
| `bootctl-list.json`            | `bootctl list --json=short`, as root              |
| `bootctl-status.txt`, `.err`   | `bootctl status`, its stdout and stderr, as a user on the XPS 14 and root on the Surface |
| `efivar-<name>-<guid>`         | `/sys/firmware/efi/efivars/<name>-<guid>`, read whole |
| `fido2-token-L.txt`            | `fido2-token -L`, with no YubiKey plugged in      |
| `fido2-token-I.txt`            | `fido2-token -I`, of the key that lists: on the XPS 14 its virtual linux-id, captured on 2026-10-22 |
| `lsblk.json`                   | `lsblk -J -O -b`, the short form of the guide's `--json --output-all --bytes` |
| `sbctl-status.json`            | `sbctl status --json`                             |
| `sbctl-verify.json`            | `sbctl verify --json`, as root                    |
| `efivar-db-<guid>`             | the firmware's db, which trusts its sbctl db certificate |
| `sbctl-db.pem`                 | that certificate, `/var/lib/sbctl/keys/db/db.pem`, on the Surface |
| `tpm2-properties-<kind>.txt`   | `tpm2_getcap properties-<kind>`, which prints the same with the guide's `-T device:/dev/tpmrm0` |
| `tpm2-handles-nv-index.txt`    | `tpm2_getcap handles-nv-index`, the same                      |
| `tpm2-nvreadpublic-<index>.txt` | `tpm2_nvreadpublic -T device:/dev/tpmrm0 <index>` on the XPS 14, captured on 2026-10-24: its pcrlock policy's index, and an NvPCR's |
| `pcrlock-nvindex.json`         | `jq -c '{nvIndex}' /var/lib/systemd/pcrlock.json`, the NV index in that list systemd-pcrlock keeps the disk's policy in; the rest of the file is sealed to that TPM and of no use to a test |
| `tpm2-load-another-tpms-key.err` | `tpm2_load -T device:/dev/tpmrm0 -C parent.ctx -u public -r private` on the XPS 14, captured on 2026-10-24: `tpm-simulator/`'s key, its halves taken out of its file, under the storage key the guide makes from the owner's seed, as a cleared TPM refuses a key made before |

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

## In a TPM Simulator

`tpm-simulator/` holds `ssh-tpm-keygen.tpm` and its `.pub`, a key ssh-tpm-keygen
0.9.0 made in go-tpm-tools' simulator (`SSH_TPM_AGENT_SWTPM=1`), with no
PIN and the comment `alice@machine`, on 2026-10-24: a TSS2 private key file
as a real machine's, made by no machine's TPM.

| File                           | Command                                           |
| ------------------------------ | ------------------------------------------------- |
| `luks-dump-before-wipe.json`   | `cryptsetup luksDump --dump-json-metadata`, with the random key's slot still there |
| `luks-dump-installed.json`     | the same on the machine's disk once the guide's install took the random key's slot off |
| `luks-dump-sealed.json`        | the same once the guide sealed it to the TPM and preferred that slot, parsed and written again by the test's Python |
| `lsblk-blank.json`             | `lsblk --json --output-all --bytes`, the machine's disk blank |
| `lsblk-laid-out.json`          | the same once the machine's layout made its volume and mounted it at /mnt |
| `fido2-token-L.txt`            | `fido2-token -L`, one canokey plugged in          |
| `fido2-token-I.txt`            | `fido2-token -I` of a canokey with its PIN set, booted from a stick in QEMU on 2026-10-22, one of its PIN's tries spent |
| `bootctl-list-from-the-stick.json` | `bootctl --esp-path=/mnt/boot list --json=short` on a repair stick, in `vm-guide-recovers`: an entry named by its hash, and `isSelected` null |
| `pcrlock-log.json`             | `systemd-pcrlock log --json=short`, as root, on a disk the guide reinstalled and sealed, booted by OVMF straight from the disk on 2026-10-22 |
| `pcrlock-log-through-the-menu.json` | the same, booted through OVMF's boot menu (`systemctl reboot --firmware-setup`, then Return on the disk), which measured its menu program into PCR 4 before the separator |
| `pcrlock-policy.json`          | `jq -c '{pcrBank,pcrValues,nvIndex}' /var/lib/systemd/pcrlock.json` on that disk, the same on both boots |
| `pcr-sha256-<n>`               | `/sys/class/tpm/tpm0/pcr-sha256/<n>` for PCRs 0, 4 and 7 on the straight boot |
| `pcr-sha256-4-through-the-menu` | the same for PCR 4 on the boot through the menu; PCRs 0 and 7 read as on the straight boot |
