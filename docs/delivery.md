# Delivery

How a change gets from a branch to Joel's screen quickly, and the habits
that keep it quick for every session sharing the builders. [testing.md][1]
says what each check proves and where a case is proven; this says how to
spend them. Every rule here comes from a day the cycle went slow.

## The Targets

They're measured from the raw records: `nix-offload/there.jsonl` under
XDG's state folder for every run sent to a builder, the lent builder's
`changed-checks/runs.jsonl` for each evaluation, and the merges on main.

- Merge to on: a median under 8 minutes from a merge to the switch that
  puts it on the machine.
- Last commit to merge: a median under 20 minutes for a change outside
  [`must-run.txt`][2]'s areas.
- A VM run's wait for a builder: a median under 2 minutes.
- The machine Joel uses: CPU pressure over five minutes under 15% from
  agents' work.

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
  Joel's work on 2026-10-25.

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

[1]: testing.md
[2]: ../tools/merge-net/must-run.txt
[3]: ../tools/merge-net/quarantine.json
