# Testing

Checks are for the loop of making a change: each one has to tell whoever is
changing core.nix something they need to know, quickly, and something no
cheaper check already tells them. A check sits in the `checks.nix` of the
folder it proves something about, a module's or a hardware model's, and
[`checks.nix`][1] at the root gathers them, names the virtual machine tests,
and holds the few about the repository as a whole. Nothing runs on a
schedule.

## The Loop

```sh
nix fmt                                  # format, and lint every language strictly
nix build .#checks.x86_64-linux.<name>   # the checks the change touches, as you go
run-subtests <vm-test> <words>           # a VM test's subtests named, in seconds
changed-checks --build --no-vm           # every check the change touched, VM tests aside
nix flake check --keep-going             # everything, once, before pushing
```

While editing, a change is proven by the cheapest checks that show it,
which [AGENTS.md][6] lists by kind of change, and a VM test only by the
subtests it touches. `changed-checks`, in the dev shell, finds them all: it
evaluates every check at the change's merge base with origin's main and in
the working tree, and names each whose derivation was added, changed or
removed, or no longer evaluates. A check whose derivation is the same
proves what it proved before, so `--build` builds only the others, and
`--no-vm` leaves out the VM tests, for `run-subtests` to take a piece of.
`nix flake check` runs once, before pushing, not after each edit: on a
machine shared with other work, a whole run each time is what makes the
loop slow, and the load it adds is what makes VM tests time out.

`nix fmt` lints as it formats, at each linter's strictest, test code
included: ruff with every rule and mypy `--strict` for Python (`ruff.toml`,
`mypy.ini`), shellcheck for shell, luacheck for Lua, each file read as the
Lua of the program that runs it (`.luacheckrc`), biome with every rule that
fits for the JavaScript QML imports (`biome.jsonc`, read through
`tests/qt-javascript-lint.sh`), rumdl for Markdown (`.rumdl.toml`), zsh
itself for what it can't parse, since nothing lints zsh further, statix and
deadnix for Nix, and the `formatting` check holds the repository to it.

Two linters run in checks rather than `nix fmt`, since each needs what a
build brings. clippy reads every crate and single-file program with its
pedantic group on top of its defaults, less three lints about a published
library's API, as `lints` in `modules/clippy/clippy.nix` says; `clippy.toml` holds
its settings, and the `-passes-clippy` checks hold the code to both, which
a bare `cargo clippy` doesn't. qmllint reads the account's shell and the
greeter (`quickshell-passes-qmllint`), and each offscreen check's own
folder, its cases and stand-ins included, before running it
(`tests/shell-folder-lint.sh`). A finding that's right as it is says so
where it is, with its reason: `#[expect(..., reason = ...)]` in Rust, a
`biome-ignore` comment in JavaScript, `qmllint disable` in QML.

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

Nothing else runs them: there's no CI ([0012][7]). A machine repository can
pin a branch's commit as soon as it's pushed, and pull requests merge with a
merge commit, so that commit stays on main.

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
| `work-yields-to-what-the-user-works-in` | jobs is idle beside app.slice, pi finds it, and a window's scope is not idle |
| `work-swaps-its-own-memory`         | jobs is slowed past half of memory, with no hard limit that would end its work |
| `timed-work-goes-ahead-of-other-work` | timed work sits inside jobs above its holder, held by a unit that comes back, and pi finds it |
| `joels-work-goes-before-agents` | the sorter moves a terminal's work inside jobs, above pi's work and below timed work |
| `call-audio-runs-in-real-time` | an app's audio thread can take SCHED_RR 8, below PipeWire's, for at most 200 ms at a time |
| `machines-yield-to-what-the-user-works-in` | system virtual machines are weighed as a build is, and the account's containers and session machines are idle |
| `nix-collects-before-the-disk-fills` | Nix collects garbage during a build that fills the disk |
| `resolved-is-set-per-network` | a laptop leaves resolved's global DNS settings as they ship |
| `the-shutdown-mark-cannot-stop-a-reboot` | the mark a healthy stop leaves can't fail the reboot, and what reads it outlives the last stage |
| `thermald-starts-after-its-drivers` | the XPS 14's thermal drivers load before thermald starts, and a thermald that stops is restarted |
| `firmware-leaves-the-keyboard-light` | the XPS 14 leaves the keyboard light to its key, still holding its other firmware settings |
| `chromium-knows-the-panels-colours` | the XPS 14's Chrome, Slack and 1Password convert their colours to its panel's P3 |
| `the-panel-slows-on-a-low-battery` | the XPS 14's panel drops to 60 Hz below 30% on battery, holds there, and comes back to its fastest rate on the charger |
| `the-camera-takes-its-modules-tuning` | the XPS 14's camera reads Dell's tuning for the module a machine names, and Intel's otherwise |
| `the-camera-gives-limited-range` | the XPS 14's camera relay, run on a full-range ramp, squeezes it exactly into limited range and neither of its converters changes it after |
| `usb-camera-holders-follow-the-stream` | a USB camera's watch names its holders only while a streaming interface has a nonzero alternate setting, names its file for the camera, made a plain file name, and removes it when the node goes or the watch is stopped; a holder's name is one printable line, and only a unit in its cgroup is left out |
| `the-surface-keyboard-types-at-the-disk-prompt` | the Surface Laptop 4's initrd has every driver on its keyboard's path, and keyd lays that keyboard out there |
| `the-click-strength-holds-after-sleep` | a machine naming a click strength sets it as each hidraw device appears and after every sleep, and one naming none runs nothing |
| `disk-guard-stays-in-the-background` | the disk guard yields to the account, and a command overrunning its memory never stops it |
| `the-mac-guards-its-disk` | a Mac's guards run under launchd, with a Mac's paths and nothing to confine a run |
| `disk-guard-finds-a-machines-programs-first` | a machine's programs for the disk guard come first on its PATH under systemd and launchd, with core's git after |
| `spend-watch-stays-in-the-background` | the AI spend watch yields to the account, comes back when it fails, and leaves its status behind when it stops |
| `claude-shows-its-status-line` | Claude Code is told of its status line on Linux and macOS, in settings it can still write |

