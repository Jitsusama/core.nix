# Roles

What a machine is for, one folder per role, each file only importing modules
and other roles. A file's name says its module system, as everywhere in
core.nix, and each opens with what it brings. A system role gives its
home-manager half to every account through `home-manager.sharedModules`, and
`workstation`, `graphical` and `laptop` each import `base`.
[docs/architecture.md][architecture] says how roles fit together.

| Here           | For                                | Files                                 |
| -------------- | ---------------------------------- | ------------------------------------- |
| `base/`        | every machine, a server included   | `nixos.nix`, `darwin.nix`, `home.nix` |
| `workstation/` | a machine Joel writes code on      | `nixos.nix`, `darwin.nix`, `home.nix` |
| `graphical/`   | a machine with a screen before him | `nixos.nix`, `darwin.nix`             |
| `laptop/`      | a NixOS machine he carries         | `nixos.nix`                           |

[architecture]: ../docs/architecture.md
