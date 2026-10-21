# Testing

Checks are for the loop of making a change: each one has to tell whoever is
changing core.nix something they need to know, quickly, and something no
cheaper check already tells them. Every check is defined in
[`checks.nix`][1], and nothing runs on a schedule.

## The Loop

```sh
nix build .#checks.x86_64-linux.<name>   # the checks the change touches, as you go
nix flake check --keep-going             # everything, before pushing
```

`nix flake check` runs every check, but Nix reuses any check whose inputs
didn't change, so a change pays only for the checks it can affect. With the
builds already done, what's left is evaluation, under a minute on a 16-core
laptop busy with other builds.

`--keep-going` lets every check finish when one fails, so the run names all
the failures rather than the first. Determinate Nix needs it besides: when a
check fails while others are still building, it cancels them and then aborts
on an assertion (`buildResults.size() == toBuild.size()`) before naming the
check that failed. A change to the boot path reruns the install test; a change
to a theme reruns the programs that read it and the desktop test.

To try a change on a real machine, build it from its repository with core
pointed at the checkout, and switch to it once it builds:

```sh
nixos-rebuild build --flake .#<machine> --override-input core path:$HOME/src/core.nix
```

CI runs the same checks on every pull request and on main, as a backstop for
a push that skipped them. Nothing waits on it: a machine repository can pin a
branch's commit as soon as it's pushed, and pull requests merge with a merge
commit, so that commit stays on main.

## The Pyramid

From the bottom up, each tier costs more than the one below and has to show
something it can't.

### Promises

Read from evaluated machines, without building them. They catch an option
that changed meaning, a role that brings what it shouldn't, or a setting that
quietly stopped holding.

| Check                               | Proves                                                    |
| ----------------------------------- | --------------------------------------------------------- |
| `nixos-role-twice-is-once`          | importing a NixOS role twice leaves the machine unchanged |
| `darwin-role-twice-is-once`         | the same for a nix-darwin role                            |
| `disabling-a-module-removes-it`     | `disabledModules` removes a module a role brought in      |
| `one-ssh-agent`                     | a workstation with a screen answers SSH with the TPM only |
| `onepassword-works-with-chrome`     | 1Password has its Chrome helper and unlocks with polkit   |
| `yubikey-on-every-workstation`      | Linux and Mac workstations both have the YubiKey's tools  |
| `ergodox-on-every-workstation`      | every workstation can flash the ErgoDox, a Mac as much as Linux |
| `zsh-is-the-login-shell`            | an account on NixOS logs in to zsh, as one on a Mac does  |
| `one-ruleset-for-every-program`     | the firewall, libvirt and Podman share nftables, and a reload flushes none of them |
| `disks-go-by-their-name`            | a disk's partitions and open volume carry its name        |
| `batteries-leave-time-to-power-off` | UPower powers a laptop off at 5%, before it goes flat     |
| `the-profile-saves-power-on-a-low-battery` | the power profile follows the charger, goes to power saver below 20% on battery, and holds a profile picked by hand |
| `disks-are-scrubbed`                | the disk's Btrfs is scrubbed monthly, from its root       |
| `booted-generation-stays-installed` | a switch never removes the generation the machine booted  |
| `session-memory-is-protected`       | every slice above session.slice carries a hard memory floor |
| `a-faint-theme-is-refused`          | a theme with text under WCAG's 4.5:1 doesn't build        |
| `quickshell-restarts-with-its-files` | a switch restarts the shell when a file it reads changes  |
| `quickshell-opens-programs-through-niri` | programs the shell opens survive the shell restarting |
| `one-niri` | the screens go off through the machine's patched niri, not a second one |
| `the-overview-takes-its-pieces` | a machine moves, cuts, retunes and adds the overview's pieces |
| `quickshell-takes-its-shape-from-the-theme` | every corner and border in the shell is the theme's |
| `icon-marks-past-the-basic-plane` | an icon mark past U+FFFF reaches the shell whole |
| `nix-builds-yield-to-the-user` | Nix's builds run at a lower weight than the user's programs |
| `nix-collects-before-the-disk-fills` | Nix collects garbage during a build that fills the disk |
| `resolved-is-set-per-network` | a laptop leaves resolved's global DNS settings as they ship |
| `the-shutdown-mark-cannot-stop-a-reboot` | the mark a healthy stop leaves can't fail the reboot, and what reads it outlives the last stage |
| `thermald-starts-after-its-drivers` | the XPS 14's thermal drivers load before thermald starts, and a thermald that stops is restarted |
| `firmware-leaves-the-keyboard-light` | the XPS 14 leaves the keyboard light to its key, still holding its other firmware settings |
| `chromium-knows-the-panels-colours` | the XPS 14's Chrome, Slack and 1Password convert their colours to its panel's P3 |
| `the-panel-slows-on-a-low-battery` | the XPS 14's panel drops to 60 Hz below 30% on battery, holds there, and comes back to its fastest rate on the charger |
| `the-camera-takes-its-modules-tuning` | the XPS 14's camera reads Dell's tuning for the module a machine names, and Intel's otherwise |
| `the-surface-keyboard-types-at-the-disk-prompt` | the Surface Laptop 4's initrd has every driver on its keyboard's path, and keyd lays that keyboard out there |
| `the-click-strength-holds-after-sleep` | a machine naming a click strength sets it as each hidraw device appears and after every sleep, and one naming none runs nothing |
| `disk-guard-stays-in-the-background` | the disk guard yields to the account, and a command overrunning its memory never stops it |
| `the-mac-guards-its-disk` | a Mac's guards run under launchd, with a Mac's paths and nothing to confine a run |
| `spend-watch-stays-in-the-background` | the AI spend watch yields to the account, comes back when it fails, and leaves its status behind when it stops |

