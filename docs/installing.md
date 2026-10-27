# Installing

How a NixOS laptop goes from a blank disk to Secure Boot with Joel's own keys
and a disk that opens with the TPM and a PIN. It applies to a machine that
imports [`disko`][1] and [`lanzaboote`][2], as [the laptop example][3] does.
The `vm-secure-boot-installs` check walks the same steps on a virtual machine
with a TPM, from the first boot to the TPM refusing a changed PCR, so a pin
update that breaks any of them fails its checks.

## What It Ends Up As

| Piece       | What it is                                                     |
| ----------- | -------------------------------------------------------------- |
| Boot        | systemd-boot and each generation, signed by lanzaboote         |
| Boot menu   | hidden; hold Space at power-on for an older generation         |
| Bad boots   | three tries for a new generation, then the last good one       |
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

A new generation counts as good once it reaches multi-user.target. Until then
systemd-boot counts each boot in its file name, and after three that never got
there it boots the newest generation that did, which a kernel that panics or
an initrd that can't open the disk would otherwise leave to the hidden menu.
The session is then told which generation failed. Three boots of a new
generation powered off at the PIN count the same way.

Microsoft's keys stay out, so nothing they ever signed boots here, such as a
shim that shows a fake disk prompt to learn the PIN. A machine whose firmware
runs option ROMs enrols their checksums from the TPM's event log beside the
keys, so the firmware keeps running them; one whose firmware runs none
enrols the keys alone. [Decision 0008][4] weighs this against the
alternatives.

## Before Starting

- The machine repository has the machine, importing `disko` and `lanzaboote`,
  with `jitsusama.disk.swapSize` at least the size of the memory.
- A USB stick that boots NixOS, with the machine repository on it or reachable
  over the network. If the stick is itself a machine built on core's `disko`,
  its `jitsusama.disk.name` differs from the one being installed, or the
  install can take the stick's open volume for the new one.
- The YubiKey, and somewhere off the machine to keep a recovery key.
- Whether the machine's firmware runs option ROMs, if anyone knows. A machine
  that does sets `boot.lanzaboote.autoEnrollKeys.includeChecksumsFromTPM`.
  Nobody has to know in advance: step 4 stops and says so when the firmware
  runs one.

## 1. Boot the Stick

In the firmware's setup screen (F2 on a Dell), turn Secure Boot off, then boot
the stick. Find the disk's stable name:

```sh
ls -l /dev/disk/by-id/ | grep -v part
```

That name is `jitsusama.disk.device` in the machine's settings.

## 2. Lay Out the Disk and Install

```sh
sudo nix run github:nix-community/disko/de5708739256238fb912c62f03988815db89ec9a#disko-install -- \
  --flake <machine repository>#<machine> \
  --disk main /dev/disk/by-id/<disk> \
  --write-efi-boot-entries
```

disko is named by the commit of its v1.13.0 tag, the one core locks. Root's
nix has no GitHub token, and a tag makes it ask GitHub's API to resolve it,
which refuses requests without one once the network's shared quota is spent;
a commit downloads without asking.
Root can't sign in to a private input either, so when the machine repository
has one, build the machine's system as the account first, which fetches the
input into the store where root finds it:

```sh
nix build --no-link <machine repository>#nixosConfigurations.<machine>.config.system.build.toplevel
```

disko asks for a passphrase before it encrypts the partition. It only has to
last until step 7, so a long throwaway phrase is fine. The boot loader goes
on unsigned, because the keys don't exist yet. disko-install unmounts the
new system when it finishes, so mount it again to set the account's password
before rebooting:

