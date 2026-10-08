# 0006: Build the Kernel Here

## Status

Accepted, 2026-10-08.

## Context

Joel's NixOS laptops are development machines first: big builds, many
containers, a compositor and terminal that must never stutter while a build
runs. The kernel decides most of that: how the scheduler shares the CPU, when
the kernel lets itself be preempted, how often it ticks, and how it was
compiled. nixpkgs's kernels are configured to suit every NixOS machine, so
they make the safe choices on each.

He also wants to understand and control every part of these machines, and to
learn from building it, rather than trust someone else's choices.

## Decision

core.nix builds its own kernel, in `modules/kernel/`:

- **Source:** kernel.org's stable series, at the point release nixpkgs pins,
  through nixpkgs's own kernel builder.
- **Compiler:** Clang with ThinLTO, for the CPU each machine's hardware
  module names, so every machine runs a kernel tuned for its own cores.
- **Patches:** only small ones with a clear purpose and a maintainer who
  releases them for every stable series, each fetched from its author at a
  fixed commit with its hash. Today that's BORE alone.
- **Settings:** set by intent in `settings.nix`, one comment per intent,
  applied over nixpkgs's common configuration.
- **Strictness:** configuration errors fail the build, and a check proves
  every setting survives into the finished configuration.

## Alternatives

- **nixpkgs's kernel, or its Zen or XanMod variants.** Nothing to maintain,
  and cached. But they're configured for everyone, and the variants bundle
  many patches chosen by someone else.
- **CachyOS's kernel, prebuilt for NixOS.** The settings wanted here, already
  built. But it comes from a pre-patched source tree carrying dozens of
  patches, and from a cache outside Joel's control, which is hard to trace.
- **CachyOS's source tree, built here.** Built locally, but still dozens of
  patches nobody here chose one by one.
- **One kernel for every machine, at an ISA level such as x86-64-v3.** One
  build to cache, but it needs another out-of-tree patch, and no machine gets
  tuned for its own cores. Naming each CPU needs no patch at all.

## Consequences

- The kernel isn't in the NixOS cache, and each CPU gets its own build, which
  takes about an hour on a laptop. A cache of its own saves a machine that
  wait only when another machine with the same CPU built it first.
- Each new series takes some work: finding BORE's version for it and
  updating the hash. Point releases need none, unless BORE stops applying,
  which the configuration check catches.
- Rust is off, because LTO and BTF can't be combined with it.
