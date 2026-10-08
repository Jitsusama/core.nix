# 0002: Choosing Is Importing

## Status

Accepted, 2026-10-07.

## Context

Machines differ in which tools they have: a Mac has Homebrew and its
terminals, a NixOS laptop has its compositor, and a server would have neither.
Something has to say which machine gets what.

A common answer is to import every module everywhere and give each an
`enable` option, so that a machine turns tools on by setting switches. A
machine's tools are then spread across switches set in many files, and a
reader has to search for every one of them to know what a machine runs.

## Decision

A machine gets a tool by importing its module, and goes without by not
importing it. Modules have no `enable` option of their own. Roles group the
imports for what a machine is for, and are modules that do nothing but
import. Options exist only for settings that differ between machines, under
`jitsusama.*`.

## Alternatives

- **An `enable` option on every module.** Familiar from NixOS itself, and
  lets a machine switch a tool off after a role turned it on. But the
  answer to "what does this machine run" is scattered across switches, and
  every module carries the same guard.
- **Choosing modules by name, as strings, resolved by a builder.** Tried with
  the dendritic design; see [0001][1]. Short to write, but a string can't be
  followed to the file it names without knowing the resolver.

## Consequences

- What a machine runs is the list of roles it imports, and each role is one
  short list of imports that an editor can follow line by line.
- A machine that wants most of a role but not all of it names the modules it
  goes without in `disabledModules`, or imports a smaller role and adds to
  it. If that comes up often, the role is split.
- Importing a module twice is harmless, so roles can share modules and build
  on one another. A module that takes an input is a function's result rather
  than a path, so `flake.nix` gives it its file's identity through
  `moduleFrom`; [0005][2] says more.

[1]: 0001-plain-flake.md
[2]: 0005-modules-and-options-are-the-interface.md
