# Testing

Checks are for the loop of making a change: each one has to tell whoever is
changing core.nix something they need to know, quickly, and something no
cheaper check already tells them. A check sits in the `checks.nix` of the
folder it proves something about, a module's or a hardware model's, and
[`checks.nix`][1] at the root gathers them, names the virtual machine tests,
and holds the few about the repository as a whole. Nothing runs on a
schedule.

## The Loop

The commands are in [AGENTS.md][6]'s Commands, kept there alone so the two
can't drift. While editing, a change is proven by the cheapest checks that show it,
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

What each check proves is written in [`check-descriptions.nix`][8], by its
name, and carried in its `meta.description`, so the list is one query away:

```sh
nix eval --json .#checks.x86_64-linux --apply 'builtins.mapAttrs (_: check: check.meta.description)'
```

### Promises

Read from evaluated machines, without building them. They catch an option
that changed meaning, a role that brings what it shouldn't, or a setting that
quietly stopped holding.

### The Shell's Logic

Run in Qt's own JavaScript engine, the one the shell runs it in, over cases
written beside each in `tests/`.

### Examples

Whole machines that use core only through its outputs, as a machine
repository does. Between them they import every role and the Dell's
hardware, so a module that sets an option that doesn't exist, or relies on
one nothing brings, fails here; the Surface's hardware is evaluated by its
own checks.

### Programs

The real program reads the files an account gets, because a program that
can't use its configuration usually falls back to its own without a word.
The kernel checks build only its configuration: the patched source, run
through Kconfig, which drops a setting whose dependencies aren't met. [The
kernel guide][2] says what that catches.

### The Shell on niri

The account's shell files and its real niri configuration, run on a real
niri nested in a headless sway drawn in software, inside the build sandbox
(`tests/nested-niri.sh`), in seconds rather than a machine's minutes. It
proves what the shell does on niri that needs no system service: a surface
landing as niri's layer, what a bind's command does. It asserts through the
shell's reports and `niri msg`, never pixels. `the-notices-land-on-niri` is
the first. Since nothing types there, Joel is away as soon as the shell's
idle time passes, which `the-notices-wait-while-joel-is-away` uses.

