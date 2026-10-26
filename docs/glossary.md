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

| Word | Means | Also | Not | In code |
| ---- | ----- | ---- | --- | ------- |
| machine | one computer, which the launcher shows as Machine | computer | host | `nixosConfigurations.laptop`; `hostName`, which NixOS requires |
| machine repository | a repository of machines built on core.nix, adding its own modules | | | any flake with core.nix as an input |
| module | one program or subsystem, configured | | | `modules/bat/` becomes `homeModules.bat` |
| kernel module | a part of the kernel loaded on its own, always written whole | | | `boot.kernelModules` |
| role | what a machine is for | | | `roles/workstation/` becomes `nixosModules.workstation` |
| hardware | one machine model, and what it needs | | | `nixosModules.dell-xps-14-da14260` |
| identity | who Joel is on a machine | | | `jitsusama.identity.email` |
| age key | the file that opens what was sealed to it with age | | age identity | |
| theme | the look every program Joel sees shares | | | `jitsusama.theme` |
| colour | what a colour means in the theme, such as the accent | | colour role | `jitsusama.theme.roles`, to be renamed |
| account | the account on a machine that home-manager configures | | | `home-manager.users.<account>` |

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
| filling | the disk guard's first band, under a quarter free, when it starts taking room back | | pressure | `pressure` |
| nearly full | the disk guard's second band, under a tenth free, when it takes more | | urgent | `urgent` |
| critically full | the disk guard's last band, under 4% free, when it may freeze what fills the disk | | critical | `critical` |
| memory | what programs run in while the machine is on | RAM | | |
| swap | memory set aside out of the way, compressed first, then in the swap file | | swapfile | zram |
| memory pressure | how long programs are waiting for memory | | | PSI |
| job | work Joel or an agent starts, run below the desktop so it gives way to it | | | `jobs.slice` |
| kind of work | what a job is, which says how much the machine favours it: what Joel waits on at a terminal, timed work or agents' work | | job class | the slices `jobs-awaited`, `jobs-timed` and `jobs-work` |
| queue | heavy work, such as builds and VM tests, waiting its turn on the machine | machine queue | | |
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
| camera board | the part in the lid holding the built-in camera, which Dell fits in more than one kind | | camera module | `jitsusama.dellXps14.cameraModule` |
| image processor | the part turning the built-in camera's raw picture into an image | | ISP, IPU | |
| IR camera | the infrared camera beside the built-in camera, for signing in by face | | | `ir-camera`, Howdy |
| desk light | the light on the monitor, lit while a camera is in use | | | Litra |
| TPM | the chip that keeps the machine's keys and seals its disk | security chip | | |
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

## The Terminal

| Word | Means | Also | Not | In code |
| ---- | ----- | ---- | --- | ------- |
| shell | zsh, which Joel types commands into in a terminal | | | zsh |
| prompt | the line the shell writes before each command | | | |

## The Desktop and the Screen

What Joel sees and reaches for on the desktop.

| Word | Means | Also | Not | In code |
| ---- | ----- | ---- | --- | ------- |
| desktop | the screen and everything on it around the windows: the launcher, notices, the margin and niri's workspaces | | | niri, Quickshell |
| screen | a monitor, built in or plugged in, which the launcher lists under Screens | display, monitor | | niri's outputs |
| workspace | one of niri's rows of windows on a screen | | | |
| notice | a card at the top right saying what a program said | notification | | `Notifications.qml`, `org.freedesktop.Notifications` |
| mark | one drawn symbol, such as each one in the margin | glyph | icon | `glyphs.nix` |
| question | a centred box asking Joel something, such as a password or a yes | | | `Polkit.qml`, `Askpass.qml` |
| layer | what a keyboard's keys do, one set of several, which a key switches between | | | the ErgoDox's layers, `Layer.qml` |
| Power and modes | the launcher's row for sleep, locking, signing out and the desktop's modes | | | |
| launcher | the window on Super+Space that finds programs, windows, actions and answers | | palette, command palette | `Launcher.qml` |
| foot | the launcher's bottom line: the time, the battery and what the keys do | | | `foot` |
| margin | the row of marks at the top of the screen saying what's true now | | corner | `Margin.qml`, `overview.pieces` |
| Now | the launcher's list of what's on and waiting, on Super+O | | What's on | `WhatsOn.qml` |
| meter | a level shown while it changes, such as the volume | | overlay | `Meter.qml` |
| map | the keyboard's layout, shown on screen | | | `KeyboardMap.qml` |
| wallpaper | the picture behind every window | | painting | |
| overview | niri's view of every workspace and its windows, on Super+A | | | |
| window | a program's rectangle on the screen | | | |
| program | something Joel runs, with or without a window | | app, application | the `applications` provider |
| Keep awake | the setting that keeps the machine from sleeping or locking while it's on | | Caffeinate | |
| Do not disturb | the setting that holds notices back while it's on | | dnd, quiet | `session:quiet` |
| brightness | how bright a screen is, which follows the room's light | | | the `backlight` program |
| backlight | the keyboards' light | | | |

