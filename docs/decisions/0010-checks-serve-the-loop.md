# 0010: Checks Serve the Loop

## Status

Accepted, 2026-10-10. Replaces [0003][1]'s scheduled pin update; core.nix
still owns the pins.

## Context

CI took 25 to 53 minutes a run, and changes waited on it before a machine
could take them. About 21 minutes of a 30-minute run went to evaluating each
of 123 modules alone on a bare machine of every platform, one after another.
That proved a module evaluates on its own, which the example machines already
show for every module together, and it built nothing. The virtual machine
tests took most of the rest. A weekly workflow updated the pins and ran
everything again, whether or not anyone was working on core.nix.

## Decision

Checks are for the loop of making a change. Each one has to tell whoever is
making it something they need, quickly, that no cheaper check already does,
and the heavier tiers have to earn their place: a few virtual machines above
real programs reading their files, above whole example machines, above
promises read from evaluated machines. [Testing][2] lists them and says how a
new one earns its place.

The checks run locally before a push, and `nix flake check` reruns only the
ones a change can affect. CI runs them again as a backstop, with the virtual
machines on runners of their own, and nothing waits on it. Nothing runs on a
schedule: the pins move when someone updates them, through the same loop.

## Alternatives

- **Keep the module matrix, and speed CI up** with parallel evaluation or
  bigger runners. Faster, but still paying for a signal the examples already
  give.
- **Run the heavy checks on a schedule.** A failure would arrive days after
  the change that caused it, to nobody working on it.
- **Keep the weekly pin update.** Updates would land unasked, each one a
  change nobody was there to try on a machine.

## Consequences

- A module that quietly relies on another passes as long as every machine
  that imports it also brings the other, which is when it matters.
- A module nothing imports isn't evaluated, so it goes into an example.
- Pins age until someone updates them: `nix flake update`, then the loop.
- Pull requests merge with a merge commit, so a machine repository can pin a
  branch's commit before the merge and keep it after.

[1]: 0003-core-owns-the-pins.md
[2]: ../testing.md
