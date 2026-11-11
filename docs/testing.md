# Testing

Checks are for the loop of making a change: each one has to tell whoever is
changing core.nix something they need to know, quickly, and something no
cheaper check already tells them. A check sits in the `checks.nix` of the
folder it proves something about, a module's, a hardware model's or a
tool's, and those about the repository as a whole sit in `tests/repository/`
and `tests/rules/`. [`checks.nix`][1] at the root gathers them and names the
virtual machine tests not yet in an area. Nothing runs on a
schedule but the net under main, which runs each merge's VM tests after it
lands, as The Net Under Main below says.

## The Loop

The commands are in [AGENTS.md][6]'s Commands, kept there alone so the two
can't drift. While editing, a change is proven by the cheapest checks that show it,
which [AGENTS.md][6] lists by kind of change, and a VM test only by the
subtests it touches. `changed-checks`, in the dev shell, finds them all: it
evaluates every check at the change's merge base with origin's main and in
the working tree, and names each whose derivation was added, changed or
removed, or no longer evaluates. A check whose derivation is the same
proves what it proved before, so only the others need building, and it
builds them, all but the VM tests, which `run-subtests` takes a piece of
and `merge-net needs` names where one runs whole before a merge.
`--build` adds every VM test the change moved, which no gate asks for, so
it says how many before it sends them: on 6 November one run for a change
to the shell alone held every VM builder for 66 minutes. A named check
proves nothing until it's built:
core's main broke on 30 October on a change whose run named checks and
built only some, and `--name-only` is for naming alone. With no
`--system` it compares the system with the most checks, since core holds
its checks about checks on Linux alone, and that change's run, on a Mac,
compared only the Mac's. `nix flake check` runs only for a
change every check reads, such as `flake.lock`: on a machine shared with
other work, a whole run is what makes the loop slow, and the load it adds
is what makes VM tests time out.

`nix fmt` lints as it formats, at each linter's strictest, test code
included: ruff with every rule and mypy `--strict` for Python (`ruff.toml`,
`mypy.ini`), shellcheck for shell with the optional checks no script
breaks, luacheck for Lua, each file read as the
Lua of the program that runs it (`.luacheckrc`), biome with every rule that
fits for the JavaScript QML imports (`biome.jsonc`, read through
`modules/treefmt/qt-javascript-lint.sh`) and, formatting it too, for
Node's `.mjs` tests and pi's TypeScript, and for JSON, taplo for TOML, rumdl for
Markdown (`.rumdl.toml`), zsh
itself for what it can't parse, since nothing lints zsh further, statix and
deadnix for Nix, and the `formatting` check holds the repository to it.

Two linters run in checks rather than `nix fmt`, since each needs what a
build brings. clippy reads every crate and single-file program with its
pedantic group on top of its defaults, less three lints about a published
library's API, as `lints` in `modules/clippy/clippy.nix` says; `clippy.toml` holds
its settings, and the `-passes-clippy` checks hold the code to both, which
a bare `cargo clippy` doesn't. qmllint reads the account's desktop and the
greeter (`quickshell-passes-qmllint`), and each offscreen check's own
folder, its cases and stand-ins included, before running it
(`tests/lib/shell-folder-lint.sh`). A finding that's right as it is says so
where it is, with its reason: `#[expect(..., reason = ...)]` in Rust, a
`biome-ignore` comment in JavaScript, `qmllint disable` in QML. clippy and
Biome refuse theirs without one, and `every-suppression-says-why` holds the
rest: a `# noqa`, `# type: ignore`, `# shellcheck disable=` or `qmllint
disable` says why after the codes it names or in a comment just above.
Those before the rule wait, counted by file, in
`tests/rules/every-suppression-says-why-waiting.nix`, where a count only
falls.
shellcheck's four optional checks that find faults but that many scripts
still break, for a hidden failure, a `set -e` turned off, an uppercase
variable nothing sets and a `case` with no default, run in
`every-script-meets-shellcheck-at-its-strictest`, whose findings wait by
script in the same way. Two more stay off, as `modules/treefmt/treefmt.nix`
says why: braces on every variable and `[[ ]]` for every test.

