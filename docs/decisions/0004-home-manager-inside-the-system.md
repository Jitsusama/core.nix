# 0004: home-manager Runs Inside the System

## Status

Accepted, 2026-10-07.

## Context

The Macs applied their system with `darwin-rebuild` and their account with a
standalone `home-manager switch`, each from its own configuration with its
own package set. Two commands meant two moments when a machine was half
updated, and two package sets meant two copies of nixpkgs to evaluate.

## Decision

Each machine builds one NixOS or nix-darwin system with home-manager's module
inside it, configured with `useGlobalPkgs` and `useUserPackages`. There are no
standalone home-manager configurations.

## Alternatives

- **Standalone home-manager.** Applies without root and suits machines whose
  system Nix does not manage. Every machine here has its system managed by
  Nix, so it only adds the second command.

## Consequences

- One `nixos-rebuild switch` or `darwin-rebuild switch` applies the machine
  and the account together, and rolls back together.
- home-manager uses the system's nixpkgs and its settings, such as allowing
  unfree packages, so they are set once, by the `nixpkgs` module.
- Applying the account needs `sudo`, as applying the system already did.
