# Delivery

How a change gets from a branch to Joel's screen quickly, and the habits
that keep it quick for every session sharing the builders. [testing.md][1]
says what each check proves and where a case is proven; this says how to
spend them. Every rule here comes from a day the cycle went slow.

## The Targets

They're measured from the raw records: `nix-offload/there.jsonl` under
XDG's state folder for every run sent to a builder, the lent builder's
`changed-checks/runs.jsonl` for each evaluation, and the merges on main.

- Merge to on: a median under 18 minutes from a merge to the switch that
  puts it on the machine. The follower keeps switches 30 minutes apart,
  since on 9 November the launcher stopped taking keys after five shell
  restarts in half an hour, so a merge waits 15 minutes on average for
  the next. The target goes back to 8 once the launcher is shown to
  survive repeated restarts and the spacing can shrink while Joel is away.
- Last commit to merge: a median under 20 minutes for a change outside
  [`must-run.txt`][2]'s areas.
- A VM run's wait for a builder: a median under 2 minutes.
- The machine Joel uses: CPU pressure over five minutes under 15% from
  agents' work.
- The net: a merge that breaks a check, a VM test or any other, is caught
  within an hour of landing, so a round runs as many tests at once as the VM builders hold.
- Memory: no machine-wide out-of-memory kill for a week, and swap rarely
  touched. Sessions that only wait are parked or ended rather than held.
- Runs and tokens per landing: falling week on week, each group against
  its own last week, since landings differ too much in size to compare
  across groups.

Every hour the figures go to whoever steers the work as one short card:
the medians and p90s above, the VM wait, the machine's load, runs and
tokens per landing, the sessions open and the memory they hold, and the
top bottleneck with its owner and fix. A missed target raises an alarm the
moment it is missed, not at the next card.

The watch runs outside every session, so it outlives the one keeping the
cycle fast: [cadence][5] writes the hour's card to
`~/.local/state/cadence/card.txt` every five minutes, asks that session
once about each missed target, and asks the orchestrator above it when the
session doesn't answer within twenty minutes or is gone for ten. When no
card has gone for 65 minutes, cadence sends the orchestrator its own rather
than ask, since asking waits on the session's turn.
`jitsusama.cadence.watcher` and `jitsusama.cadence.orchestrator` name them by
their mesh addresses; with neither, it writes the card alone.

Each Friday at 18:00, `cadence review` sends the orchestrator, and the
session `jitsusama.cadence.owners` names for `review`, the week's noise
and blind spots. Noise is, by sender, the notices, looks, landings and
messages nobody acted on or that came again. Blind spots are pains Joel
had in an area no reading from his side names, axes with no reading from
his side, and readings in spec in an area his pains named that week.

It keeps the sessions up too. Given `jitsusama.cadence.revive.restart`,
the words that start a session, each tick first starts again every
orchestrator and every orchestrator's session whose pi died, on its own
log, puts its window back and tells the orchestrator. An orchestrator is one
named, or one with none above it that hears its own `#orchestration/`
group or that another session names, gone or running. A session whose pi
ended on purpose, by quitting or by a signal such as `mesh_end`'s,
as pi's own record of the session says, or whose quest is done, stays down, and so does one a boot
ended, its pi last running in an earlier boot: Joel reopens those when he
asks. On 2026-11-07 a harness
install killed 22 sessions as they reloaded between 01:09 and 01:52, and
each was started again by the next tick or sooner. One started three times in an hour is
left down and reported, since something kills it as it starts. A session a
holder still holds, its pi parked, is left to wake in its own window, since
one started beside it would write the same log. Before a session is
started again, every kitty window left showing it dead is closed, its gone
holder's or one the harness marked as the session's with no pi under it,
so a restart replaces a window rather than adding one. Nothing is started until
the desktop has been up 15 minutes, since before sign-in there is nowhere to
open a window.

