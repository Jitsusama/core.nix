# core.nix

The shared part of every one of Joel's machines, as a Nix library: the NixOS,
nix-darwin and home-manager modules each machine is built from, the roles that
group them, and the hardware each laptop needs. It holds no machines and nothing
tied to an employer; those live in the repositories that build on it.

## 🧭 Where It Fits

A machine repository takes core.nix as an input and builds its machines from
the modules and roles here, adding what only its own machines need. core.nix
knows nothing about the repositories that use it, so anything only some
machines want belongs in theirs.

## 🧩 How It Fits Together

Everything here is plain Nix: no framework, nothing discovered from the file
tree, nothing passed in from somewhere you can't see. [`flake.nix`][1] is the
index. It names every module and role, and it is the one place that gives a
module the inputs it needs. To see how any file fits, search for its path in
`flake.nix`.

A module is an ordinary module that configures one tool, and its file name
says which module system it belongs to:

```nix
# modules/bat/home.nix, trimmed: a home-manager module
{
  programs.bat = {
    enable = true;
    config.style = "changes";
  };

  programs.zsh.shellAliases.cat = "bat";
}
```

`flake.nix` gives it a name:

```nix
homeModules = {
  bat = ./modules/bat/home.nix;
  # ...
};
```

A role says what a machine is for, and only imports:

```nix
# roles/workstation/home.nix: a machine Joel writes code on
{ homeModules }:
{
  imports = [
    homeModules.base
    homeModules.neovim
    # ...
  ];
}
```

A machine, in its own repository, calls the stock builder and imports the
roles it plays. A system role brings its home-manager half to every account:

```nix
nixosConfigurations.laptop = nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    ./machines/laptop
  ];
};
```

Choosing a module is importing it. There are no `enable` switches to turn
modules on, and options exist only for settings that differ between machines,
such as `jitsusama.identity.email`.

## 📁 Layout

```text
core.nix/
├── flake.nix              # the index: every module, role and input
├── modules/<tool>/        # one directory per tool
│   ├── home.nix           #   its home-manager module
│   ├── darwin.nix         #   its nix-darwin module
│   ├── nixos.nix          #   its NixOS module
│   ├── system.nix         #   one module for both NixOS and nix-darwin
│   └── <config files>     #   the tool's own configuration, in its own format
├── roles/<role>/          # what a machine is for, one file per module system
├── hardware/<model>/      # what one machine model needs, kernel included
├── examples/              # machines written the way a machine repository would
├── checks.nix             # what `nix flake check` runs
├── tests/                 # checks that boot a virtual machine
├── treefmt.nix            # what `nix fmt` runs
└── docs/                  # how it works, and why
```

## 🚀 Using It

A machine repository takes core.nix as an input and follows its pins, so every
machine runs a combination CI has checked:

```nix
inputs = {
  core.url = "github:Jitsusama/core.nix";
  nixpkgs.follows = "core/nixpkgs";
  home-manager.follows = "core/home-manager";
  nix-darwin.follows = "core/nix-darwin";
};
```

Machines are applied with the stock commands, `sudo darwin-rebuild switch` or
`sudo nixos-rebuild switch`. [Machines][2] has the rest, and [`examples/`][8]
shows a whole machine of each kind.

## 🔧 Working on It

```sh
nix fmt            # format and lint everything
nix flake check    # every check; reruns only the ones a change can affect
```

To try a change on a real machine before it lands, point its repository at
your checkout:

```sh
nix build --override-input core path:$HOME/src/core.nix \
  .#darwinConfigurations.<machine>.config.system.build.toplevel
```

## 📚 Documentation

- [Architecture][3]: the vocabulary, how the pieces connect, and how to trace
  any of them.
- [Conventions][4]: the rules the code follows.
- [Modules][5]: adding a module or a role, and changing one from another
  repository.
- [Machines][2]: building a machine from core.nix.
- [Installing][11]: a laptop from a blank disk to Secure Boot and a disk the
  TPM unlocks.
- [Testing][6]: what the checks catch.
- [The kernel][9]: the kernel core.nix builds for NixOS, and how to change it.
- [Hardware][10]: each machine model core.nix supports, and what's left.
- [Decisions][7]: what was decided, against what, and why.

[1]: flake.nix
[2]: docs/machines.md
[3]: docs/architecture.md
[4]: docs/conventions.md
[5]: docs/modules.md
[6]: docs/testing.md
[7]: docs/decisions/README.md
[8]: examples/
[9]: docs/kernel.md
[10]: docs/hardware.md
[11]: docs/installing.md
