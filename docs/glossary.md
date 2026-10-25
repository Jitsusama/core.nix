# Glossary

What every thing on Joel's machines and in their tooling is called: one word
for each thing, and one thing for each word, used the same way in code,
directories, docs, on screen and in what agents say. [The design][design] says
how things look and behave; this says what they're called. A word that isn't
here yet hasn't been settled, and a word under [Still to Settle](#still-to-settle)
means two things today.

Each table has the same columns:

- **Word**: the thing's name, as it's written in prose and shown on screen.
- **Means**: what it is, in a few plain words.
- **Also**: other words that find it, as when typed into the launcher, but are
  never written as its name.
- **Not**: names it replaces, which are never written for it.
- **In code**: how code spells it, where code has to spell it otherwise, such
  as a program's name for a module or NixOS's own option.

The-glossary-names-each-thing-once holds every table to that: no word is the
Word, an Also or a Not of two things.

## The Library

The words core.nix is built from.

| Word               | Means                                          | Also | Not  | In code                                                 |
| ------------------ | ---------------------------------------------- | ---- | ---- | ------------------------------------------------------- |
| machine            | one computer                                   |      | host | `nixosConfigurations.laptop`; `hostName`, which NixOS requires |
| machine repository | a repository of machines built on core.nix     |      |      | any flake with core.nix as an input                     |
| module             | one program or subsystem, configured           |      |      | `modules/bat/` becomes `homeModules.bat`                |
| role               | what a machine is for                          |      |      | `roles/workstation/` becomes `nixosModules.workstation` |
| hardware           | one machine model, and what it needs           |      |      | `nixosModules.dell-xps-14-da14260`                      |
| layer              | what a machine repository adds to its machines |      |      | the machine repository's own modules                    |
| identity           | who Joel is on a machine                       |      |      | `jitsusama.identity.email`                              |
| theme              | the look every program Joel sees shares        |      |      | `jitsusama.theme.roles.accent`                          |
| account            | the user home-manager configures               |      |      | `home-manager.users.<account>`                          |

## The Machine and Its Parts

What a machine is made of, and the programs that look after each part.

| Word | Means | Also | Not | In code |
| ---- | ----- | ---- | --- | ------- |
| firmware | the program in the machine's chip that starts it, and its settings | | BIOS, UEFI | |
| firmware password | the password that keeps the firmware's settings shut | | BIOS password | |
| disk | the machine's storage | | drive, SSD, storage | NVMe; `jitsusama.disk`, from disko |
| boot partition | the disk's small first part, which the firmware starts from | | ESP | `/boot` |
| filesystem | how a disk's room is kept as files | | | Btrfs, a subvolume |
| disk guard | the program that keeps each filesystem from filling: one for the machine, one for the account | | | `disk-guard system`, `disk-guard user` |
| freeze | the disk guard stopping a program that fills a filesystem, until there's room | frozen | | `SIGSTOP` |
| thaw | letting a frozen program carry on | | | `disk-guard thaw` |
| memory | what programs run in while the machine is on | RAM | | |
| swap | memory set aside out of the way, compressed first, then in the swap file | | swapfile | zram |
| memory pressure | how long programs are waiting for memory | | | PSI |
| slice | a share of the machine's CPU, memory and disk that a group of programs runs in | | | a systemd `.slice` |
| shaper | the program that keeps the network's queue on the machine, sorted by slice, so a call stays quick | | | `shaper`, CAKE |
| connection | the network link out of the machine | uplink | line | |
| tunnel | a VPN's link through the network | VPN | | `jitsusama.shaper.tunnels` |
| Wi-Fi | the network without a cable | wireless | WLAN | `wifid`, `wifi` |
| network sign-in page | the page a network shows before it lets the machine out, as a hotel's does | | captive portal | `wifid-sign-in` |
| firewall | what keeps others' connections out | | | nftables |
| Bluetooth | the radio that reaches headphones, keyboards and mice | | | BlueZ |
| power profile | how hard the CPU and firmware run: performance, balanced or power saver | | thermal mode, platform profile | power-profiles-daemon |
| charger | the power cable, and what it's plugged into | | mains | AC |
| sleep | the machine paused with its memory kept, until it wakes | | suspend | `systemctl suspend`, s2idle |
| wake | coming back from sleep | | resume | |
| lid | the laptop's lid, whose closing puts it to sleep away from a desk | | | `lid-docked` |
| docked | the laptop at a desk with a monitor plugged in, kept awake with its lid shut | at a desk | on a dock | |
| image processor | the part turning the built-in camera's raw picture into an image | | ISP, IPU | |
| IR camera | the infrared camera beside the built-in camera, for signing in by face | | | `ir-camera`, Howdy |
| desk light | the light on the monitor, lit while a camera is in use | | | Litra |
| login key | the SSH key that signs Joel in to other machines, kept in the TPM | | everyday SSH key | `id_ecdsa.tpm`, ssh-tpm-agent |
| spare | the login key's copy on the YubiKey, for when the TPM can't be used | | | `<account>@yubikey` |
| Joel's key | the key that signs Joel's own commits | Joel's signing key | | `joel-signing`, `joelKey` |
| agents' key | the key that signs agents' commits | agents' signing key | agent's key | `agent-signing`, `agentKey` |
| passkey | a key the TPM holds for signing in to websites | | FIDO security key | `linux-id` |
| keyring | where passwords programs keep are held, opened at sign-in | | | gnome-keyring |
| greeter | what asks Joel to sign in when the machine starts | | | greetd, tuigreet |
| console | the text screen the machine starts on, and the ones past Ctrl+Alt+F2 | | | kmscon |
| splash | the picture shown while the machine starts and stops | | boot screen | Plymouth |
| lock screen | what hides the desktop while Joel is away, until he signs in again | | | swayidle |
| device check | the program that tells Joel a device stopped working | | | `device-check`, smartd |
| virtual machine | a computer run as a program on this one | VM | | libvirt, QEMU |
| container | a program run apart from the machine, with its own files | | | Podman |

## Still to Settle

Words that mean two things today. Each waits on one choice, and moves into a
table above once it's made.

- **shell**: zsh, as this glossary used to say, and the desktop's own windows
  (the launcher, notices and the margin), as most of the code and [the
  design][design] say.
- **layer**: what a machine repository adds, and a keyboard's layer.
- **role**: what a machine is for, and what a colour means (`jitsusama.theme.roles`).
- **identity**: who Joel is, and an age identity, the file that opens what was
  sealed to it.
- **module**: one of core's modules, a kernel module, and the camera module.
- **machine**: one computer here, and `Computer` in the launcher.
- **the kinds of work**: what Joel waits on at a terminal, timed work, agents'
  work and the desktop's each run in their own slice, but the set has no
  name, and "jobs" names both the agents' slice and pi's background
  commands.
- **how full a disk is**: the disk guard says "nearly full" and "critically
  full" on screen and `urgent` and `critical` in code, and calls the band
  below them "pressure", which is also memory's word.
- **TPM**: the chip that keeps the machine's keys is shown by its acronym;
  a plain word for it is still to be chosen.

[design]: design.md