On 2026-11-05 the last commit to merge took a median 41 minutes from 17:00
to 21:00 against 6 from 06:00 to 12:00, and a subtests run that takes 3
minutes on a quiet builder took 30. The builders were full, and most of what
filled them was runs that told nobody anything new.

## Spending the Builders

- A VM test runs whole before a merge only when `merge-net needs` names
  it. Otherwise the subtests that reach the change run, with
  `run-subtests --there`, and the net runs the rest after the merge.
- A test failing on main fails on every branch. Before reading a failure
  as the change's own, look for it in the net's results and in
  [`quarantine.json`][3], and never run it again to see main's failure
  twice: on 2026-11-05 two tests failing on main failed in 43 runs, about
  18 hours of runs end to end, across 13 trees.
- A run isn't sent twice for one commit. A pass is kept and answered
  without building; a failure is read from its log, fixed, and the fix
  sent. That day 42 VM runs repeated an earlier run's commit and tests.
- Evaluation leaves the machine. `changed-checks` sends its whole run
  unless asked `--here`; a bare `nix flake check` held 24.5 GB beside
  Joel's work on 2026-10-25. The net's `nix eval` of one test at a merge
  goes through `nix-offload there --run` too: on 2026-11-11 four at once
  here held 3.4 GB while a search ran.
- A gate is a command of its own. The harness's queue lets a run straight
  to the builder only when every heavy step in its command sends its work
  away, so a `git push` in the same command kept one 18 minutes at place 6
  on 2026-11-10. `nix-offload there --changed` in the tree is the same gate
  and waits behind nothing here; that afternoon a `changed-checks` run
  waited a median 3 minutes for memory it never used.
- A test suite that isn't a Nix check runs on a builder too: `nix-offload
  there --run [--copy PATH]... -- COMMAND` runs it in a checkout of the
  tree's commit, with the store paths it needs sent first. On 2026-11-10
  suites run here held Joel's machine at a load of 99 to 229.
- Commit before gating. An uncommitted tree's `changed-checks` runs on the
  machine Joel uses, since a builder can only fetch a commit: on
  2026-11-06 one compiled a whole machine's closure there.
- `changed-checks --build` builds every VM test a change moved, whole, and
  no gate asks for it: on 2026-11-06 one run for a change to the shell
  alone held every VM builder for 66 minutes.
- A flake.lock bump of one input that moves no VM test, by `merge-net
  needs`, is gated by `changed-checks` alone; every other flake.lock change
  runs everything.

## Waiting

- A long run goes to the background, or to a tool's own watch, and its
  result comes back by itself. Nothing polls a builder, a queue or a
  change in a loop.
- What's done is closed: a session whose work has landed, a tunnel, a
  headless browser, a watch. Each holds memory and cores the next build
  needs.

## When the Cycle Slows

The rule that held the speed goes here, with the day it was learned, and
the measurement that shows it. A rule that no longer holds a number goes.

- A Rust program with hundreds of crates that changes often builds its
  dependencies once, apart from itself ([`modules/cargo/dependencies.nix`][4]).
  On 2026-11-06 the reader compiled 330 crates on every change, 16.6
  minutes before it could reach the machine; it now compiles its own crate
  in 2. A program whose whole build takes a few minutes keeps the plain
  build: the guide's takes 2.3, and moving it would cost the install's VM
  tests.
- The net runs twelve VM tests at once, three builders' room. At four on
  2026-11-06 a round of 33 tests took 85 minutes and filled one builder of
  six, while 13 merges landed behind it untested, finishing 35 runs in 81
  minutes; at twelve the next round finished 29 in its first 33.
- A merge whose build leaves the machine unchanged counts as on from the
  moment the switch finds that, not from the next switch that changes
  something; counting it late made a quarantine look 54 minutes slow.

[1]: testing.md
[2]: ../tools/merge-net/must-run.txt
[3]: ../tools/merge-net/quarantine.json
[4]: ../modules/cargo/dependencies.nix
[5]: ../modules/cadence/home.nix
