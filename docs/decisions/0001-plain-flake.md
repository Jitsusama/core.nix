# 0001: A Plain Flake, With `flake.nix` as the Index

## Status

Accepted, 2026-10-07.

## Context

core.nix began as two trees, `home-manager/<tool>/` and `nix-darwin/<tool>/`,
each exported as a bare path that machine repositories imported whole. That got
the important things right: trust decided which repository code lived in,
each tool had one cohesive directory, and everything was plain module-system
Nix with no framework.

Adding NixOS as a third platform showed its limits:

- A tool touching two platforms was split across two trees.
- Importing was all or nothing; leaving one tool out meant listing the rest.
- Inputs reached modules through `specialArgs`, which can't be traced from
  the module that uses them.
- Bare-path outputs are not a flake output any tool understands.
- Nothing was tested.

Above all, the code has to stay easy to follow: a reader who opens any file
should be able to find out how it fits, by opening other files.

## Decision

core.nix is a plain flake. Each tool has one directory under `modules/`, with
one ordinary module per system, named by kind: `home.nix`, `darwin.nix`,
`nixos.nix`, or `system.nix` for both NixOS and nix-darwin. `flake.nix` names
every module and role in `homeModules`, `darwinModules` and `nixosModules`,
and is the one place that gives a module the inputs it needs, as an outer
function taking them by name.

## Alternatives

- **Per-platform trees, kept.** Simple and familiar, but it keeps every limit
  above.
- **The dendritic pattern on flake-parts and import-tree.** Built in full and
  then dropped. Every file is a flake-parts module found by scanning the tree,
  and machines choose features by name through a resolver. It made a machine's
  configuration short, but put a framework, file discovery and about 600
  lines of our own machinery between a module and the machine using it. A
  reader could no longer open a file and see how it fits.
- **den.** Grew out of the dendritic pattern into a framework with its own
  vocabulary for hosts, users and aspects: more to learn, and more
  indirection still.
- **blueprint and Snowfall.** Map a fixed directory layout onto outputs by
  convention, which is discovery again, and awkward when a tool spans
  platforms.

## Consequences

- There is no dependency beyond Nix, nixpkgs, home-manager and nix-darwin,
  and nothing a reader must learn first.
- Adding a module takes one line in `flake.nix` besides the module itself.
- The loop over systems for checks and the formatter is written out by hand,
  in two lines.
- A tool with modules for two systems, such as Homebrew, is brought in by a
  system role that imports one half and gives accounts the other. Both are
  visible in the role, but nothing checks they stay in step.
- Machine repositories migrate onto the new outputs; until they do, core.nix
  keeps exporting the old ones beside them.