### The Shell's Logic

Run in Qt's own JavaScript engine, the one the shell runs it in, over cases
written beside each in `tests/`.

| Check                                 | Proves                                                  |
| ------------------------------------- | ------------------------------------------------------- |
| `the-launcher-finds-what-is-meant`    | the right row comes first for each query, within budget |
| `the-launcher-finds-emoji`            | emoji are found by name and meaning, best first         |
| `the-launcher-finds-the-keyboards-keys` | a key is found by what it types, and shown where it is |
| `the-launcher-answers-what-is-asked`  | sums, addresses, measures, zones and colours answer     |
| `the-music-follows-the-player`        | the music is the player that started last, never playerctld's echo |
| `the-disk-piece-reads-the-guards`     | the disk piece says the worst disk as the guards wrote it, and a guard gone quiet |
| `the-shell-reads-the-spend-watch`     | the AI spend piece and collection draw what the watch wrote, put what wants a look first, step into each figure's parts, hide amounts while sharing, and answer reports by their words |
| `palette-rows-keep-their-words-apart` | a palette's row and field keep their words apart and inside them, however long, ending what doesn't fit in an ellipsis |
| `a-claimed-chord-is-refused`        | a chord two layers claim, or one claims twice, is refused |
| `a-theme-changes-everything`        | another theme leaves none of the first's colours behind   |
| `nothing-names-its-own-look`        | no module but the theme's names a colour or a face        |
| `formatting`                        | everything is formatted and passes the linters            |
| `stands-alone`                      | nothing here names a machine, its repository or employer  |
| `ci-runs-every-vm-test`             | CI's workflow runs every virtual machine test             |

### Examples

Whole machines that use core only through its outputs, as a machine
repository does. Between them they import every role, every module and the
hardware core knows, so a module that sets an option that doesn't exist, or
relies on one nothing brings, fails here.

| Check            | Proves                                                     |
| ---------------- | ---------------------------------------------------------- |
| `example-nixos`  | a NixOS workstation with a screen evaluates                |
| `example-laptop` | a laptop on the XPS 14, its disk and Secure Boot, evaluates |
| `example-darwin` | a Mac workstation evaluates                                |

### Programs

The real program reads the files an account gets, because a program that
can't use its configuration usually falls back to its own without a word.
The kernel checks build only its configuration: the patched source, run
through Kconfig, which drops a setting whose dependencies aren't met. [The
kernel guide][2] says what that catches.