## The Harness

The words of the agents' tooling: quests, sessions, trees and reviews.

| Word | Means | Also | Not | In code |
| ---- | ----- | ---- | --- | ------- |
| quest | a piece of Joel's work, with its own folder, README and documents | | | `QEST-` ids, the `quest` tool |
| subquest | a quest that is part of another's work | | | `kind: subquest` |
| sidequest | a quest that came up beside another, not part of it | | | `kind: sidequest` |
| plan | a quest's document for building something, through think, draft and build | | | `PLAN-` ids |
| research | a quest's document for finding something out: question, method, findings, verdict | | | `RSCH-` ids |
| brief | a quest's document asking someone for something: audience, ask, background, recommendation | | | `BRIF-` ids |
| report | a quest's document on what happened, its outcomes and lessons | | | |
| Journey | a quest's dated log of what happened | | | `quest journal` |
| session | a pi working for Joel, in a window of its own | | | |
| peer | another session on the same machine, reached over the mesh | | instance, swarm | |
| mesh | how sessions on one machine message each other | | | `mesh_send` |
| turn | a mesh message that starts a peer's turn at once, the mesh's loudest attention level | | | |
| glance | a mesh message given to a peer as one line, since it's another peer's own business | | | |
| hold | a peer asking every other to put something off, such as a switch, until it says it's over | | | `intent: hold` |
| steer | Joel's words, delivered to a session from outside its window | | | `steer/<ADDRESS>/` |
| hand | a subagent, seen on the mesh as its parent's hands | | | `hand.ts` |
| tree | a folder holding a repository's files to work in: a worktree or a snapshot | | working tree | the `work` tool |
| worktree | a tree checked out at a branch, one session's alone | | | `git worktree` |
| snapshot | a tree pinned at a commit, read only and shareable | | | |
| checkout | a repository's clone, which trees are cut from | | | |
| cut | making a tree | | scaffold | `work tree` |
| release | giving back a tree one holds | | prune, reap | `work release`; `quest tree-prune`, to be renamed |
| tidy | reporting which trees on the machine are spent, removing nothing | | | `work tidy` |
| reclaim | taking back the trees nobody owns | | | `work reclaim` |
| scratch folder | a quest's folder for clones, data and builds, its tmp/ cleared when it concludes | | | the quest workspace |
| background command | a command a session runs without waiting on it | | | the `jobs` tool |
| fleet | subagents run at once on one piece of work | | fan-out, batch | the `subagent` tool |
| subagent | a pi a session starts to do part of its work | | sub-agent, worker, lane | |
| council | reviewing models each reading one change on their own | | | `review_ask council` |
| judge | the model that consolidates a council's findings | | | |
| finding | something a review raised | | | |
| change | work put up for review | | PR, pull request, proposal | |
| person | the human a session works for | | user, human, operator | |
| status line | the line under a session saying how it stands | | status bar | |

## Checks

How core.nix proves what it promises. A check's name is a sentence saying what
it holds, subject first, with no prefix but `vm-`, which marks a VM test and
nothing else; its description in `check-descriptions.nix` finishes the
sentence. It may name the program its module is named after, since that
says which module broke.

| Word | Means | Also | Not | In code |
| ---- | ----- | ---- | --- | ------- |
| check | one promise that `nix flake check` builds and holds | | | `checks.x86_64-linux.<name>` |
| VM test | a check that boots machines in virtual machines | | | a `vm-` name |
| subtest | a named part of a VM test, which can run alone | | | `with subtest(...)` |
| example machine | a whole machine built only from core's outputs, as a machine repository would | | | `example-nixos`, `example-laptop`, `example-darwin` |
| stand-in | something standing in for a real program or device in a check | | fake | |
| shot | a picture a VM test takes of one state of the screen | | photograph | `screendump`, `shots-changed` |
| decision record | a document saying why the design is what it is | | ADR | `docs/decisions/` |

## Still to Settle

Words that mean two things today. Each waits on one choice, and moves into a
table above once it's made.

Nothing waits now: Joel settled the last sixteen on 2026-10-25.

[design]: design.md