### The Shell's Logic

Run in Qt's own JavaScript engine, the one the shell runs it in, over cases
written beside each in `tests/`.

| Check                                 | Proves                                                  |
| ------------------------------------- | ------------------------------------------------------- |
| `the-launcher-finds-what-is-meant`    | the right row comes first for each query, within budget |
| `the-launcher-lands-where-it-did`     | a thing is one row, the same letters keep their row, one slip teaches nothing |
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
| `the-shell-speaks-its-javascript`   | the shell calls no method Quickshell's JavaScript lacks   |
| `formatting`                        | everything is formatted and passes the linters            |
| `stands-alone`                      | nothing here names a machine, its repository or employer  |
| `run-subtests-keeps-what-is-named`  | run-subtests keeps a script's boot and helpers and only the subtests named |
| `changed-checks-names-what-moved`   | changed-checks names a check added, changed, removed or broken by its derivation, and builds VM tests only when asked |

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
| `dell-events-are-known` | the XPS 14's dell-wmi ignores the EC's hourly type 0x0012 event rather than log an unknown key |
| `the-colour-camera-says-it-is-upside-down` | the XPS 14's camera bridge reports its colour sensor rotated 180 degrees |
| `the-npu-finds-its-compiler` | the XPS 14's graphics drivers hold the NPU's driver and Intel's compiler beside it |
| `the-window-buffers-stay-under-load` | the XPS 14's udev sets xe's `shrinker_non_purgeable_seeks` to 64, so reclaim under a build leaves window buffers alone |
| `the-panel-dims-after-a-bad-read` | the XPS 14's kernel keeps to VESA's brightness controls when its boot-time read of the panel's eDP revision comes back wrong |
| `the-gpu-security-link-loads-with-the-gpu` | the XPS 14's initrd loads mei_me and mei_gsc_proxy with xe, inside the 20 s xe waits for its CSME link |
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
| `the-litra-follows-the-cameras` | `litra` finds the lights by their ids, speaks their reports, and counts a camera streaming only by a whole file naming a program |
| `the-ergodox-watch-tells-the-layer` | the layer watcher finds the board and says only what it's told |
| `the-click-strength-finds-the-intensity` | the click strength's setter finds a haptic pad's intensity, the XPS 14's among them, and writes only it, only when it differs |
| `btrfs-reclaims-its-chunks`       | tmpfiles turns Btrfs's reclaim on for data and metadata   |
| `the-firmware-takes-what-is-declared` | `firmware-settings` names what differs, writes it with the password, then wipes it, and refuses a value it can't read |
| `devices-that-stop-working-are-named` | `device-check` names a failed or missing device, or a disk's fault, once, and again when it's fixed |
| `kernel-trouble-is-named` | `device-check` names a kernel that oopsed, locked up, saw a hardware fault or held a task stuck, once, and not its harmless taints |
| `disks-say-when-they-fail` | a workstation runs smartd on its disks and hands their faults to device-check, not to every terminal |
| `smartd-stops-cleanly-without-disks` | the workstation's own smartd command, run with no disks, stops cleanly rather than failing |
| `disk-guard-accepts-its-configuration` | the disk guard passes its scenarios and accepts the configs a workstation and its account get |
| `disk-guard-passes-clippy`        | clippy finds nothing in the disk guard or its scenarios   |
| `disk-guard-compiles-for-macos`   | the disk guard and its scenarios compile for macOS, clean |
| `network-revive-passes-clippy`    | clippy finds nothing in network-revive or its tests       |
| `lid-docked-passes-clippy`        | clippy finds nothing in lid-docked or its tests           |
| `jobs-sorter-sorts-by-behaviour`  | the sorter moves a group busy two seconds running, never the shell's or pi's, and sends a scope started from jobs back there unless it sets limits |
| `jobs-sorter-passes-clippy`       | clippy finds nothing in the jobs sorter or its tests      |
| `shaper-finds-the-rate`           | the shaper reads a queue from delayed pings, not one late ping or a silent host; cuts under what the line carried, holds while it drains, and climbs while it's quiet; forgets a cut ten quiet minutes old, and keeps one the line then proves; reads sets, routes and queues from what the kernel prints |
| `shaper-passes-clippy`            | clippy finds nothing in the shaper or its tests           |
| `lag-watch-tells-who-waited`      | lag-watch counts the desktop or apps waiting as trouble and work waiting on work as none, names the app's unit that stalled most, tells of the desktop's or an app's program the kernel ended for memory, finds the threads it watches by name, and sums each generation by its load |
| `lag-watch-passes-clippy`         | clippy finds nothing in lag-watch or its tests            |
| `vm-first-finds-the-vms`          | vm-first reads a thread's nice and policy, knows qemu by its name and nothing else |
| `vm-first-passes-clippy`          | clippy finds nothing in vm-first or its tests             |
| `dictation-works-it-out`          | dictation's spectrogram, transform and vocabulary match their references |
| `dictation-hears-the-clip`        | Whisper on the CPU types a public clip back word for word |
| `dictation-passes-clippy`         | clippy finds nothing in dictation or its tests            |
| `dictation-switches-to-the-new-one` | a switch restarts dictation after reloading its units |
| `shutdown-status-reads-the-last-stage` | shutdown-status reads only the last stage's steps, says forced once when it begins unmarked, and leftovers after three quiet seconds |
| `shutdown-status-passes-clippy`   | clippy finds nothing in shutdown-status or its tests      |
| `open-with-passes-clippy`         | clippy finds nothing in the portal's chooser or its tests |
| `xdg-open-opens-each-kind-in-its-app` | xdg-open opens markdown in the editor declared for text, in the terminal named for it, and a picture in its viewer |
| `device-check-passes-clippy`      | clippy finds nothing in device-check, device-check-disk or their tests |
| `guide-proves-its-steps`          | the guide counts a step done only once observing it again says so, reads what real machines' tools printed, writes sticks that sfdisk, fsck.fat and their own key read back whole, signs EFI programs byte for byte as sbsign does, and finds every program it runs |
| `guide-passes-clippy`             | clippy finds nothing in the guide or its tests            |
| `guide-compiles-for-macos`        | clippy finds nothing in the guide as macOS compiles it     |
| `ir-camera-passes-clippy`         | clippy finds nothing in ir-camera or its tests            |
| `v4l2loopback-holders-passes-clippy` | clippy finds nothing in the cameras' holders watcher or its tests |
| `rust-programs-pass-clippy`       | clippy finds nothing in the single-file programs or their tests |
| `record-window-passes-clippy`     | clippy finds nothing in record-window or its tests        |
| `plymouth-waits-passes-clippy`    | clippy finds nothing in plymouth-waits or its tests       |
| `the-splash-keeps-systemd-quiet`  | with the splash, systemd writes only failures to the console |
| `spend-watch-accepts-its-configuration` | spend-watch passes its tests and accepts the config an account gets, a feed and a cap among it |
| `spend-watch-passes-clippy`       | clippy finds nothing in spend-watch or its tests          |
| `wifid-reads-the-air`             | wifid's tests pass: what it hears and offers, how each network is joined, the addresses it shows, what it keeps, and sign-ins |
| `wifid-sign-in-reads-its-network` | the sign-in window's tests pass: its proxy lets on only the window, and it follows which card wants a sign-in |
| `wifid-passes-clippy`             | clippy finds nothing in wifid, its sign-in window or their tests |
| `claude-status-line-says-how-it-stands` | Claude Code's status line shows the folder, the model and the context's size, cached tokens counted, in the theme's colours |