| Check                             | Proves                                                    |
| --------------------------------- | --------------------------------------------------------- |
| `kernel-settings-hold`            | the kernel's patches apply and every setting holds        |
| `kernel-settings-hold-<model>`    | the same for the kernel that hardware builds              |
| `speaker-amps-have-every-dai-named` | the XPS 14's SoundWire machine driver names only DAIs its amplifiers register |
| `bluetooth-keeps-its-power-limits` | the XPS 14's kernel reads Bluetooth's revision 2 SAR table and antenna gain setting from UEFI |
| `the-panel-offers-variable-refresh` | the XPS 14's kernel reads the panel's refresh range from DisplayID's Adaptive Sync block |
| `the-panel-knows-its-luminance` | the XPS 14's kernel reads the panel's luminance range from DisplayID's Display Parameters block |
| `platform-profiles-reach-the-legacy-file` | every profile Dell's driver offers, the XPS 14's SoC slider takes too |
| `ipu7-outlives-its-open-files` | the XPS 14's capture side is freed when its last node closes, not when the IPU unbinds |
| `the-camera-holds-no-memory-it-never-uses` | neither half of the XPS 14's camera asks at probe for a firmware log only Intel's offline logger writes, and the capture side frees its firmware's memory when it unbinds |
| `niri-accepts-its-configuration`  | niri loads the files an account gets                      |
| `niri-cursor-has-its-shapes`      | the cursor theme niri names has the shapes programs ask for |
| `kitty-accepts-its-configuration` | kitty loads its files without a complaint                 |
| `btop-draws-in-the-theme`         | btop draws in the theme's colours, not its own            |
| `gimp-accepts-its-configuration`  | GIMP reads its preferences, the theme's palette and colours |
| `inkscape-accepts-its-configuration` | Inkscape starts a drawing on the theme's desk and reads its stylesheet |
| `spotify-draws-in-the-theme`      | Spotify's client is built in the theme's colours and faces |
| `spotify-player-accepts-its-configuration` | spotify-player starts with its settings          |
| `tuple-shares-through-the-portal` | Tuple reads the portal setting where the switch writes it |
| `git-settings-stay-writable`      | programs write git's settings and the declared ones still win |
| `neovim-starts-in-the-theme`      | Neovim starts in bamboo, transparent, without a complaint |
| `cargo-links-with-mold`           | cargo links an account's Rust builds with mold            |
| `the-ergodox-layout-builds`       | the layout fits the board and answers with its own serial |
| `the-ergodox-command-reaches-the-board` | `ergodox` writes each command whole to the board's channel alone |
| `the-pinentry-asks-the-shell` | linux-id's pinentry asks the askpass in the program's words |
| `the-ergodox-watch-tells-the-layer` | the layer watcher finds the board and says only what it's told |
| `the-click-strength-finds-the-intensity` | the click strength's setter finds a haptic pad's intensity, the XPS 14's among them, and writes only it, only when it differs |
| `btrfs-reclaims-its-chunks`       | tmpfiles turns Btrfs's reclaim on for data and metadata   |
| `the-firmware-takes-what-is-declared` | `firmware-settings` names what differs, writes it with the password, then wipes it |
| `devices-that-stop-working-are-named` | `device-check` names a failed or missing device, or a disk's fault, once, and again when it's fixed |
| `kernel-trouble-is-named` | `device-check` names a kernel that oopsed, locked up, saw a hardware fault or held a task stuck, once, and not its harmless taints |
| `disks-say-when-they-fail` | a workstation runs smartd on its disks and hands their faults to device-check, not to every terminal |
| `smartd-stops-cleanly-without-disks` | the workstation's own smartd command, run with no disks, stops cleanly rather than failing |
| `disk-guard-accepts-its-configuration` | the disk guard passes its scenarios and accepts the configs a workstation and its account get |
| `disk-guard-passes-clippy`        | clippy finds nothing in the disk guard or its scenarios   |
| `disk-guard-compiles-for-macos`   | the disk guard and its scenarios compile for macOS, clean |
| `network-revive-passes-clippy`    | clippy finds nothing in network-revive or its tests       |
| `guide-proves-its-steps`          | the guide counts a step done only once observing it again says so, reads what real machines' tools printed, writes sticks that sfdisk, fsck.fat and their own key read back whole, signs EFI programs byte for byte as sbsign does, and finds every program it runs |
| `guide-passes-clippy`             | clippy finds nothing in the guide or its tests            |
| `guide-compiles-for-macos`        | clippy finds nothing in the guide as macOS compiles it     |
| `ir-camera-passes-clippy`         | clippy finds nothing in ir-camera or its tests            |
| `record-window-passes-clippy`     | clippy finds nothing in record-window or its tests        |
| `plymouth-waits-passes-clippy`    | clippy finds nothing in plymouth-waits or its tests       |
| `the-splash-keeps-systemd-quiet`  | with the splash, systemd writes only failures to the console |
| `spend-watch-accepts-its-configuration` | spend-watch passes its tests and accepts the config an account gets, a feed and a cap among it |
| `spend-watch-passes-clippy`       | clippy finds nothing in spend-watch or its tests          |

### Virtual Machines

Each boots a machine under QEMU, which takes minutes, so each covers what only
a running system shows.

