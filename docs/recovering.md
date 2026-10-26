# Recovering

What to do on a bad day, for a laptop [installed][1] with Secure Boot on Joel's
own keys and a disk the TPM opens. Each section starts from what happened:
the disk won't open, the TPM or the firmware's keys were cleared, a new
generation won't start, nothing boots, a YubiKey is lost, the disk dies, the
laptop is gone, or 1Password won't open. A machine's own repository adds
what's particular to it, such as anything an employer manages.

## What Makes It Recoverable

None of these is needed day to day. Each is a way back when the everyday one
is gone, so each lives somewhere other than the laptop, but for a YubiKey
left plugged in, which is lost with it. The recovery key is the seven words
the guide made, kept in 1Password under the machine's name and "Recovery
Phrase", or systemd's own from an install by hand. The stick's passphrase is
a second set of seven words, the stick's own rather than any machine's, kept
as "NixOS Install Phrase": it opens only the stick's vault, and replaces
nothing.

| Piece               | Where it lives                 | What it's for                        |
| ------------------- | ------------------------------ | ------------------------------------ |
| Recovery key        | off the laptop                 | opens the disk when nothing else can |
| Stick's passphrase  | off the laptop                 | opens the stick's vault, to install  |
| Each YubiKey        | on Joel, one apart from it     | opens the disk; holds the SSH spare  |
| The machine's stick | apart from the laptop          | recovers or installs the machine     |
| Firmware password   | kept off the laptop            | keeps Secure Boot on                 |
| Machine repository  | where it's hosted              | builds the machine again             |
| 1Password's code    | printed, with its Emergency Kit | opens 1Password when nothing else can |

The machine's stick is the one its repository writes with `nix run .#guide`
(core's `lib.guide`), on any x86-64 Linux machine. It carries the machine's
system, so an install needs no network, and its rescue bundle: the machine's
Secure Boot keys, sealed to its YubiKeys and its recovery key, so a
reinstall keeps them and never needs the firmware cleared. Any stick the
guide wrote can recover a machine; only the machine's own can install it.

## The Disk Asks Again After the PIN

The TPM didn't release its key, so the prompt has moved on to the YubiKey.
Type the YubiKey's PIN and touch it, and the disk opens. On a machine the
guide installed that's the same PIN: the guide seals the TPM with the
YubiKeys' PIN, so a plugged-in YubiKey is tried with what was typed and
only asks for a touch. [When the TPM
Refuses][2] says why that happens and how to seal the disk to the TPM again.
Anything it doesn't name means the laptop didn't boot what it should: find
out why before typing the recovery key.

## The TPM Was Cleared

A cleared TPM holds none of what it did: the disk's seal is gone, and so are
the keys it kept, which it can never load again. The disk opens with a
YubiKey and its PIN, as [above][9]. At that start the installed guide seals
the disk to the TPM again, then makes the TPM's SSH and signing keys anew
under a new PIN, saying why on screen. GitHub still holds the old public
keys, so replace them as when [the disk dies][4]: the SSH key and both
signing keys on GitHub, and the signing keys in the machine repository's
`jitsusama.signing.allowedSigners`.

A machine the guide didn't install does neither by itself: seal it again as
[When the TPM Refuses][2] says, and make the keys as in [Make the SSH
Keys][3].

## The Firmware Lost Its Secure Boot Keys

Nothing to do but have a YubiKey plugged in. The installed system enrols its
keys again by itself over the next restarts. The TPM refuses those boots,
since the firmware doesn't match what it was sealed to, so each asks for the
YubiKey's PIN. Once the keys are back, the TPM opens the disk again.

## A New Generation Won't Start

Nothing to do at first. A generation that never reaches multi-user.target
three times running is passed over, and the newest one that did boots
instead, saying which failed once the session starts. To pick one by hand,
hold Space at power-on for the boot menu.

Once it's running, fix the configuration and switch, or keep booting the
good one until then:

```sh
sudo bootctl set-default <entry>   # bootctl list names them
```

## Nothing Boots

Boot a stick the guide wrote: plug it in with the laptop off and start it
from the stick, as the guide's last screen said when the stick was written. The
machine's own stick is signed with the machine's keys, so it boots with
Secure Boot on. Another stick needs Secure Boot turned off in the firmware's
setup, which asks for its password; turn it back on before booting the disk
again, or the TPM refuses that boot.

A repair stick starts recovering when it boots, and `guide recover` starts it
from any stick's shell. It shows the disk it found before anything is typed,
opens it with the YubiKey's PIN or the recovery key, never the TPM, and
mounts everything under /mnt. Then it offers, coming back after each:

- a root shell inside the installed system;
- which generation boots next;
- the boot loader installed again, signed with the machine's keys;
- a lost YubiKey replaced (below).

## A YubiKey Is Lost

The disk still opens with the TPM and the PIN, so nothing is urgent but the
accounts the YubiKey signs in to.

1. From a stick the guide wrote, `guide recover` replaces it: it tells the
   YubiKey still held from the lost one, enrols the new one and proves it,
   and only then wipes the lost one's slots, so the disk never has fewer
   ways in.
2. Turn the new YubiKey's OTP off and make its SSH spare, as in [Make the SSH
   Keys][3]: `ykman config usb --disable OTP`, then `ssh-keygen -t
   ed25519-sk -O resident -O verify-required`.
