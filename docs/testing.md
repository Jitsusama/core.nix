# Testing

`nix flake check` runs everything below, and CI runs it on every pull request
and on main. Every check is defined in [`checks.nix`][1]. Most evaluate
machines without building them, which takes about three minutes on a 16-core
laptop once the inputs are downloaded. The rest build on x86_64-linux, because
nothing short of running them proves what they cover: the kernel's
configuration for every piece of hardware, niri and kitty reading back their
files, and a virtual machine taken through [the whole install][3].

## What Each Check Proves

| Check                             | Proves                                                    |
| --------------------------------- | --------------------------------------------------------- |
| `nixos-<name>`                    | the NixOS module or role evaluates on a bare machine      |
| `darwin-<name>`                   | the nix-darwin module or role evaluates on a bare Mac     |
| `home-on-nixos-<name>`            | the home-manager module or role evaluates on NixOS        |
| `home-on-darwin-<name>`           | the home-manager module or role evaluates on a Mac        |
| `nixos-role-twice-is-once`        | importing a NixOS role twice leaves the machine unchanged |
| `darwin-role-twice-is-once`       | the same for a nix-darwin role                            |
| `disabling-a-module-removes-it`   | `disabledModules` removes a module a role brought in      |
| `one-ssh-agent`                   | a workstation with a screen answers SSH with the TPM only |
| `onepassword-works-with-chrome`   | 1Password has its Chrome helper and unlocks with polkit   |
| `example-nixos`                   | the NixOS example machine evaluates                       |
| `example-laptop`                  | the laptop example evaluates                              |
| `example-darwin`                  | the Mac example machine evaluates                         |
| `formatting`                      | everything is formatted and passes the linters            |
| `nothing-work-specific`           | no module, role or example names anything work-only       |
| `kernel-settings-hold`            | the kernel's patches apply and every setting holds        |
| `kernel-settings-hold-<model>`    | the same for the kernel that hardware builds              |
| `niri-accepts-its-configuration`  | niri loads the files an account gets                      |
| `kitty-accepts-its-configuration` | kitty loads its files without a complaint                 |
| `btop-draws-in-the-theme`         | btop draws in the theme's colours, not its own            |
| `neovim-starts-in-the-theme`      | Neovim starts in bamboo, transparent, without a complaint |
| `cargo-links-with-mold`           | cargo links an account's Rust builds with mold            |
| `secure-boot-installs`            | the whole install works, on a VM with a TPM               |
| `ssh-tpm-agent-signs`             | an SSH key in a VM's TPM signs, its PIN asked for         |
| `commits-are-signed`              | Joel's commits and agents' are signed with their own keys |
| `keyring-opens-without-asking`    | the keyring opens with its TPM-sealed password, unasked   |
| `speakers-are-tuned`              | the XPS 14's speakers hear the tuning, headphones don't   |
| `desktop-works`                   | the desktop shows, opens, locks and asks, in the theme    |

A bare machine has nothing but home-manager, nixpkgs' settings and one
account, which every machine has. So a module that quietly relies on another,
or sets an option that doesn't exist, fails its own check and the check of
every role that imports it. A system role's home-manager half is evaluated in
the account, so it is checked too.

The composition checks hold the promises roles make when combined, and the
examples use core only through its outputs, as a machine repository does, so
they test the library from a consumer's side.

The kernel check builds only the configuration: the patched source, run
through Kconfig. [The kernel guide][2] says what it catches.

The install check builds a disk image with disko and boots it under QEMU with
a software TPM, then follows [the install guide][3] step by step: the keys are
made and enrolled, the disk is bound to the TPM and a PIN, the passphrase is
wiped, and the TPM refuses once a measured PCR changes. It lives in
[`tests/secure-boot.nix`][4], since it's longer than the rest put together.

The checks read the modules from `flake.nix`, so a new module or role is
checked as soon as it is named there. The Mac checks evaluate on Linux, so one
machine checks every platform. The home modules only a Linux account uses,
listed as `linuxOnly` in `checks.nix`, are checked on NixOS alone.

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
[2]: kernel.md
[3]: installing.md
[4]: ../tests/secure-boot.nix