`nix flake check` runs every check, but Nix reuses any check whose inputs
didn't change, so a change pays only for the builds it can affect. It still
evaluates every check, and that is what a whole run costs: on 2026-10-25 the
305 checks took 1,006 s to evaluate uncached and 771 s from Nix's
evaluation cache, on a 16-core laptop busy with other builds, and one run
held 24.5 GB. `changed-checks` keeps the base's derivations by commit, so
once a base is kept a comparison pays for one evaluation, the working
tree's: 461 s on 2026-10-26, against 1,373 s and more for one that had to
evaluate a base that had just merged. `changed-checks --keep-base` keeps a
commit ahead of time, so a branch rebased onto it pays for its own tree
alone; it goes only to a builder already standing, never asking for one,
and keeps it on this machine only with `--here`. And
`changed-checks keep-main`, a service beside the net, does it for each
head of main once it has stood two minutes, and again on each
builder lent afresh, which starts with nothing kept: on 2026-11-05, before
it ran, 42 of the lent builder's 105 runs evaluated their base first, a
median of 444 s, and a replay of that day's heads had it sparing 29 of
them. It evaluates a chunk
of checks for every 4 cores and 6 GiB free, from 2 to 8 at once, since one
kept under 2 of 16 cores busy: on 2026-10-26 a working tree took 324 s one
chunk at a time, peaking at 5.8 GB, and one a few commits later took
157.5 s two at once, peaking at 10.8 GB. On 2026-11-05 the lent builder,
32 cores with 125 GB nearly all free, took a median of 8 minutes a side
two at a time, four fifths of each run. Each run
adds a line to `~/.local/state/changed-checks/runs.jsonl` saying what each
side's evaluation took, chunk by chunk, what it named, and what its build
took, so these times are read from the runs rather than measured once.

Wherever nix-offload can send it, the whole run leaves the machine, since
each evaluation holds gigabytes and several cores beside the person's
work: on 2026-10-30 every session changing core evaluated its own, a
steady part of a load of 92 to 96. `nix-offload there --changed` sends
the tree to the lent builder, which runs the commit's own changed-checks
and builds what it names, or to the second lent builder when the first
doesn't answer, waiting for one to stand again while a lender runs, or
else, with none running, to the first machine in
`jitsusama.nix-offload.evaluators` that answers and takes the
repository's GitHub owner, which only evaluates, and what it names is
built as any `there` build is. The far side keeps
each base's derivations as a run here would. A `--base` goes with the
run as its merge base's commit, which the far side has from origin's
main. `--here` runs it on this machine, as does a base whose merge base
origin's main doesn't hold, a change to a tracked file not yet
committed, since the far side checks a commit, or nowhere taking it
while no lender runs. While one runs and nothing takes the run, it ends
saying so, rather than evaluating here. With none running and nowhere
taking it, a run without `--here` runs here only while CPU pressure over
a minute is under 15%, and otherwise fails saying to run it again; the
net tries its round again two minutes later. On 2026-11-07, with no
builder answering, the net's evaluations and a session's ran here at up
to 6.3 GB each, and held CPU pressure near 70% with Joel at the machine.
The net's runs pass `--first`, which `nix-offload there` marks so a builder
lets them into a free turn or one of two it keeps for them, ahead of
sessions' runs waiting: on 2026-11-08 the net's comparison waited an hour
behind sessions' gates and tested nothing. Only the net's own unit,
`merge-net.service`, may ask; anyone else's `--first` is said and dropped.

