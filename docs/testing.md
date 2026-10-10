# Testing

Checks are for the loop of making a change: each one has to tell whoever is
changing core.nix something they need to know, quickly, and something no
cheaper check already tells them. Every check is defined in
[`checks.nix`][1], and nothing runs on a schedule.

## The Loop

```sh
nix build .#checks.x86_64-linux.<name>   # the checks the change touches, as you go
nix flake check                          # everything, before pushing
```

`nix flake check` runs every check, but Nix reuses any check whose inputs
didn't change, so a change pays only for the checks it can affect. With the
builds already done, what's left is evaluation, about half a minute on a
16-core laptop. A change to the boot path reruns the install test; a change
to a theme reruns the programs that read it and the desktop test.

To try a change on a real machine, build it from its repository with core
pointed at the checkout, and switch to it once it builds:

```sh
nixos-rebuild build --flake .#<machine> --override-input core path:$HOME/src/core.nix
```

CI runs the same checks on every pull request and on main, as a backstop for
a push that skipped them. Nothing waits on it: a machine repository can pin a
branch's commit as soon as it's pushed, and pull requests merge with a merge
commit, so that commit stays on main.

## The Pyramid

From the bottom up, each tier costs more than the one below and has to show
something it can't.

### Promises

Read from evaluated machines, without building them. They catch an option
that changed meaning, a role that brings what it shouldn't, or a setting that
quietly stopped holding.

| Check                               | Proves                                                    |
| ----------------------------------- | --------------------------------------------------------- |
| `nixos-role-twice-is-once`          | importing a NixOS role twice leaves the machine unchanged |
| `darwin-role-twice-is-once`         | the same for a nix-darwin role                            |
| `disabling-a-module-removes-it`     | `disabledModules` removes a module a role brought in      |
| `one-ssh-agent`                     | a workstation with a screen answers SSH with the TPM only |
| `onepassword-works-with-chrome`     | 1Password has its Chrome helper and unlocks with polkit   |
| `yubikey-on-every-workstation`      | Linux and Mac workstations both have the YubiKey's tools  |
| `zsh-is-the-login-shell`            | an account on NixOS logs in to zsh, as one on a Mac does  |
| `disks-go-by-their-name`            | a disk's partitions and open volume carry its name        |
| `booted-generation-stays-installed` | a switch never removes the generation the machine booted  |
| `formatting`                        | everything is formatted and passes the linters            |
| `stands-alone`                      | nothing here names a machine, its repository or employer  |
| `ci-runs-every-vm-test`             | CI's workflow runs every virtual machine test             |

### Examples

Whole machines that use core only through its outputs, as a machine
repository does. Between them they import every role, every module and the
hardware core knows, so a module that sets an option that doesn't exist, or
relies on one nothing brings, fails here.

| Check            | Proves                                                     |
| ---------------- | ---------------------------------------------------------- |
| `example-nixos`  | a NixOS workstation with a screen evaluates                |
| `example-laptop` | a laptop on the XPS 14, its disk and Secure Boot, evaluates |
| `example-darwin` | a Mac workstation evaluates                                |

### Programs

The real program reads the files an account gets, because a program that
can't use its configuration usually falls back to its own without a word.
The kernel checks build only its configuration: the patched source, run
through Kconfig, which drops a setting whose dependencies aren't met. [The
kernel guide][2] says what that catches.

| Check                             | Proves                                                    |
| --------------------------------- | --------------------------------------------------------- |
| `kernel-settings-hold`            | the kernel's patches apply and every setting holds        |
| `kernel-settings-hold-<model>`    | the same for the kernel that hardware builds              |
| `niri-accepts-its-configuration`  | niri loads the files an account gets                      |
| `niri-cursor-has-its-shapes`      | the cursor theme niri names has the shapes programs ask for |
| `kitty-accepts-its-configuration` | kitty loads its files without a complaint                 |
| `btop-draws-in-the-theme`         | btop draws in the theme's colours, not its own            |
| `neovim-starts-in-the-theme`      | Neovim starts in bamboo, transparent, without a complaint |
| `cargo-links-with-mold`           | cargo links an account's Rust builds with mold            |

### Virtual Machines

Each boots a machine under QEMU, which takes minutes, so each covers what only
a running system shows.

| Check                             | Proves                                                  |
| --------------------------------- | ------------------------------------------------------- |
| `vm-secure-boot-installs`         | the whole install works, and the disk opens only for it |
| `vm-commits-are-signed`           | commits are signed with the TPM's keys, PINs asked for  |
| `vm-keyring-opens-without-asking` | the keyring opens with its TPM-sealed password, unasked |
| `vm-speakers-are-tuned`           | the XPS 14's speakers hear the tuning, headphones don't |
| `vm-desktop-works`                | the desktop shows, opens, locks and asks, in the theme  |

The install test builds a disk image with disko and boots it with a software
TPM, then follows [the install guide][3] step by step: the keys are made and
enrolled, the disk is bound to the TPM and a PIN, the passphrase is wiped,
and the TPM refuses once a measured PCR changes. On the way it checks that
enrolling without checksums refuses the option ROM of QEMU's network card,
which the firmware runs. It lives in [`tests/secure-boot.nix`][4].

## Earning a Check

- A new check goes in the lowest tier that can show what it covers. Most
  belong with the promises.
- A check above the promises says what the tiers below it miss.
- What a running system shows goes in an existing virtual machine where it
  fits. A new one has to cover something none of them can, and its name
  starts with `vm-`, which is how CI gives it a runner of its own.
- A check that only repeats another one's signal goes.
- A new module is evaluated by the examples once a role or another module
  imports it. One that nothing imports goes into an example.

## Checking a Machine Repository Against a Change

The machine repositories evaluate their machines in their own checks. To see
what a change here does to one, compare the derivations it produces before
and after:

```sh
nix eval --raw --override-input core path:$HOME/src/core.nix \
  .#darwinConfigurations.<machine>.config.system.build.toplevel.drvPath
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