| Check                             | Proves                                                  |
| --------------------------------- | ------------------------------------------------------- |
| `vm-secure-boot-installs`         | the whole install works, and the disk opens only for it |
| `vm-boot-asks-in-the-theme`       | the splash asks for the disk, names a slow job, and lasts a restart to its end |
| `vm-guide-installs`               | the guide, driven by its keys, installs a machine from its stick: two keys and a recovery key, Secure Boot enforcing with its keys |
| `vm-guide-writes-a-stick`         | the guide writes a stick as the person, through UDisks2, unmounting what was mounted from it |
| `vm-commits-are-signed`           | commits are signed with the TPM's keys, PINs asked for  |
| `vm-passkeys-come-from-the-tpm`   | passkeys and security keys are made in the TPM, each after a yes |
| `vm-keyring-opens-without-asking` | the keyring opens with its TPM-sealed password, unasked |
| `vm-speakers-are-tuned`           | the XPS 14's speakers hear the tuning, headphones don't |
| `vm-desktop-works`                | it boots clean; the desktop shows, opens, locks and asks |
| `vm-greeter-signs-in`             | the greeter signs in and out, not waiting on the account's setup |
| `vm-wifi-joins`                   | wifid joins, nudges a stuck look, finds sign-ins, keeps others' routing rules |
| `vm-disk-guard-keeps-the-disk`    | the guards answer each band, sparing what's in use       |
| `vm-network-revives-units`        | what the network let down starts, and reloads on change  |
| `vm-clock-keeps-time`             | the clock sets from the machine's servers past a network's silent one |
| `vm-libvirt-stops-quickly`        | libvirt's daemon stops at once, its udev thread starved  |
| `vm-ir-camera-opens-for-readers`  | the IR camera waits for udev and its loopback, opens only while read, its frames grey, no frame from before a reader opened |
| `vm-swap-comes-down-while-probed` | the compressed swap comes down, its reset waiting out udev's probe |
| `vm-reboot-cannot-hang`           | a reboot forces on after a minute of a service not stopping, the watchdog resets a stuck last stage, and Plymouth hears which |

The install test builds a disk image with disko and boots it with a software
TPM, then follows [the install guide][3] step by step: the keys are made and
enrolled, the disk is bound to the TPM and a PIN, the passphrase is wiped,
and the TPM refuses once a measured PCR changes. On the way it checks that
enrolling without checksums refuses the option ROM of QEMU's network card,
which the firmware runs, and that no NvPCR is set up at boot or measured at
login. It lives in [`tests/secure-boot.nix`][4].

### What Changed on Screen

The desktop test photographs every state it puts the desktop in, each once
the screen holds still, and keeps the pictures in its result. No check
compares them with stored ones: a stored picture would fail on every change
meant, and two runs of the same code don't draw every picture the same, as a
clock or a notice's age moves on. Instead, a change to how anything looks is
compared with main, and every picture it changed is looked at:

```sh
desktop=checks.x86_64-linux.vm-desktop-works
main=$(nix build --no-link --print-out-paths "github:Jitsusama/core.nix#$desktop")
here=$(nix build --no-link --print-out-paths ".#$desktop")
nix develop -c shots-changed "$main" "$here" changed
```

`shots-changed` names each picture that changed, most first, by how many
pixels, and draws it in `changed/` before, after and their difference, cut
to where it changed. A picture that should have changed and isn't named
matters as much as one named that shouldn't have. The pictures that move by
themselves are named in [`tests/shots-that-move.txt`][5], each with what
moves in it, and listed apart. One joins that list only once two runs of
unchanged code show it moving, for a cause the test can't hold still; a
picture caught before its state was drawn is a fault in the test instead.

## Earning a Check

- A new check goes in the lowest tier that can show what it covers. Most
  belong with the promises.
- A check above the promises says what the tiers below it miss.
- What a running system shows goes in an existing virtual machine where it
  fits. A new one has to cover something none of them can, and its name
  starts with `vm-`, which is how CI gives it a runner of its own.
- A check that only repeats another one's signal goes.
- A new module is evaluated by the examples once a role or another module
  imports it. One that nothing imports goes into an example.

## Checking a Machine Repository Against a Change

The machine repositories evaluate their machines in their own checks. To see
what a change here does to one, compare the derivations it produces before
and after:

```sh
nix eval --raw --override-input core path:$HOME/src/core.nix \
  .#darwinConfigurations.<machine>.config.system.build.toplevel.drvPath
nix run nixpkgs#nix-diff -- before.drv after.drv
```

A refactor should leave the path unchanged. Anything else shows up in the
diff, file by file. A list in a different order is still a change, and needs
explaining before it lands.

## Trusting a New Check

Before relying on a new check, break what it covers and watch it fail. Undo
the break from a saved copy of the file, not from git, so that a staged break
isn't restored by accident.

[1]: ../checks.nix
[2]: kernel.md
[3]: installing.md
[4]: ../tests/secure-boot.nix
[5]: ../tests/shots-that-move.txt
