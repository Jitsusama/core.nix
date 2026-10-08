# Testing

`nix flake check` runs everything below, and CI runs it on every pull request
and on main. Every check is defined in [`checks.nix`][1]. They evaluate
machines without building them, which takes about three minutes on a 16-core
laptop once the inputs are downloaded.

## What Each Check Proves

| Check                           | Proves                                                    |
| ------------------------------- | --------------------------------------------------------- |
| `nixos-<name>`                  | the NixOS module or role evaluates on a bare machine      |
| `darwin-<name>`                 | the nix-darwin module or role evaluates on a bare Mac     |
| `home-on-nixos-<name>`          | the home-manager module or role evaluates on NixOS        |
| `home-on-darwin-<name>`         | the home-manager module or role evaluates on a Mac        |
| `nixos-role-twice-is-once`      | importing a NixOS role twice leaves the machine unchanged |
| `darwin-role-twice-is-once`     | the same for a nix-darwin role                            |
| `disabling-a-module-removes-it` | `disabledModules` removes a module a role brought in      |
| `example-nixos`                 | the NixOS example machine evaluates                       |
| `example-darwin`                | the Mac example machine evaluates                         |
| `formatting`                    | everything is formatted and passes the linters            |
| `nothing-work-specific`         | no module, role or example names anything work-only       |

A bare machine has nothing but home-manager, nixpkgs' settings and one
account, which every machine has. So a module that quietly relies on another,
or sets an option that doesn't exist, fails its own check and the check of
every role that imports it. A system role's home-manager half is evaluated in
the account, so it is checked too.

The composition checks hold the promises roles make when combined, and the
examples use core only through its outputs, as a machine repository does, so
they test the library from a consumer's side.

The checks read the modules from `flake.nix`, so a new module or role is
checked as soon as it is named there. The Mac checks evaluate on Linux, so one
machine checks every platform.

## Running Part of It

```sh
nix build .#checks.x86_64-linux.darwin-zsh               # one module
nix build .#checks.x86_64-linux.darwin-workstation       # one role
nix build -L .#checks.x86_64-linux.nothing-work-specific # with its output
```

## Checking a Machine Repository Against a Change

The machine repositories evaluate their machines in their own checks. To see
what a change here does to one, compare the derivations it produces before
and after:

```sh
nix eval --raw --override-input core path:$HOME/src/core.nix \
  .#darwinConfigurations.methuselah.config.system.build.toplevel.drvPath
nix run nixpkgs#nix-diff -- before.drv after.drv
```

A refactor should leave the path unchanged. Anything else shows up in the
diff, file by file. A list in a different order is still a change, and needs
explaining before it lands.

## Trusting a New Check

Before relying on a new check, break what it covers and watch it fail. Undo
the break from a saved copy of the file, not from git, so that a staged break
isn't restored by accident.

[1]: ../checks.nix
