# The Disk Guard

A Rust daemon that keeps a machine's disks from filling, taking only what
regenerates, cheapest first, and never what a build or a person is using. It
runs twice: `disk-guard system`, a root service that owns what lies outside
every home, and `disk-guard user` in each account, which owns the home.
[`nixos.nix`][1] and [`home.nix`][2] render each one's config and run it.

## How It Decides

- **Bands.** Each filesystem a source or the store lies on is healthy, under
  pressure, urgent or critical by its free space (15%, 8% and 3% by default),
  and a pass reclaims towards the target (20%). A forecast starts a pass
  before the next band arrives. Btrfs pools are one filesystem however many
  subvolumes they hold.
- **Order.** Below healthy a pass takes, in turn: waste (orphaned targets,
  oversized logs, Nix's leftover build directories), Nix generations and
  garbage, idle build variants and cold scratch, producers' own collection,
  caches cheapest to refill first, idle targets and installs, and last what
  is occupied. While healthy, routine upkeep runs at most every six hours,
  only in a spare moment: on mains or a battery over half, ten minutes after
  boot or resume, with nothing stalling on the disk or memory.
- **Windows.** How long a thing has to sit idle before it goes narrows with
  each band, and counts only time the machine was awake. `src/plan.rs` holds
  the table.
- **Use.** Before and after it moves a target aside, the guard judges whether
  any process uses it: a build holding cargo's or rustc's locks, a process
  with a file, map or working directory beneath it, or build variables
  naming it. Busy stays; a target only sat in waits for critical; a process
  it can't read keeps everything it might use, unless the system daemon
  vouches for it.
- **Removal.** Everything is reached through directories opened beneath the
  source's root, never through a symlink, another filesystem or `..`. A
  target is renamed into a trash beside it, judged again, then emptied; a
  disk too full to rename deletes in place. Each step is in the journal, and
  a guard starting after a crash empties what was begun.
- **Nix.** The system daemon lists the dead paths in a spare moment and
  deletes them oldest first, keeping three days' within a budget of 10% of
  the filesystem. It expires generations, never the current, running or
  booted one, the newest five, or one with a boot entry. It links the
  store's identical files weekly in a scope of its own at idle priority,
  stopping when the moment ends. An account's guard on the same filesystem
  asks it to collect rather than collecting itself; with no system daemon
  answering, it collects with `nix store gc --max` from urgent.

## Where Things Live

| Path                      | Holds                                                    |
| ------------------------- | -------------------------------------------------------- |
| `src/config.rs`           | the config each daemon reads, and what it refuses        |
| `src/band.rs`, `decide.rs` | bands, the forecast and the moment                      |
| `src/plan.rs`, `pass.rs`  | the order, the windows, and a pass that measures as it goes |
| `src/survey.rs`           | each source's candidates                                 |
| `src/anchor.rs`           | removal that never leaves its root                       |
| `src/activity.rs`, `locks.rs` | whether something is in use                          |
| `src/host.rs`, `linux.rs` | the machine the guard runs on                            |
| `src/daemon.rs`, `serve.rs` | a daemon's step and its loop                           |
| `src/socket.rs`, `vouch.rs` | what the two daemons say to each other                 |
| `src/garbage.rs`, `generations.rs`, `dedupe.rs` | Nix's garbage, generations and linking |
| `tests/guard/`            | the scenarios, one file per module                       |

## Working on It

`direnv` enters the flake's development shell in this directory, with cargo,
clippy and rustfmt. `cargo test` runs every scenario in about ten seconds,
and the package runs them again as it builds. `nix fmt` from the repository
root formats the Rust as it formats everything else.

- Write a scenario's test first, against the crate's public surface, and see
  it fail on its assertion before making it pass.
- Before trusting a test of a branch already written, break the branch and
  watch the test fail.
- A scenario never reaches the machine's own Nix, systemd or desktop. Any
  config a test steps a daemon with names a fake or missing `nix.bin` and
  `nix.store`, and a system config a fake `dedupe.systemd_run`: the defaults
  name the real ones, and once ran a real `nix store gc` from a test. After
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