```sh
sudo nix run github:nix-community/disko/de5708739256238fb912c62f03988815db89ec9a -- \
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
`/var/lib/sbctl`, signs the boot loader and leaves the keys on the boot
partition for systemd-boot. Then, since Secure Boot is off in setup mode,
`secure-boot-enrol-restart.service` reboots. systemd-boot enrols them into
the firmware and reboots again. Type the passphrase once more, then check:

```sh
bootctl status | grep 'Secure Boot'   # Secure Boot: enabled (user)
sudo sbctl verify                     # every file on the boot partition signed
```

A machine the guide installs skips the first of those boots: the stick runs
the same commands inside the installed system, through the
`secure-boot-enrolment` program core's lanzaboote module puts at the system's
top, so systemd-boot enrols the keys as the machine first starts, before any
disk asks for a PIN.

Some firmware, such as Dell's, locks itself into deployed mode once it has a
platform key, so `enabled (deployed)` is as good as `enabled (user)`.

If it still says setup or audit mode, the keys never reached the firmware.
lanzaboote leaves them for systemd-boot only when
`prepare-sb-auto-enroll.service` succeeds, and restarts for them only while
the firmware isn't enforcing, so look at both:

```sh
systemctl status prepare-sb-auto-enroll
journalctl -b -u prepare-sb-auto-enroll -u secure-boot-enrol-restart
```

A firmware already enforcing with these keys, as on a reinstall that carried
them, gets neither the files nor a restart. Once Secure Boot enforces,
`secure-boot-enrolled.service` takes the files off the boot partition. Left
there, systemd-boot would put these keys back the moment the firmware's were
cleared to install the machine afresh, if its own disk booted before the
stick, after a countdown and before any disk asks for a PIN.

`Found OptionROM in the bootchain` means the firmware runs an option ROM that
the keys alone would stop. Set
`boot.lanzaboote.autoEnrollKeys.includeChecksumsFromTPM = true;` in the
machine, install it for the next boot and reboot, which tries again:

```sh
nixos-rebuild boot --sudo --flake <machine repository>#<machine>
```

With it set on a machine whose firmware runs none, sbctl fails the other
way, with `could not find any OpROM entries in the TPM eventlog`; take it out
again.

## 5. Check the Policy

Once the machine has booted with Secure Boot on, the policy covers all three
PCRs:

```sh
sudo jq '[.pcrValues[].pcr] | unique' /var/lib/systemd/pcrlock.json   # [0, 4, 7]
```

`[0, 4]` means Secure Boot isn't on yet, so go back to step 4 before binding
the disk to it.

## 6. Bind the Disk to the TPM and a PIN

Turn the YubiKey's OTP off first. Touched when nothing has asked for it, the
YubiKey types a one-time password and presses Enter, which at the disk prompt
submits it as a PIN. FIDO2 and the smart card keep working:

```sh
ykman config usb --disable OTP
```

```sh
disk=/dev/disk/by-partlabel/system
sudo systemd-cryptenroll --tpm2-device=auto --tpm2-with-pin=yes \
  --tpm2-pcrlock=/var/lib/systemd/pcrlock.json $disk
sudo systemd-cryptenroll --fido2-device=auto $disk
sudo systemd-cryptenroll --recovery-key $disk
```

The first asks for the passphrase and a new PIN: give it one of the TPM's own
that you'll remember at every start, such as your account password, which can
change later without changing it. The second asks for the YubiKey's PIN and a
touch. The third prints a recovery key: keep it somewhere off this
machine, and check it opens the disk:

```sh
sudo cryptsetup open --test-passphrase $disk
```

The boot prompt tries the PIN on each key in token order, preferred key slots
first. Prefer the TPM's, so it always goes first, even after it's enrolled
again and its token comes after the YubiKey's. Otherwise every boot sends the
PIN to the YubiKey first and spends one of its tries:

```sh
sudo systemd-cryptenroll $disk   # the tpm2 line gives its slot
sudo cryptsetup config $disk --key-slot <tpm2 slot> --priority prefer
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

On a machine the guide installed, it makes the TPM's keys itself, at the boot
after the disk is sealed: every key a module names in `jitsusama.tpmKeys`, as
its account, with one PIN for each that asks for one. After a cleared TPM, which
can never load the keys it held before, it makes them again. Then it keeps the
login key's spare on each YubiKey the stick enrolled, asking for each by name:
it downloads the spare a YubiKey already holds, and makes one only on a YubiKey
holding none. On any other machine, make them by hand, as below.

Run this step as the account, never under sudo. Every key here belongs in the
account's home, and under sudo they land in root's.

The login key lives in the TPM, and a spare lives on the YubiKey for
when the TPM is wiped or the laptop is gone:

```sh
ssh-tpm-keygen -C "$USER@$(hostname)"
ssh-keygen -t ed25519-sk -O resident -O verify-required -C "$USER@yubikey"
```

The first writes `~/.ssh/id_ecdsa.tpm`, which [`ssh-tpm-agent`][5] loads
whenever it starts. It asks for the key's PIN once a session, through the
themed prompt. The second asks for the YubiKey's FIDO2 PIN (set one first with
`ykman fido access change-pin`) and a touch. It stays on the YubiKey, so
`ssh-keygen -K` brings it back onto any machine. If the YubiKey already holds
the spare from another machine, ssh-keygen asks whether to overwrite it:
answer no and run `ssh-keygen -K` instead, because a new key leaves every
place the old one was added holding a key nothing can use. Add both public
keys to GitHub as authentication keys.