A check that passed at a commit isn't built again there, wherever it would
go: each builder nix-offload lends starts with an empty store, so a check
built again on the next was paid in full (716 s, then 790 s, for one on
2026-11-03). nix-offload keeps each check `there` built that passed, and
each `there --changed` run that passed with what it named, by commit, in
`nix-offload/passed/<commit>` under XDG's state folder. The same run again
at that commit says what it named and builds nothing, and one asking for
more tells the far side's changed-checks which checks passed, as
`CHANGED_CHECKS_PASSED`. `run-subtests --there` with the same words at that
commit runs nothing and copies the pictures from where the last run put
them, while they're still there. A failure, any other build whose output is
wanted back (`--apply`, `--out`, `--print-out-paths`) and anything but a
check are never kept. Removing the commit's file has it all built again.

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

## The Net Under Main

A change merges once the checks `changed-checks` builds pass, VM tests
aside, and the subtests that reach what it touched, in each VM test that
owns one of its files, pass with `run-subtests --there`: about ten minutes.
`merge-net needs` names those VM tests, of the ones that moved from main: a
test owns the files `checks.nix` imports for it and the paths they name
beside them, and the folder of each module or role it imports by name, or
only the roles' when it imports one, since it then tests the machine the
role makes. On 10 November a change to the slices module's disk targets
named no VM test, and the slices test failed on main alone.
A change `merge-net needs` says touches something a revert would come too
late for runs the VM tests it names, whole, first: boot, the initrd, the
kernel, the disk and LUKS, the TPM, secure boot, the install, the stick and
its guide, the greeter or lock, `flake.lock` or nix-offload
(`tools/merge-net/must-run.txt`). Of those, it names only the ones whose
derivation moved from main, since the rest would give main's result. A
change to nix-offload, which moves none, sends one VM test's subtests
through itself with `run-subtests --there` instead. `merge-net ready COMMIT`,
asked before a head merges, refuses one that doesn't sit on origin's main
as it is now, or at which no `changed-checks` run that built what it named
passed, from nix-offload's record of what passed at each commit. A rebase
needs no run of its own when that run passed at an earlier commit of the
same change, by git's patch id of a diff without context lines, so an
edit of main's beside the change leaves it the same, and main has since
moved none of its files. A file either side moved that names a path the
other renamed or deleted counts as one of them, since a list naming a
script the branch renamed breaks main as surely as a shared edit. A JSON
file both changed at different keys counts as two files, as though split
by key: an array of objects each with its own `id` is read by it, a change
of its order or of any other array's length changes it whole, and the
rebased head may differ from each side only where the other changed it.
The launcher's `words.json` is the one this is for, which a quarter of
main's merges change. When main moved some, `changed-checks` runs again at
the rebased head, never a single check chosen by hand, since which checks
read a file shows only in their derivations; Nix builds only what the
rebase moved and takes the rest from the builders. The one exception is a
list every change adds to, which main moves faster than a run takes: when
each shared file is one, the run at the head is that list's own check
(`every-check-says-what-it-proves` for `check-descriptions.nix`, the check
a `tests/rules/<check>-waiting.nix` is named for) with
`nix-offload there . checks.x86_64-linux.<check>`, and the quarantine list
needs only to read as a list of tests, each once. Nor is there ever a third
run: once the same change passed whole at two commits, a clean rebase onto
any newer main merges with none, whatever main moved, and the net tests it
after, unless a file either side moved names a path the other renamed or
deleted since the newest of the two. Main moves faster than a run, and one
branch ran six times in a day, every run passing. Two changes sharing a
file merge in the order their gates started, so the one nearly through
isn't the one made to run again: `nix-offload there --changed` says each
gate going in `nix-offload/running/<pid>.json` and keeps its start with its
pass, and `ready` refuses a head while a gate that started before its own,
in another tree, still runs and changes one of its files other than the
lists. A gate whose process is gone, or that started three hours ago, holds
nothing back. A change run again after a merge left its pass stale keeps
its tree's place, the start of the first gate `ready` saw there in the last
six hours (`places.json` in the net's state), so it goes behind only the
gates that started before that one. A machine switches only
to merged main, and a bad switch is undone by switching back to the
generation before it.

The rest run after the merge, on the net: `merge-net watch`, which
`modules/merge-net` runs as a daemon, follows main and runs each check
a head changed since the last head it passed, on the builders, twelve at
once: the checks other than VM tests first, since they take seconds, then
the VM tests. A branch built before its rebase, or that built only some of
what it named, merges a red check main never ran; on 2026-11-08 four were
red on main by 11:00, and none was caught. A check that fails twice at one
commit is broken: the net finds which merge broke it the moment it does,
while the round's other tests run on, searching only the merges that changed the test's derivation,
since a derivation fails wherever it does, and running a searched merge's
two runs at once, and only the subtests that failed in both runs at the
head, as `run-subtests --there` builds them, where pytest named any, checks
that the merge changed the test and that the
test passed just before it, writes `~/.local/state/net/hold.json` so no machine
switches to it, unless the machine already runs it, when holding would keep
only fixes off it and the hold is kept apart and says so, reverts it through a
PR merged on its own checks, clears
the hold as that PR merges, or as a test ends once it's on main, and writes a
finding for its owner in `~/.local/state/net/findings`. A check
that fails before the revert too doesn't stop it, or one break on main
would keep another in place. A revert whose checks no builder took is
tried again as each round begins, until one does. A revert that won't
apply, as when later merges built on the broken one, puts the test on the
quarantine list below with the broken merge's PR as its owner and clears
the hold, so the owner fixes it forward while switches go on. A test that
failed before that merge too, or
that didn't exist before it, as when a split moves a failing case under a
new name, is a finding for its owner, and nothing is reverted or held. So is one
that fails after a merge that didn't change it, since something outside the
code broke it. So is one that passes at main's newest head, since a fix
forward has landed: the net asks beside the search's first step, stops the
search if it was, asks again before it reverts, and once more when main
moved while the revert's checks ran, dropping the revert when the test
passes there, running nothing at a head that builds the test unchanged.
One that
fails and then passes at a commit is flaky, which is a defect in the test:
it goes on `tools/merge-net/quarantine.json` with its owner, and the net
reverts nothing for it until the owner makes it steady and takes it off.
A failure that fails one of its two runs at main's newest head isn't fixed
forward but flaky there, so it is listed at that head the same way.
Before a merge, `changed-checks`, `run-subtests` and `merge-net needs` read
that list from origin's main, so a branch can't quarantine its own failure:
a quarantined test still runs, but its failure is said and blocks nothing.
The net reads the list at its round's head and at main's newest head, so a
test listed while a round runs is searched for and reverted no more.
A run the net stops, past two hours on a builder or silent there for
thirty minutes, is a stall, not a failure (the wait for a builder counts
for neither): one stalling twice at a commit is a finding, and no
merge is blamed for it. A search its stalls leave unsettled holds no switch:
its test is kept in `~/.local/state/net/unsettled.json` and runs first at the
next head, as a held test does, since only a test's own output earns a hold.
A failure already found fixed forward at a head isn't searched again when a
restarted round hears it. A round ends with its tests: its searches, and the
PRs that list its flaky tests, settle beside the rounds after it, each kept in
`~/.local/state/net/settling` until done, so a restarted net takes them up
again. A switch leaves the net running, since stopping it mid-round loses
every run in flight; it starts its new build between rounds, once what it
was settling is done.
A round keeps one head for its VM tests, an hour or two, so beside it a quick
look every five minutes runs the other checks main's newest head moved since
the last head it looked at, and searches for and reverts a merge that breaks
one as a round does; it keeps that head in `~/.local/state/net/quick.json`.
Each round removes the runs' logs in `~/.local/state/net/logs` over three days
old, leaving `net.log`, so the folder no longer grows by gigabytes a day.
The net starts nothing while a hold on the whole machine stands, or while
`~/.local/state/net/pause` is there, and `merge-net status` says where it
is.

## The Pyramid

From the bottom up, each tier costs more than the one below and has to show
something it can't.

What each check proves is written in [`check-descriptions.nix`][8], by its
name, and carried in its `meta.description`, so the list is one query away:

```sh
nix eval --json .#checks.x86_64-linux --apply 'builtins.mapAttrs (_: check: check.meta.description)'
```

At every tier, a failing assertion says what was wrong. Where the language
shows what each side held, as Rust's `assert_eq!` and a Python comparison
under pytest do, nothing is added; where it can't, as with Rust's `assert!`
and a Python `assert` of anything but a comparison, the assertion carries a
message saying what was expected. `every-assert-says-what-was-wrong` holds
it, and those before the rule wait, counted by file, in
`tests/rules/every-assert-says-what-was-wrong-waiting.nix`, where a count
only falls.

### Promises

Read from evaluated machines, without building them. They catch an option
that changed meaning, a role that brings what it shouldn't, or a setting that
quietly stopped holding.

### The Desktop's Logic

Run in Qt's own JavaScript engine, the one the desktop runs it in, over cases
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

### The Desktop on niri

The account's desktop files and its real niri configuration, run on a real
niri nested in a headless sway drawn in software, inside the build sandbox
(`tests/nested-niri.sh`), in seconds rather than a machine's minutes. It
proves what the desktop does on niri that needs no system service: a surface
landing as niri's layer, what a bind's command does. It asserts through the
desktop's reports and `niri msg`, never pixels. `the-notices-land-on-niri` is
the first. Since nothing types there, Joel is away as soon as the desktop's
idle time passes, which `the-notices-wait-while-joel-is-away` uses.

The launcher is typed into there: wtype's keys reach the surface holding the
keyboard, the launcher's, so a case that only types and reads the launcher
runs on the account's whole shell, through pytest, with
`tests/launcher_on_niri.py` keeping the desktop test's helpers' names.
`the-launcher-answers-whats-typed-on-niri` is the first, in seconds where the
same cases took minutes in `vm-desktop-launcher-finds-by-name`.

Two things it can't do, so they stay in a machine. Nested, niri has one
output that never goes, so screens coming and going need a VM. And keys
typed there with wtype never reached niri's binds, so a check runs what a
bind runs rather than pressing it. A case needing the machine's battery,
lights, services or other programs needs a VM too. Nor does anything there end Joel being
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
run-subtests vm-desktop-launcher-moves-windows                   # list its subtests
run-subtests vm-desktop-launcher-moves-windows "opens kitty"     # run those named
run-subtests -I vm-desktop-launcher-moves-windows "opens kitty"  # then a Python prompt
run-subtests -K vm-desktop-launcher-moves-windows "opens kitty"  # keep the disks for next time
run-subtests --there vm-desktop-launcher-moves-windows "opens kitty"  # all of it on the builder
```

With `--there`, nix-offload builds the flake's
`legacyPackages.<system>.subtests` for the check and words where the VM
can run, so neither the machines nor the VM are made here, and copies the
pictures back. That build runs `run-subtests --driver` on the check's own
driver. With no builder to take it, nix-offload builds it here; one that
can't take it at all leaves run-subtests to run the driver here.

Each subtest makes what it needs, a window or a notice, when an earlier one
hasn't, so it runs alone. Each run has its own short runtime folder, since the
driver keeps its network switch's socket and the machines' state under
`XDG_RUNTIME_DIR`, so two runs at once don't collide; the folder has to be
short, as a socket's path can't pass 108 bytes. The pictures land in the
folder it prints. A break is watched failing this way too, never by
copying the tree and running the whole test.

A VM test proves wiring: that the real parts connect, a unit starting with
its config, a message reaching its receiver, the desktop drawing on niri. One
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
driver's own types. `tests/vm/devices/lid.nix` and
`tests/vm/devices/lid-vm-test.py` are a small pair to copy.
The driver's machines are fixtures (`machine`, `machines`, `driver`), and
each state a test starts from, booted or with the network let down, is a
fixture of its own, made once a run however many tests ask for it. What
the tests need from Nix is one JSON file, the helper's `values`, or a
function of the test's nodes giving it, which a test reads parsed through
the `values` fixture, as `tests/vm/keys/signing.nix` does. Each
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
emptying, and then look. `every-fixed-wait-says-why` holds it: a sleep in
a test polls, inside a loop that looks again, or says what passes while it
waits, in a comment on its line or just above, or by sleeping for a named
constant such as `UNHEARD`. Those before the rule wait, counted by file, in
`tests/rules/every-fixed-wait-says-why-waiting.nix`, and a count only falls.

A test that can't run, for want of a filesystem, a device or a tool, fails
rather than returning early, since a skip nobody sees passes a gate that
proved nothing; it runs where it can, such as in a VM. An `#[ignore]` says
what runs it instead, as `#[ignore = "needs OpenSSH, which ... gives"]`
does. `every-test-runs-or-fails` holds it, and the skips before the rule
wait, counted by file, in
`tests/rules/every-test-runs-or-fails-waiting.nix`.

A function, fake or fixture two tests need lives once, in the test
library both import (`tests/lib/` for VM tests' Python), since a copy
drifts from the first the next time one is fixed.
`every-test-function-lives-once` finds a function's body of two lines or
more in a second test file, whatever its name there, reading a Rust file's
`#[cfg(test)]` module as test code too, and the copies before the rule
wait, counted by file, in
`tests/rules/every-test-function-lives-once-waiting.nix`.

The install test builds a disk image with disko and boots it with a software
TPM, then follows [the install guide][3] step by step: the keys are made and
enrolled, the disk is bound to the TPM and a PIN, the passphrase is wiped,
and the TPM refuses once a measured PCR changes. On the way it checks that
enrolling without checksums refuses the option ROM of QEMU's network card,
which the firmware runs, and that no NvPCR is set up at boot or measured at
login. It lives in [`tests/vm/guide/secure-boot.nix`][4].

### What Changed on Screen

Each area of the desktop test has a gallery, `vm-desktop-<area>.gallery`,
the same test shooting every state it puts the desktop in, each once
the screen holds still, and keeping the pictures in its result. The area
itself takes none, since proving a change needs none and each shot
waits for the screen to hold still; a subtest that measures the screen uses
`settled_screen`, which the gallery keeps as a picture too. Still is the
picture staying the same for two spans, each stretched with how busy the
host is, since a busy host draws a fade slower than any fixed wait
(`vm_helpers.settled`). A shot that never stands still is taken, then fails
saying where the picture last changed and whether the shell, by its own
count of frames, drew meanwhile. No check
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
  with `vm-`, and it goes in its area's `checks.nix` under `tests/vm/`, or
  the root `checks.nix`'s `vmTestsOn` for a test in no area yet. Both take
  `pkgs` from `vmHostPkgs`, which starts virtiofsd through without-syncfs
  so a guest's sync never commits the host's whole filesystem. One defined
  anywhere else misses that silently.
- A VM test past about two minutes, or twenty subtests, splits by area, each
  part carrying only the stand-ins its area needs, so a change reruns only
  the part it touches. Each subtest sets up what it needs and leaves nothing
  behind, so it can run alone. `every-vm-test-keeps-to-the-pyramid` fails on
  one past twenty subtests; those past it already are listed, with their
  counts, which may only fall, in `tests/repository/checks.nix`.
- One machine carried through a story whose every step needs the ones
  before it stays whole, since each part of a split would replay the story
  to reach its first step and finish no sooner. The guide's install is one:
  its reinstall needs the first install's keys, sealed TPM and account, so
  `vm-guide-installs` keeps its subtests as the story's checkpoints, listed
  with the rest, its count held as theirs is.
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
[4]: ../tests/vm/guide/secure-boot.nix
[5]: ../tools/shots-changed/shots-that-move.txt
[6]: ../AGENTS.md#commands
[7]: decisions/0012-no-ci.md
[8]: ../check-descriptions.nix
