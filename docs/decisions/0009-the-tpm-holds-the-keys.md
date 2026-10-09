# 0009: The TPM Holds the Keys, the YubiKey Brings Them Back

## Status

Accepted.

## Context

Joel's keys used to live on the YubiKey, as an OpenPGP key that signed his
commits and opened his backups. That made the YubiKey part of every day: it
had to be plugged in and touched for work that a chip inside the laptop can
do. Joel asked that the TPM do the everyday work and that the YubiKey serve
only as the way back in when the TPM is wiped or the laptop is gone, through
FIDO2 and nothing heavier.

## Decision

The TPM holds every everyday key, and none of them can leave it: the disk's
(decision 0008), the SSH key that logs in, the keys that sign commits, and an
age identity for secrets. ssh-tpm-agent serves the SSH and signing keys; a
PIN unlocks them once a session, through a prompt drawn in the theme.

The YubiKey holds a spare of each, all through FIDO2: the disk's recovery
slot, a resident `ed25519-sk` SSH key that `ssh-keygen -K` brings back onto
any machine, and an age identity through hmac-secret, so the backups open
without the laptop. It also signs Joel in through single sign-on. Its
OpenPGP, PIV and OATH applications go unused, so the machine runs no pcscd
and no gpg-agent.

## Alternatives

- **The YubiKey's OpenPGP key, as before.** It works on every machine, but
  needs the YubiKey for every signature and every backup, and GnuPG's agent,
  smart card daemon and pinentry to reach it.
- **PIV on the YubiKey for age.** age-plugin-yubikey is the better-known
  plugin, but PIV needs pcscd and a second PIN, where FIDO2's hmac-secret
  needs neither.
- **Keys in files, encrypted with a passphrase.** They can be copied off the
  disk and guessed at elsewhere; a key in the TPM can only be used on this
  laptop, and the TPM stops guessing at its PIN.

## Consequences

- A wiped TPM loses the everyday keys for good. The spares on the YubiKey
  get Joel back in, and new TPM keys replace the old ones.
- Everything that names a key has to name both: GitHub gets the TPM's and the
  YubiKey's SSH keys, and the backups are encrypted to both age identities.
- The backups that are still encrypted to the OpenPGP key move to age before
  the YubiKey's OpenPGP key can be retired.
