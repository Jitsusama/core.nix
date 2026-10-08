# 0005: Modules and Options Are the Interface

## Status

Accepted, 2026-10-07.

## Context

core.nix is a library. Two machine repositories build on it, one personal and
one for work, and it has to serve personal and work laptops on NixOS and
macOS, and perhaps servers later. Each repository adds its own layer and
changes some of what core sets: the work layer sets a different email, adds
its own zsh start-up code, and may drop a tool a role brought in.

So the library needs an interface: what it hands its consumers, and how they
change what it gives them. It must not become a framework they're forced into,
and every change a consumer makes should be traceable by opening files.

## Decision

- **Configuration is handed over as modules,** in `homeModules`,
  `darwinModules` and `nixosModules`. Modules and roles alike are plain
  modules a machine imports.
- **Settings a layer is expected to change are options** under `jitsusama.*`,
  declared by core's modules. Identity is the first.
- **Machine repositories build their own machines** with the stock
  `nixosSystem` and `darwinSystem`. core exports no function that builds one.
- **Every module has a fixed identity.** Modules imported by path have one
  already; `flake.nix` gives the rest their file's through `moduleFrom`. So a
  module can be imported twice, and named in `disabledModules`.
- **Consumers change what core sets with the module system alone,** in this
  order of preference: add to it, set a `jitsusama.*` option, replace one
  value with `lib.mkForce`, or remove a module with `disabledModules`. When
  more than one machine has to replace the same value, core gets an option
  for it.
- **The contract is the output names and the options.** File paths, and the
  values core gives other options, may change in any commit. Renaming or
  removing an output or an option is a breaking change, marked `!`.

## Alternatives

- **Functions that build machines,** such as a `mkHost` taking a list of
  roles, or roles called with arguments. Short to use, but the arguments are
  a second configuration language without types, documentation or merging,
  which a later layer can't change. The builder owns the call to
  `nixosSystem`, so NixOS's own documentation no longer applies as written.
- **Every core value wrapped in `lib.mkDefault`,** so a consumer's plain
  setting always wins. A machine setting one key of a settings group would
  silently drop core's other values in that group, unless every value were
  wrapped one by one.
- **Modules as function results without a fixed identity.** Simpler by a few
  lines, but the module system can't tell two copies apart. A module that two
  roles import would be merged twice, and `disabledModules` refuses it.

## Consequences

- A machine repository reads like any NixOS or nix-darwin configuration, and
  its machines can be built and applied with the stock commands.
- Changes a layer makes are visible where it makes them, as ordinary module
  definitions.
- core has to declare an option before a layer can change a setting cleanly,
  and `lib.mkForce` is the stopgap until it does.
- The `examples/` machines use only the outputs, so `nix flake check` tests
  the interface from a consumer's side.
- A module given its identity by `moduleFrom` sits one level deeper in the
  import tree, which can change the order its list entries are merged in. No
  module relies on that order; see the [conventions][1].

[1]: ../conventions.md#order-is-never-assumed
