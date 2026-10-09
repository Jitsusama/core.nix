# 0007: Hardware Lives Here

## Status

Accepted.

## Context

Each laptop needs things only its model needs: a kernel compiled for its CPU,
a patch for one of its drivers, its firmware, and workarounds for its
quirks. Something has to hold them, and say where each came from, so a reader
can tell a model's needs apart from a person's choices.

## Decision

Each model gets a NixOS module at `hardware/<model>/nixos.nix`, named after
the model as its maker names it, such as `dell-xps-14-da14260`, and exported
under that name. It imports the kernel module and sets the kernel's CPU, so
the hardware decides the kernel. It holds only what the model needs, and each
setting says why, with its source when it came from someone else: upstream,
nixos-hardware or Omarchy. A machine imports its hardware beside its roles.

## Alternatives

- **nixos-hardware's profiles.** The community's place for this, and the
  first place to look. It has no merged profile for the XPS 14, and its
  profiles choose a kernel of their own and set much of what they set with
  `lib.mkDefault`, which leaves a reader unsure what wins. Where a merged
  profile fits, a hardware module can import it and keep only the difference.
- **Hardware in each machine's directory.** The kernel's CPU option, its
  build and its checks are core's, and core's CI couldn't check a model
  defined elsewhere. A model used by machines in two repositories would be
  written twice, since machine repositories don't import each other.
- **`nixos-generate-config`'s hardware file.** It records what it detected
  without saying why, and mixes the model's needs with the machine's disks.
  The disks stay with the machine; the rest moves here.

## Consequences

- Each model's kernel is its own build, and its configuration is checked with
  its patches by `nix flake check`.
- A model's quirks are written down once, where every machine of that model
  finds them.
- core.nix names hardware makers and models. That's not employer-specific, so
  it's allowed here.
