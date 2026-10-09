# Installing

How a NixOS laptop goes from a blank disk to Secure Boot with Joel's own keys
and a disk that opens with the TPM and a PIN. It applies to a machine that
imports [`disko`][1] and [`lanzaboote`][2], as [the laptop example][3] does.
The `secure-boot-installs` check walks the same steps on a virtual machine
with a TPM, from the first boot to the TPM refusing a changed PCR, so a pin
update that breaks any of them fails CI.

## What It Ends Up As

| Piece       | What it is                                                     |
| ----------- | -------------------------------------------------------------- |
| Boot        | systemd-boot and each generation, signed by lanzaboote         |
| Secure Boot | Joel's keys only, made on the machine and never copied off it  |
| Disk        | a 1 GB boot partition, then LUKS2 holding Btrfs subvolumes     |
| Daily key   | the TPM, bound to PCRs 0, 4 and 7, plus a PIN                  |
| Recovery    | the YubiKey (FIDO2), then a recovery key kept off the machine  |
| Firmware    | an administrator password, so nobody can turn Secure Boot off  |
| SSH keys    | the TPM's every day, and a spare on the YubiKey                |
| Signing     | two keys in the TPM: Joel's, confirmed each time, and agents'  |
| Keyring     | a random password the TPM seals at the first login             |

The TPM opens the disk only for the same firmware (PCR 0), the same boot
loader, kernel, initrd and command line (PCR 4), and the same Secure Boot keys
and state (PCR 7). Something signed with Joel's own keys but built differently
still changes PCR 4, so it gets no key. lanzaboote rewrites the policy after
every rebuild, so updates never need the disk enrolled again.

Microsoft's keys stay out, so nothing they ever signed boots here, such as a
shim that shows a fake disk prompt to learn the PIN. The firmware still runs
the option ROMs it ran when the keys went in, by their checksums from the
TPM's event log. [Decision 0008][4] weighs this against the alternatives.

## Before Starting

- The machine repository has the machine, importing `disko` and `lanzaboote`,
  with `jitsusama.disk.swapSize` at least the size of the memory.
- A USB stick that boots NixOS, with the machine repository on it or reachable
  over the network.
- The YubiKey, and somewhere off the machine to keep a recovery key.

## 1. Boot the Stick

In the firmware's setup screen (F2 on a Dell), turn Secure Boot off, then boot
the stick. Find the disk's stable name:

```sh
ls -l /dev/disk/by-id/ | grep -v part
```

That name is `jitsusama.disk.device` in the machine's settings.

## 2. Lay Out the Disk and Install

```sh
sudo nix run github:nix-community/disko/v1.13.0#disko-install -- \
  --flake <machine repository>#<machine> \
  --disk main /dev/disk/by-id/<disk> \
  --write-efi-boot-entries
```

disko asks for a passphrase before it encrypts the partition. It only has to
last until step 7, so a long throwaway phrase is fine. The boot loader goes
on unsigned, because the keys don't exist yet. disko-install unmounts the
new system when it finishes, so mount it again to set the account's password
before rebooting:

```sh
sudo nix run github:nix-community/disko/v1.13.0 -- \
  --mode mount --flake <machine repository>#<machine>
sudo nixos-enter --root /mnt -c 'passwd <account>'
```

## 3. Put the Firmware in Setup Mode

Reboot into the setup screen and remove the stick. Under Secure Boot's Expert
Key Management, delete all the keys: with no platform key, the firmware is in
setup mode and accepts new ones. Turn Secure Boot on if the firmware allows it
in setup mode; otherwise turn it on after step 4.

## 4. Let the First Boot Make and Enrol the Keys

Boot the disk and type the passphrase. lanzaboote makes the keys in
`/var/lib/sbctl`, signs the boot loader, leaves the keys on the boot partition
for systemd-boot and reboots. systemd-boot enrols them into the firmware and
reboots again. Type the passphrase once more, then check:

```sh
bootctl status | grep 'Secure Boot'   # Secure Boot: enabled (user)
sudo sbctl verify                     # every file on the boot partition signed
```

## 5. Check the Policy