### Virtual Machines

Each boots a machine under QEMU, which takes minutes, so each covers what only
a running system shows.

While changing what one covers, run only the subtests the change touches.
A test's driver builds in seconds once its machines are built, and runs
whatever script it's given; `run-subtests`, in the dev shell, builds it and
runs everything outside a subtest (the boot, the helpers) and the subtests
whose names hold the words given, in under half a minute for one desktop
subtest, boot included:

```sh
run-subtests vm-desktop-works                       # list its subtests
run-subtests vm-desktop-works "opens kitty"         # run those named
run-subtests -I vm-desktop-works "opens kitty"      # then a Python prompt, machines up
run-subtests -K vm-desktop-works "opens kitty"      # keep the disks for the next run
```

A subtest that uses what an earlier one left, a window or a notice, needs
that one named too. Each run has its own short runtime folder, since the
driver keeps its network switch's socket and the machines' state under
`XDG_RUNTIME_DIR`, so two runs at once don't collide; the folder has to be
short, as a socket's path can't pass 108 bytes. The pictures land in the
folder it prints. A break is watched failing this way too, never by
copying the tree and running the whole test.

A VM test proves wiring: that the real parts connect, a unit starting with
its config, a message reaching its receiver, the shell drawing on niri. One
path through each connection is enough; the many cases of a behaviour, such
as a search's ranking or a guard's thresholds, go in the tiers below, where
they run in milliseconds.

