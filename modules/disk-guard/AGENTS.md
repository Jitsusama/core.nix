# The Disk Guard

A Rust daemon that keeps a machine's disks from filling, taking only what
regenerates, cheapest first, and never what a build or a person is using. It
runs twice: `disk-guard system`, a root service that owns what lies outside
every home, and `disk-guard user` in each account, which owns the home.
[`nixos.nix`][1] and [`darwin.nix`][3] render the machine's config and run
it under systemd or launchd, and [`home.nix`][2] the account's.

## How It Decides

- **Bands.** Each filesystem a source or the store lies on is healthy, under
  pressure, urgent or critical by its free space (25%, 10% and 4% by
  default), and a pass reclaims towards the target (35%). A forecast starts a pass
  before the next band arrives. Btrfs pools are one filesystem however many
  subvolumes they hold.
- **Order.** Below healthy a pass takes, in turn: waste (orphaned targets,
  oversized logs, Nix's leftover build directories), Nix generations and
  garbage, idle build variants and cold scratch, producers' own collection,
  caches cheapest to refill first, idle targets and installs, older builds a
  newer one of their crate replaced, and last what is occupied. While
  healthy, routine upkeep takes what's due every six hours: on mains or a
  battery over half, ten minutes after boot or resume, however busy the
  machine, since a workstation that's always busy would otherwise never
  be tidied.
- **Windows.** How long a thing has to sit idle before it goes narrows with
  each band, and counts only time the machine was awake. Even while healthy,
  waste goes, and so does what nobody has used for long: a build after a
  week, an install after two, a replaced build after three days, a core
  after a week. `src/plan.rs` holds the table.
- **Use.** Before and after it moves a target aside, the guard judges whether
  any process uses it: a build holding cargo's or rustc's locks, a process
  with a file, map or working directory beneath it, or build variables
  naming it. Busy stays; a target only sat in waits for critical; a process
  it can't read keeps everything it might use, unless the system daemon
  vouches for it or it has exited since the scan, as most of a build's do
  within a second.
- **Measuring.** A pass reclaims a batch, lets it settle and measures it. A
  group whose batches free under a tenth of what they predict, three times
  running, is left for the next group; a batch predicting under a gigabyte
  measures whatever is writing, so it neither counts nor teaches. A pass
  that falls short rests before the next, doubling from a minute to fifteen
  under pressure, five at urgent and one at critical.
- **Pacing.** A busy machine never stops a pass; deleting slowly is how it
  spares the machine. Btrfs frees what's unlinked only when it commits, and
  saves wait behind a commit carrying a big delete, which no I/O priority
  reaches. So upkeep and a pass under pressure rest after every 2,000
  unlinks for 30 seconds, past Btrfs's own commit, and longer while the
  guard's probe, a page beside its state synced once a second, comes back
  slow more than three times in two minutes, until two minutes pass
  without. Before measuring, they wait for Btrfs's own commit rather than
  ask for one, since a commit asked for holds up every save on the disk. A
  disk falling to urgent ends the resting. Urgent and critical passes never
  rest, and ask for their commits. Either keeps the status file's
  heartbeat, resting or not: a step that blocks, such as a commit waited
  for, a command, a Nix deletion or the system daemon's answer, runs on a
  thread of its own while the pass beats, since the desktop calls a guard
  silent for two minutes dead. `src/pace.rs` holds it; `pacing` in the
  config sets the batch and the rest.
- **Removal.** Everything is reached through directories opened beneath the
  source's root, never through a symlink, another filesystem or `..`. A
  target is renamed into a trash beside it, judged again, then emptied; a
  disk too full to rename deletes in place. Each step is in the journal, and
  a guard starting after a crash empties what was begun.
- **Replaced builds.** The account's guard marks every target's `deps` with
  fanotify and keeps when each artifact was last opened in `uses.json`, since
  under noatime nothing on disk says. From pressure, a build of a crate with a
  newer one beside it goes once nothing has opened it for a day and an hour,
  with cargo's locks held; a `deps` never watched dates nothing.
- **Freezing.** At critical with nothing left to take, the account's guard
  freezes the process that wrote more than half of the last minute's writes,
  at a mebibyte a second or more, with a file open on that disk: it and its
  descendants move into a scope of their own, which is frozen. Never the guard,
  nothing in a session, nothing of another account's. It thaws once the disk
  is above urgent, or by `disk-guard thaw`, after which it's never frozen
  again.
- **Nix.** The system daemon lists the dead paths every six hours in a spare
  moment, and deletes them oldest
  first, keeping three days' within a budget of 10% of
  the filesystem. It expires generations, never the current, running or
  booted one, the newest five, or one with a boot entry. It links the
  store's identical files weekly in a scope of its own at idle priority,
  only while nothing stalls on the disk or memory, since it can wait
  without cost, stopping when the moment ends. An account's guard on the same filesystem
  asks it to collect rather than collecting itself; with no system daemon
  answering, it collects with `nix store gc --max` from urgent.
- **A Mac.** The same daemons run under launchd, asking macOS through
  `crate::sys`, which is `linux.rs` or `macos.rs`. Nothing hears who opens a
  file there without Apple's entitlement, so no older build is dated or
  offered; nothing freezes a writer yet; there's no scope for a linking run;
  and Time Machine's local snapshots are thinned from urgent on.

## Where Things Live

| Path                      | Holds                                                    |
| ------------------------- | -------------------------------------------------------- |
| `src/config.rs`           | the config each daemon reads, and what it refuses        |
| `src/band.rs`, `decide.rs` | bands, the forecast and the moment                      |
| `src/plan.rs`, `pass.rs`  | the order, the windows, and a pass that measures as it goes |
| `src/pace.rs`             | resting between batches, by the disk's own feel          |
| `src/survey.rs`           | each source's candidates                                 |
| `src/anchor.rs`           | removal that never leaves its root                       |
| `src/activity.rs`, `locks.rs` | whether something is in use                          |
| `src/host.rs`, `linux.rs`, `macos.rs` | the machine the guard runs on, through `crate::sys` |
| `src/procargs.rs`, `pmset.rs` | what macOS says, parsed where Linux's tests reach it |
| `src/daemon.rs`, `serve.rs` | a daemon's step and its loop                           |
| `src/socket.rs`, `vouch.rs` | what the two daemons say to each other                 |
| `src/garbage.rs`, `generations.rs`, `dedupe.rs` | Nix's garbage, generations and linking |
| `src/replaced.rs`, `opens.rs`, `unwatched.rs` | older builds, and the watch that dates them, or a Mac's lack of one |
| `src/writers.rs`, `freeze.rs`, `freezer.rs` | who writes most, and freezing them through systemd |
| `src/desktop.rs`, `notifier.rs` | notices and power saving over D-Bus, or on a Mac |
| `tests/guard/`            | the scenarios, one file per module                       |

## Working on It

`direnv` enters the flake's development shell in this directory, with cargo,
clippy and rustfmt, and points `TMPDIR` at the account's runtime tmpfs.
`cargo test` runs every scenario in about ten seconds there, and the package
runs them again as it builds, on `/dev/shm`. On Btrfs instead, every pass a
scenario steps would commit the machine's own disk, or wait for its commit.
`nix fmt` from the repository root formats the Rust as it formats everything
else.

- Write a scenario's test first, against the crate's public surface, and see
  it fail on its assertion before making it pass.
- Before trusting a test of a branch already written, break the branch and
  watch the test fail.
- A scenario never reaches the machine's own Nix, systemd or desktop. Any
  config a test steps a daemon with names a fake or missing `nix.bin` and
  `nix.store`, and a system config a fake `dedupe.systemd_run`: the defaults
  name the real ones, and once ran a real `nix store gc` from a test. A
  stub never falls back to a program by bare name either, since `PATH`
  finds the real one: a stub of an unconfined run once ran the real
  `systemd-run`. After
  adding anything that runs a program by default, point the default at a
  script that logs its calls and run every scenario: nothing may call it.
- A test that waits on a thread to show nothing happened gives it time to,
  and fakes are scripts that refuse an argument they don't expect before
  doing anything.
- The package builds in Nix's sandbox, at the batch scheduling policy and on
  a disk other builds are filling, so a scenario expects the policy it runs
  under and never the real disk's trend.

[1]: nixos.nix
[2]: home.nix
[3]: darwin.nix