Commits and tags are signed with two more keys in the TPM, as the Macs keep
theirs in the Secure Enclave. [`tpm-signing`][6] picks Joel's when he commits
at a terminal and the agents' when a harness commits over pipes:

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

At boot, systemd asks for each device's PIN on its own, the TPM first. Each
prompt names which PIN it wants: "TPM2 PIN" takes the TPM's, asked once, and
when the TPM won't release the key, "security token PIN" follows for the
YubiKey's, then a touch when it blinks. A TPM refusing the boot costs no try.
On a disk with a YubiKey enrolled, the disk module turns off
systemd-cryptsetup's token plugins in the initrd, since their prompt, "LUKS2
token PIN", tries one PIN on every token: it would hand the TPM's PIN to a
plugged-in YubiKey, and three such tries block it. A disk with none keeps
them, since without them systemd would never ask for its passphrase. A
YubiKey's PIN typed where the TPM's is asked costs the TPM a try, which counts
towards its lockout even on a boot its policy refuses. The recovery key is the
seven words exactly as the guide showed them, lowercase and one space apart.
The recovery key is tried only once the YubiKey is unplugged or blocked, so
typed while it is plugged in it opens nothing; to use it, unplug the YubiKey.
A wrong PIN costs a try on both: the TPM locks itself after 32 and forgives
one every two hours it's on, and the YubiKey's FIDO2 PIN blocks for good after
8 in a row.

The policy can't follow a change it didn't predict. systemd-pcrlock rewrites
it with a recovery PIN it keeps sealed to the policy itself, so once the
firmware no longer matches, nothing can rewrite it. Relax the policy before
the change:

- **Before a firmware update,** through fwupd or the firmware's own flash.
  It changes PCR 0, so take PCR 0 out of the policy:

  ```sh
  sudo systemd-pcrlock unlock-firmware-code
  sudo systemctl restart systemd-pcrlock-make-policy
  sudo jq '[.pcrValues[].pcr] | unique' /var/lib/systemd/pcrlock.json   # [4, 7]
  ```

  The first boot on the new firmware locks it again by itself, and the PIN
  opens the disk at every boot throughout. Updates through fwupd work with
  Secure Boot on, because lanzaboote signs fwupd's EFI program with Joel's
  keys.
- **Before changing a Secure Boot setting or key.** It changes PCR 7; run
  `systemd-pcrlock unlock-secureboot-policy` first, in the same way.
- **After too many wrong PINs.** The TPM locks itself for a while to stop
  guessing; the YubiKey still works.
- **After a boot through the firmware's boot menu.** Choosing the disk from
  the firmware's own menu, or from its setup screen, measures that menu
  program into PCR 4 before the boot loader, which the policy leaves out.
  The TPM refuses that boot alone, so open it with the YubiKey and enrol
  nothing: the next ordinary boot opens with the PIN. Sealing again here
  would seal the disk to boots through the menu, and refuse every ordinary
  one. The guide tells this boot from the event log and leaves the seal
  alone.

When a change got there first, unlock with the YubiKey, then replace the
policy and enrol the TPM again. The last command checks the new slot opens
with the PIN without opening anything; this cryptsetup ignores
`--external-tokens-path`, so systemd's token plugins go on the library path:

```sh
disk=/dev/disk/by-partlabel/system
sudo systemd-pcrlock remove-policy
sudo systemctl restart systemd-pcrlock-make-policy
sudo systemd-cryptenroll --unlock-fido2-device=auto --wipe-slot=tpm2 \
  --tpm2-device=auto --tpm2-with-pin=yes \
  --tpm2-pcrlock=/var/lib/systemd/pcrlock.json $disk
sudo systemd-cryptenroll $disk   # the tpm2 line gives its slot
sudo cryptsetup config $disk --key-slot <tpm2 slot> --priority prefer
sudo env LD_LIBRARY_PATH=/run/current-system/sw/lib/cryptsetup \
  cryptsetup open --test-passphrase --token-only --token-type systemd-tpm2 $disk
```

Anything else that makes the TPM refuse means the machine didn't boot what it
should: find out why before typing the recovery key.

[1]: ../modules/disko/nixos.nix
[2]: ../modules/lanzaboote/nixos.nix
[3]: ../examples/laptop.nix
[4]: decisions/0008-own-keys-and-a-tpm-sealed-disk.md
[5]: ../modules/ssh-tpm-agent/home.nix
[6]: ../modules/tpm-signing/home.nix