Once the machine has booted with Secure Boot on, the policy covers all three
PCRs:

```sh
sudo jq '[.pcrValues[].pcr] | unique' /var/lib/systemd/pcrlock.json   # [0, 4, 7]
```

## 6. Bind the Disk to the TPM and a PIN

```sh
disk=/dev/disk/by-partlabel/disk-main-system
sudo systemd-cryptenroll --tpm2-device=auto --tpm2-with-pin=yes \
  --tpm2-pcrlock=/var/lib/systemd/pcrlock.json $disk
sudo systemd-cryptenroll --fido2-device=auto $disk
sudo systemd-cryptenroll --recovery-key $disk
```

The first asks for the passphrase and a new PIN, the second for the YubiKey's
PIN and a touch. The third prints a recovery key: keep it somewhere off this
machine, and check it opens the disk:

```sh
sudo cryptsetup open --test-passphrase $disk
```

## 7. Wipe the Passphrase

Reboot and unlock with the PIN alone. Then the passphrase goes, so the disk
opens only with the TPM and PIN, the YubiKey, or the recovery key:

```sh
sudo systemd-cryptenroll --wipe-slot=password $disk
sudo systemd-cryptenroll $disk   # tpm2, fido2 and recovery, nothing else
```

## 8. Lock the Firmware

In the setup screen, set an administrator password, so turning Secure Boot
off or changing its keys needs it.

## 9. Make the SSH Keys

The everyday SSH key lives in the TPM, and a spare lives on the YubiKey for
when the TPM is wiped or the laptop is gone:

```sh
ssh-tpm-keygen -C "$USER@$(hostname)"
ssh-keygen -t ed25519-sk -O resident -O verify-required -C "$USER@yubikey"
```

The first writes `~/.ssh/id_ecdsa.tpm`, which [`ssh-tpm-agent`][5] loads
whenever it starts. It asks for the key's PIN once a session, through the
themed prompt. The second asks for the YubiKey's FIDO2 PIN (set one first with
`ykman fido access change-pin`) and a touch. It stays on the YubiKey, so
`ssh-keygen -K` brings it back onto any machine. Add both public keys to
GitHub as authentication keys.

Commits and tags are signed with two more keys in the TPM, as the Macs keep
theirs in the Secure Enclave. [`signing`][6] picks Joel's when he commits at a
terminal and the agents' when a harness commits over pipes:

```sh
mkdir -p ~/.local/share/signing
ssh-tpm-keygen -C "$USER@$(hostname) signing" -f ~/.local/share/signing/joel-signing
ssh-tpm-keygen -C "$USER@$(hostname) agents" -f ~/.ssh/agent-signing
systemctl --user restart ssh-tpm-agent
```

Give Joel's a PIN and leave the agents' empty. Joel's lives outside `~/.ssh`
so the agent doesn't load it on its own; the `signing-key` unit adds it so
that every signature asks to be confirmed. Add both public keys to GitHub as
signing keys, and to the machine repository's
`jitsusama.signing.allowedSigners`, so git can say whose a signature is.

## When the TPM Refuses

At boot, systemd asks for the PIN first. When the TPM won't release the key,
it asks for the YubiKey, then for the recovery key. Expect that:

- **After a firmware update.** It changes PCR 0. Unlock once with the YubiKey;
  that boot measures the new firmware and rewrites the policy, so the PIN
  works again from the next one. Updates through fwupd work with Secure Boot
  on, because lanzaboote signs fwupd's EFI program with Joel's keys.
- **After changing a Secure Boot setting or key.** It changes PCR 7 in the
  same way.
- **After too many wrong PINs.** The TPM locks itself for a while to stop
  guessing; the YubiKey still works.

Anything else that makes the TPM refuse means the machine didn't boot what it
should: find out why before typing the recovery key.

[1]: ../modules/disko/nixos.nix
[2]: ../modules/lanzaboote/nixos.nix
[3]: ../examples/laptop.nix
[4]: decisions/0008-own-keys-and-a-tpm-sealed-disk.md
[5]: ../modules/ssh-tpm-agent/home.nix
[6]: ../modules/signing/home.nix