Its script is Python in a file of its own, `tests/<name>-vm-test.py`, which
the test reads with `builtins.readFile` and `skipTypeCheck = true`, so ruff
and mypy read it as strictly as any other Python, as `nix fmt` runs them.
The driver gives the script its names as globals; for type checkers, it
imports them from `tests/driver_names.pyi` under `if TYPE_CHECKING:`, and
mypy checks every call against the driver's own types before a machine
boots. What the script needs from Nix comes as a program on the machine's
path, or as one JSON file of values it reads at its top, never spliced into
its text. Every subtest stays at the top of the file, where `run-subtests`
finds it.

A test waits on what shows the state it needs: a unit's readiness, a line
in the journal, a file, the compositor's event stream. A fixed sleep only
guesses how long a loaded machine takes, and fails or wastes time when it's
wrong. Showing that something didn't happen takes an event too: wait until
what would have made it happen is done, such as systemd's job queue
emptying, and then look.

| Check                             | Proves                                                  |
| --------------------------------- | ------------------------------------------------------- |
| `vm-secure-boot-installs`         | the whole install works, and the disk opens only for it |
| `vm-boot-asks-in-the-theme`       | the splash asks for the disk, names a slow job, and lasts a restart to its end; a sign-in console greets the account, draws the theme's face, padded, signs in to a shell told it's kmscon, greets again once it's left, and its keys go back to the desktop; one kmscon can't draw falls back to the login prompt, greeting included |
| `vm-boot-asks-on-every-screen`    | with two screens of different sizes, the splash's ask shows whole on both, its bottom left corner on the smaller one included |
| `vm-guide-installs`               | the guide, driven by its keys, installs a machine from its stick: two keys and a recovery key of seven words, shown alone for a phone and typed back, Secure Boot enforcing with its keys, then seals the disk to the TPM with a PIN, which opens it at the next boot; clears the TPM, opens with a YubiKey and seals again; a host writes a stick from the rescue bundle the install sealed to the recovery key, which the firmware boots while enforcing; then the install's own stick, which it made the machine's by carrying its keys and bundle and signing itself with them, boots enforcing and reinstalls the machine with the same keys |
| `vm-guide-writes-a-stick`         | the guide writes a stick as the person, through UDisks2, unmounting what was mounted from it, its passphrase seven words shown alone for a phone and typed back, another's words refused, the old stick's rescue bundles read from its vault mounted only for reading and the host's own kept for the person and carried onto the new stick, which is the host's own machine's, so the bundle it keeps for itself is opened with its passphrase and signs the stick's boot program with the machine's db key |
| `vm-guide-opens-the-vault`        | a stick's vault sealed to a FIDO2 key opens with its PIN, and without it by the passphrase |
| `vm-guide-recovers`               | a repair stick finds a machine's disk, opens it with a YubiKey's PIN, mounts it whole, sets which generation boots next, replaces a lost key, reinstalls a boot loader gone from the ESP, and gives a root shell inside |
| `vm-commits-are-signed`           | commits are signed with the TPM's keys, PINs asked for  |
| `vm-passkeys-come-from-the-tpm`   | passkeys and security keys are made in the TPM, each after a yes |
| `vm-keyring-opens-without-asking` | the keyring opens with its TPM-sealed password, unasked |
| `vm-speakers-are-tuned`           | the XPS 14's speakers hear the tuning, headphones don't |
| `vm-pebble-speakers-are-tuned`    | a Pebble X Plus hears its tuning, headphones don't      |
| `vm-mv7-voice-is-levelled`        | an MV7's voice comes up 12 dB, its room and s sounds less, its peaks under -1 dBFS; another mic's are untouched |
| `vm-webcam-microphones-yield`     | a webcam's microphone ranks below a USB microphone's    |
| `vm-desktop-works`                | it boots clean; the desktop shows, opens, locks and asks, a USB camera's stream included |
| `vm-greeter-signs-in`             | the greeter signs in and out, not waiting on the account's setup |
| `vm-wifi-joins`                   | wifid joins, moves past a silent access point, nudges a stuck look, finds sign-ins, keeps others' routing rules |
| `vm-disk-guard-keeps-the-disk`    | the guards answer each band, sparing what's in use       |
| `vm-network-revives-units`        | what the network let down starts, and reloads on change  |
| `vm-clock-keeps-time`             | the clock sets from the machine's servers past a network's silent one |
| `vm-libvirt-stops-quickly`        | libvirt's daemon stops at once, its udev thread starved  |
| `vm-ir-camera-opens-for-readers`  | the IR camera waits for udev and its loopback, opens only while read, its frames grey, no frame from before a reader opened |
| `vm-loopback-says-who-holds-it`   | a loopback camera's file names each program another account holds it with, not its relay's unit however a program names itself, one line a name, catches a holder that forks, empties when they close it or the camera stops, keeps its folder, follows a new loopback, and takes no loopback of another name |
| `vm-swap-comes-down-while-probed` | the compressed swap comes down, its reset waiting out udev's probe |
| `vm-full-swap-ends-what-filled-it` | full swap ends the cgroup that filled it through systemd-oomd, before the kernel's OOM killer, and leaves the rest |
| `vm-apps-come-first`              | with no disk named, the account's app and session slices, and the system's user.slice, hold a 1 ms latency on the drive behind /home, through the kernel (jobs, background, machine and system none); with one named, io.cost holds its model and qos, no slice holds a target, and stopping its `disk-order-<disk>` unit turns io.cost off; every slice's io.weight follows its CPU weight; app.slice holds a hard 13% of memory, session.slice 1.5 GiB and every slice above them 16%, and jobs is slowed past half; timed work sits at 10000 inside idle jobs, and work in jobs can move itself there; a scope work in jobs starts goes back to jobs, and one with a memory ceiling stays |
| `vm-the-network-keeps-its-order`  | an account's jobs, background, the system's builds and a guest it forwards for go in CAKE's bulk tin both ways, what Joel waits on at a terminal and an app's pages best effort, its calls and Tuple's daemon video, names and the shaper's pings voice; a slice made again keeps its class; the client's own network isn't slowed; inside a WireGuard tunnel the same classes reach the tunnel's own CAKE; the queues follow the default route and go when the shaper stops |
| `vm-the-line-stays-quick`         | under four uploads on an 8 Mbit/s line 20 ms away, an app's ping stays under 80 ms with the shaper and passes 300 without; an app's upload leaves the jobs' under a quarter of what it sends; the shaper follows the line down to 4 Mbit/s and back |
| `vm-checks-go-first`              | a build user's qemu in the daemon's cgroup goes from batch at nice 5 to normal at nice -10, every thread and a later one, and a compiler beside it doesn't |
| `vm-plugging-in-wakes-it`         | every USB root hub may wake the machine, so a cable plugged into a sleeping laptop wakes it |
| `vm-lid-stays-at-the-desk`        | a shut lid suspends with no monitor, keeps running with one plugged in whose screen is off, holding one lock through the card's changes, and suspends once it's unplugged |
| `vm-admins-switch-unasked`        | an account in wheel switches to another system and back with no password; one outside wheel, and what isn't a NixOS system in the store, is refused |
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
- It goes in the `checks.nix` of the folder whose promise it holds, which
  takes the helpers it uses by name. A folder's first check makes the file,
  which the root `checks.nix` then lists, alphabetically; a name two files
  give stops the evaluation.
- A check above the promises says what the tiers below it miss.
- What a running system shows goes in an existing virtual machine where it
  fits. A new one has to cover something none of them can; its name starts
  with `vm-`.
- A VM test past about two minutes, or twenty subtests, splits by area, each
  part carrying only the stand-ins its area needs, so a change reruns only
  the part it touches. Each subtest sets up what it needs and leaves nothing
  behind, so it can run alone.
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
[6]: ../AGENTS.md#commands
[7]: decisions/0012-no-ci.md
