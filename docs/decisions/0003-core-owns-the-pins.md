# 0003: core.nix Owns the Pins

## Status

Accepted, 2026-10-07. Replaces the earlier rule that core.nix has no nixpkgs
input and each machine repository pins its own.

## Context

Each machine repository pinned its own nixpkgs, home-manager and nix-darwin. The
two drifted apart, and no combination was ever tested before a machine took
it. The machines are also about to share expensive builds, a kernel first,
which a binary cache only serves to machines on the exact nixpkgs it was
built from.

## Decision

core.nix pins nixpkgs, home-manager, nix-darwin and its tooling. Machine
repositories follow those pins:

```nix
nixpkgs.follows = "core/nixpkgs";
```

A scheduled workflow updates the pins weekly and opens a pull request only
when every check passes against them.

## Alternatives

- **Each machine repository pins.** The previous rule. It keeps core.nix free of
  nixpkgs, but tests nothing and lets machines drift.
- **A separate pins flake.** One more repository to keep in step, for no gain
  over core.nix doing it.

## Consequences

- Every machine runs a combination CI has checked.
- `nix flake update core` is the whole update for a machine repository.
- Shared builds hit the cache on every machine.
- A machine repository can still override a pin for itself with its own input,
  and gives up those guarantees for that input when it does.