3. On GitHub, delete the lost YubiKey's SSH key, the one named
   `<account>@yubikey`, and add the new one's.
4. Every site where it held a passkey: delete that passkey and add the new
   YubiKey's. The other YubiKey signs in meanwhile.
5. Anything sealed to the lost YubiKey: take its recipient out of the machine
   repository, put the new one's in, and switch, so everything is sealed
   again without it.

## The Disk Dies, or Is Wiped

Fit the new disk and boot the machine's own stick. It installs the system it
carries, with no network, and puts the Secure Boot keys back from its rescue
bundle, so the firmware needs no visit. It makes a new recovery key for the
new disk: keep its seven words in place of the old key.

An install that stops part way, say from a flat battery, isn't lost. Boot
the stick again and it says the install stopped part way, offering to carry
it on or start again, wiping the disk. Carrying on asks for a YubiKey the
install had already enrolled, and its PIN, then goes on from where it
stopped. If it stopped before any YubiKey was enrolled, nothing opens that
disk, so the stick says so and offers only to start again, wiping the disk.

The TPM's keys were files on the old disk, so the first boot makes new ones:
the everyday SSH key and both signing keys. GitHub still holds the old public
keys until they're replaced:

1. Delete the old `<account>@<host>` SSH key and the two signing keys
   (`<account>@<host> signing` and `<account>@<host> agents`) from GitHub,
   and add the new ones.
2. Put the new signing keys in the machine repository's
   `jitsusama.signing.allowedSigners` in place of the old.
3. Restore what was backed up, as the machine repository says.

## The Laptop Is Lost or Stolen

Switched off, the disk is safe. The TPM releases its key only to the same
firmware, boot loader, kernel and Secure Boot keys, and only with the PIN,
and it locks itself after 32 wrong ones, forgiving one every two hours. The
firmware's password keeps Secure Boot on, and the recovery key isn't with
it. A YubiKey plugged into it still asks for its PIN, and blocks for good
after eight wrong ones.

Taken while asleep or locked rather than off, the disk is already open and
only the lock screen stands in the way. So revoke what it holds at once, from
another machine, either way:

1. On GitHub, delete the laptop's `<account>@<host>` SSH key and both of its
   signing keys. Leave the spare of any YubiKey still held.
2. Take its signing keys out of `jitsusama.signing.allowedSigners`, so
   nothing it signs from now on counts as Joel's.
3. Take anything sealed to its TPM out of the machine repository's
   recipients, so nothing new is sealed to it.
4. Sign out of its sessions wherever the service lets you: GitHub, the
   browser's accounts, anything else it was signed in to.
5. A YubiKey that was plugged into it is lost too: [replace it][6], from
   the replacement's stick once there is one.
6. Follow the machine repository's own steps, such as telling an employer's
   IT.

Then build its replacement with the machine's stick, as when [the disk
dies][4]. New hardware's firmware trusts none of the machine's keys, so put
it in setup mode first, as [installing][5] says, and the guide makes new
Secure Boot keys there rather than putting the old ones back.

## 1Password Won't Open

The recovery key and the firmware's password are often kept in 1Password,
so being locked out of it hurts most on the day the laptop is lost too.

- **A device still unlocks it:** a phone or another computer that opens
  1Password with a fingerprint or a face keeps working. Don't sign it out or
  reinstall it, and read what's needed from it.
- **The account password or the Secret Key is gone:** an individual or
  family account comes back with its [recovery code][7]: on 1Password.com's
  sign-in page, Having Trouble Signing In?, then Use Recovery Code. It sends
  a code to the account's email, then makes a new Secret Key and account
  password. Every item stays, every device signs in again, and the recovery
  code stays good for next time. A family's organiser can recover a member
  instead; a team or business account has no recovery code, and its
  administrators recover it.
- **Neither is there:** nobody can open the account, 1Password included,
  since only the account password and the [Secret Key][8] together decrypt
  it.

So three things have to live outside 1Password:

1. The recovery code, printed with the Emergency Kit and kept apart from the
   laptop. Make one, if there isn't one, in the app's Manage Accounts, then
   Sign-in & Recovery. An account that unlocks with a passkey always has
   one.
2. The way into the account's email, since recovering needs a code sent
   there: its password known by heart or kept with the recovery code, and
   its second factor a YubiKey rather than something kept in 1Password.
3. Patience: a recovery started within an hour of signing in, or with a
   sign-in during it, is stopped, and the code then waits a day.

## Writing a New Stick

From any x86-64 Linux machine with Nix and the machine repository, with one
stick plugged in:

```sh
nix run <machine repository>#guide
```

It writes the stick as the person, never root, asking polkit for the disk,
and finishes by saying how to start that machine from it.

[1]: installing.md
[2]: installing.md#when-the-tpm-refuses
[3]: installing.md#9-make-the-ssh-keys
[4]: #the-disk-dies-or-is-wiped
[5]: installing.md#3-put-the-firmware-in-setup-mode
[6]: #a-yubikey-is-lost
[7]: https://support.1password.com/recovery-codes/
[8]: https://support.1password.com/secret-key/
[9]: #the-disk-asks-again-after-the-pin
