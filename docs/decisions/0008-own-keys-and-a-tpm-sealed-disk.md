# 0008: Only Joel's Keys, and the TPM Opens the Disk

## Status

Accepted.

## Context

Joel carries his laptops everywhere, so one can be stolen or left alone with
somebody. A thief should get nothing from the disk, and somebody with the
laptop for an hour shouldn't be able to make it boot something that learns
the PIN. Joel also asked that the TPM do the work every day, with the YubiKey
needed only for recovery.

## Decision

Every laptop uses Secure Boot through lanzaboote, with keys the machine makes
on its first boot and systemd-boot enrols on its second, so they never exist
anywhere else. Microsoft's keys stay out; the firmware still runs the option
ROMs it found when the keys went in, by their checksums from the TPM's event
log.

The disk is LUKS2, unlocked by the TPM with a PIN. The TPM releases its key
only through a systemd-pcrlock policy over PCRs 0, 4 and 7: the firmware, the
boot loader and lanzaboote's signed stub (which covers the kernel, initrd and
command line), and the Secure Boot keys and state. lanzaboote rewrites the
policy after every rebuild. The YubiKey (FIDO2) and a recovery key kept off
the machine are the only other ways in; the install passphrase is wiped.

## Alternatives

- **Microsoft's keys beside Joel's,** lanzaboote's default. Anything Microsoft
  ever signed would boot, including shims and old boot loaders still waiting
  for the revocation list to catch up, and any of them can show a fake disk
  prompt. A laptop draws its picture with its firmware, so it doesn't need
  them for a graphics card's ROM, and the setup screen can always restore the
  factory keys.
- **The TPM bound to PCR 7 alone,** the usual setup. Anything signed with
  Joel's keys would get the disk's key, such as an older generation with a
  hole since fixed. PCR 4 limits it to the generations lanzaboote installed.
- **The TPM without a PIN.** The disk would open for anyone who powers the
  laptop on, leaving only the running system's login between a thief and the
  data.
- **The YubiKey every boot.** As strong, but it has to be carried and plugged
  in every time; Joel asked for the TPM day to day.
- **Keys made elsewhere,** on another machine or a YubiKey. They'd have to be
  copied to every machine that signs its own generations, and each copy is
  one more place to steal them from.
- **Kernel lockdown as well.** It refuses modules the kernel's build didn't
  sign, which the camera's out-of-tree drivers aren't, and it blocks
  hibernation. Secure Boot with the TPM and PIN already covers a stolen or
  tampered laptop; lockdown can come back once the camera's drivers are in
  the kernel.

## Consequences

- A firmware update changes PCR 0, so the next boot needs the YubiKey once;
  that boot measures the new firmware and the PIN works again after it.
- systemd-pcrlock covers at most four generations, so lanzaboote keeps four,
  and one of them is always the generation the machine booted. pcrlock can
  only write a policy with PCR 4 while it can match the running boot's
  measurements to an installed generation, so when a fifth switch without a
  reboot removed the booted one, the disk was left sealed to PCRs 0 and 7
  until the next boot. Keeping it takes one of the four places in the boot
  menu; the generation it displaces can still be switched to.
- systemd still calls pcrlock experimental, so every disk always has the
  recovery key.
- A machine with a graphics card needs Microsoft's keys, and says so in its
  own settings.
- Reinstalling makes new keys, so the firmware goes back to setup mode first.
- `vm-secure-boot-installs` walks the install on a virtual machine with a
  TPM, so a lanzaboote, disko or systemd update that breaks any step fails
  its checks.