Two things it can't do, so they stay in a machine. Nested, niri has one
output that never goes, so screens coming and going need a VM. And keys
typed there with wtype never reached niri's binds, so a check runs what a
bind runs rather than pressing it. Nor does anything there end Joel being
away: sway's cursor moved and a virtual pointer through `wlrctl` both left
niri idle, so coming back needs a VM.

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
run-subtests vm-desktop-launcher-goes-places                   # list its subtests
run-subtests vm-desktop-launcher-goes-places "opens kitty"     # run those named
run-subtests -I vm-desktop-launcher-goes-places "opens kitty"  # then a Python prompt
run-subtests -K vm-desktop-launcher-goes-places "opens kitty"  # keep the disks for next time
```

Each subtest makes what it needs, a window or a notice, when an earlier one
hasn't, so it runs alone. Each run has its own short runtime folder, since the
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

Its tests are Python in a file of their own, `tests/<name>-vm-test.py`: test
functions that pytest runs inside the driver, through
`tests/lib/vm_pytest.py`. The test's Nix calls `tests/lib/pytest-vm-test.nix`
in place of `runNixOSTest`, naming its machines and `tests = ./<name>-vm-test.py`,
and the helper gives it the rest: the shared Python, the line that hands the
file to pytest, and no driver type check, since ruff and mypy read the file
as strictly as any other Python when `nix fmt` runs them, against the
driver's own types. `tests/lid.nix` and `tests/lid-vm-test.py` are a small
pair to copy.
The driver's machines are fixtures (`machine`, `machines`, `driver`), and
each state a test starts from, booted or with the network let down, is a
fixture of its own, made once a run however many tests ask for it. What
the tests need from Nix is one JSON file, the helper's `values`, or a
function of the test's nodes giving it, which a test reads parsed through
the `values` fixture, as `tests/signing.nix` does. Each
test is one of the driver's subtests in the log, named by its docstring's
first line, so a test runs alone, after only what it asks for. A program
the test runs on the machine is a file beside it too, never a script in a
Nix string, so a linter reads it.
`run-subtests` knows such a test by its `vm_pytest.run` and runs its script
whole, telling pytest to pick the tests whose function names hold the words
(a word's spaces read as underscores); with no words, pytest lists the tests
and nothing boots. Any `PYTEST_ADDOPTS` of your own still holds:

```sh
run-subtests vm-network-revives-units           # list its tests
run-subtests vm-network-revives-units reload    # run those named, and what they need
PYTEST_ADDOPTS=-x run-subtests vm-commits-are-signed terminal
```

One older test, usb-wake, is still a script the driver runs top to bottom:
one check and no subtests, so `run-subtests` has nothing in it to pick. It
moves to pytest in its turn, and nothing new is written that way.

A selection that picks no test fails the run, so a mistyped name can't pass
for a proof. A failed assert says what each side held, and the run ends by
listing the tests and fixtures that took a second or more. Durations given
to the driver are `datetime.timedelta`s, since NixOS 26.11 warns of bare
numbers, and pytest shows those warnings at the end of the run.

A test waits on what shows the state it needs: a unit's readiness, a line
in the journal, a file, the compositor's event stream. A fixed sleep only
guesses how long a loaded machine takes, and fails or wastes time when it's
wrong. Showing that something didn't happen takes an event too: wait until
what would have made it happen is done, such as systemd's job queue
emptying, and then look.

The install test builds a disk image with disko and boots it with a software
TPM, then follows [the install guide][3] step by step: the keys are made and
enrolled, the disk is bound to the TPM and a PIN, the passphrase is wiped,
and the TPM refuses once a measured PCR changes. On the way it checks that
enrolling without checksums refuses the option ROM of QEMU's network card,
which the firmware runs, and that no NvPCR is set up at boot or measured at
login. It lives in [`tests/secure-boot.nix`][4].

### What Changed on Screen

Each area of the desktop test has a gallery, `vm-desktop-<area>.gallery`,
the same test photographing every state it puts the desktop in, each once
the screen holds still, and keeping the pictures in its result. The area
itself takes none, since proving a change needs none and each photograph
waits for the screen to hold still; a subtest that measures the screen uses
`settled_screen`, which the gallery keeps as a picture too. No check
compares them with stored ones: a stored picture would fail on every change
meant, and two runs of the same code don't draw every picture the same, as a
clock or a notice's age moves on. Instead, a change to how anything looks is
compared with main, and every picture it changed is looked at:

```sh
area=checks.x86_64-linux.vm-desktop-launcher-answers.gallery  # each area the change touches
main=$(nix build --no-link --print-out-paths "github:Jitsusama/core.nix#$area")
here=$(nix build --no-link --print-out-paths ".#$area")
nix develop -c shots-changed "$main" "$here" changed
```

`shots-changed` names each picture that changed, most first, by how many
pixels, and draws it in `changed/` before, after and their difference, cut
to where it changed. A picture that should have changed and isn't named
matters as much as one named that shouldn't have. The pictures that move by
themselves are named in [`tools/shots-changed/shots-that-move.txt`][5],
each with what moves in it, and listed apart. One joins that list only once two runs of
unchanged code show it moving, for a cause the test can't hold still; a
picture caught before its state was drawn is a fault in the test instead.

## Earning a Check

- A new check goes in the lowest tier that can show what it covers. Most
  belong with the promises.
- It goes in the `checks.nix` of the folder whose promise it holds, which
  takes the helpers it uses by name. A folder's first check makes the file,
  which the root `checks.nix` then lists, alphabetically; a name two files
  give stops the evaluation.
- What it proves goes in `check-descriptions.nix`, under its name, in its
  place alphabetically; `every-check-says-what-it-proves` fails without it.
- A check above the promises says what the tiers below it miss.
- What a running system shows goes in an existing virtual machine where it
  fits. A new one has to cover something none of them can; its name starts
  with `vm-`, and it goes in the root `checks.nix`'s `vmTestsOn`, whose
  `pkgs` start virtiofsd through without-syncfs so a guest's sync never
  commits the host's whole filesystem. One defined anywhere else misses
  that silently.
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
[5]: ../tools/shots-changed/shots-that-move.txt
[6]: ../AGENTS.md#commands
[7]: decisions/0012-no-ci.md
[8]: ../check-descriptions.nix
