# Architecture

core.nix is a plain flake. Its design rests on one rule: every connection is a
path you can open or a name defined in a file you can open. Nothing is
discovered from the file tree, looked up by a string, or passed in from
somewhere you can't see. A reader who opens any file can find out how it
fits without knowing anything beyond Nix and the module system.

## Vocabulary

Each word has one meaning, used the same way in code, directories and docs.

| Word               | Means                                          | In code                                                 |
| ------------------ | ---------------------------------------------- | ------------------------------------------------------- |
| machine            | one computer                                   | `nixosConfigurations.laptop`                            |
| machine repository | a repository of machines built on core.nix     | any flake with core.nix as an input                     |
| module             | one program or subsystem, configured           | `modules/bat/` becomes `homeModules.bat`                |
| role               | what a machine is for                          | `roles/workstation/` becomes `nixosModules.workstation` |
| hardware           | one machine model, and what it needs           | `nixosModules.dell-xps-14-da14260`                      |
| layer              | what a machine repository adds to its machines | the machine repository's own modules                    |
| identity           | who Joel is on a machine                       | `jitsusama.identity.email`                              |
| theme              | the look every program Joel sees shares        | `jitsusama.theme.colors.accent`                         |
| account            | the user home-manager configures               | `home-manager.users.<account>`                          |

Modules are named after the program or subsystem they configure, never after
an abstraction, and no name carries a prefix saying what kind of thing it is:
the output and the directory already say that. "Host" appears only in
`hostName`, where NixOS requires it, and "shell" means zsh.

## The Pieces

```text
nixosSystem, in a machine repository           file it opens
├── ./machines/laptop                         the machine's own settings
└── core.nixosModules.workstation              roles/workstation/nixos.nix
    ├── imports nixosModules.base              roles/base/nixos.nix
    │   ├── imports nixosModules.home-manager  modules/home-manager/nixos.nix
    │   └── every account gets homeModules.base
    └── every account gets homeModules.workstation
                                               roles/workstation/home.nix
        ├── imports homeModules.base           roles/base/home.nix
        │   └── imports homeModules.git, ...   modules/git/home.nix
        └── imports homeModules.neovim, ...    modules/neovim/home.nix
```

- **Modules** are ordinary NixOS, nix-darwin or home-manager modules, one
  directory per tool under `modules/`. The file name says which kind each is.
- **Roles** say what a machine is for: `base` for every machine,
  `workstation` for one Joel writes code on, `graphical` for one with a screen
  in front of him, `laptop` for one he carries. A role only imports. A system
  role imports the system modules and adds its home-manager half to every
  account.
- **Hardware** is one machine model: what it needs to run well, in
  `hardware/<model>/nixos.nix`. It decides the kernel. [The hardware
  guide][5] says what each one does.
- **[`flake.nix`][1]** is the index. It names every module and role, by module
  system, and gives each one what it takes.
- **Machines** live in the machine repositories. They call the stock builders
  with core's roles, their layer and their own settings, and nothing stands in
  between.

## Module Systems

The file name inside a tool's, role's or hardware's directory says which module
system it belongs to, and which output of `flake.nix` names it:

| File         | Module system             | Named in                           |
| ------------ | ------------------------- | ---------------------------------- |
| `home.nix`   | home-manager              | `homeModules`                      |
| `darwin.nix` | nix-darwin                | `darwinModules`                    |
| `nixos.nix`  | NixOS                     | `nixosModules`                     |
| `system.nix` | both NixOS and nix-darwin | `nixosModules` and `darwinModules` |

A tool needing settings in more than one system has one file for each, side by
side: zsh has `home.nix` for the shell, `system.nix` for what either system
sets up around it, and `nixos.nix` for the login shell, which only NixOS sets.
`system.nix` exists for settings the two systems spell the same way, such as
zsh's and fonts, so they are written once.

## Roles and Their Two Halves

A machine imports a role once, as a system module. The system role imports
the system modules it needs, and gives its home-manager half to every account
through `home-manager.sharedModules`:

```nix
# roles/workstation/nixos.nix
{ nixosModules, homeModules }:
{
  imports = [ nixosModules.base ];
  home-manager.sharedModules = [ homeModules.workstation ];
}
```

Roles build on one another by importing: `workstation`, `graphical` and
`laptop` all import `base`. A machine playing several imports `base` more than
once, which counts once, because every role has a fixed identity; the next
section explains how.

## How Inputs Reach a File

Most modules need nothing beyond what the module system gives them. A file
that needs a flake input, or core's own modules as a role does, takes exactly
that, by name, as an outer function, and `flake.nix` is the only place that
gives it:

```nix
# modules/neovim/home.nix
{ neovim-pi }:
{ pkgs, lib, ... }:
{
  # ...
}
```

```nix
# flake.nix
neovim = moduleFrom ./modules/neovim/home.nix { inherit neovim-pi; };
```

`moduleFrom` is a few lines in `flake.nix`. It gives the result the identity
of its file, as a module imported by path has, so importing it twice counts
once and `disabledModules` can name it. Without that, the module system can't
tell two copies of a function's result apart, and merges both.

There is no `specialArgs` or `extraSpecialArgs`: an argument that arrives
through either can't be traced from the module that uses it.

## Tracing Anything

Every question about where something comes from is answered by search and by
opening files:

- **Where is a module used?** Search `flake.nix` for its path to find its
  name, then search `roles/` for that name.
- **What does a machine run?** Open the roles it imports, and follow each
  import to a module.
- **Where does an input go?** Search `flake.nix` for its name; every use is
  there.
- **Where is a setting made?** Search `modules/` for the option. Each option
  is set where the tool that owns it is configured.

For example, the `prune-branches` git alias on a NixOS workstation: the
machine imports `nixosModules.workstation`, which `flake.nix` defines as
`roles/workstation/nixos.nix`. That gives every account
`homeModules.workstation`, defined as `roles/workstation/home.nix`, which
imports `homeModules.base`, defined as `roles/base/home.nix`, which imports
`homeModules.git`, defined as `modules/git/home.nix`, where the alias is set.

## The Library's Interface

core.nix is used as a library, so what a machine repository may rely on is
fixed:

| A machine repository may rely on | It may not rely on                    |
| -------------------------------- | ------------------------------------- |
| the names of the outputs below   | core's file paths                     |
| the `jitsusama.*` options        | the values core gives other options   |

| Output                       | What it holds                                                   |
| ---------------------------- | --------------------------------------------------------------- |
| `homeModules`                | home-manager modules and roles                                  |
| `darwinModules`              | nix-darwin modules and roles                                    |
| `nixosModules`               | NixOS modules, roles and hardware                               |
| `packages`                   | what core builds itself: the kernel, for x86_64-linux           |
| `checks`                     | every module and role evaluated, the examples, formatting       |
| `formatter`                  | treefmt, for `nix fmt`                                          |
| `home-manager`, `nix-darwin` | what the machine repositories imported before the outputs above |
| `wallpapers`, `neovim-pi`    | inputs the machine repositories reached through core.nix        |

The last two rows exist only until no machine repository imports them;
`flake.nix` says so beside them.

| Option                             | Means                                                    |
| ---------------------------------- | -------------------------------------------------------- |
| `jitsusama.disk.device`            | the disk to install onto, set per machine                |
| `jitsusama.disk.swapSize`          | the swap file's size, at least the memory's              |
| `jitsusama.identity.name`          | the name Joel's work is attributed to                    |
| `jitsusama.identity.email`         | the email address it's attributed to, set per layer      |
| `jitsusama.kernel.cpu`             | the CPU the kernel is compiled for, set per hardware     |
| `jitsusama.kernel.profile`         | the kernel's AutoFDO profile, recorded per machine       |
| `jitsusama.keyboard.builtIn`       | the machine's own keyboard, set per hardware             |
| `jitsusama.login.account`          | the account signed in at boot, set per machine           |
| `jitsusama.signing.allowedSigners` | every key that signs Joel's work, per machine repository |
| `jitsusama.theme.colors.*`         | each colour of the theme, Omarchy's Osaka Jade           |
| `jitsusama.theme.font.*`           | the font every program draws text in, and its size       |
| `jitsusama.theme.shape.*`          | the border, corners and gaps every surface shares        |

Renaming or removing an output or an option is a breaking change. A machine
repository sees it only when it runs `nix flake update core`. [Decision
0005][4] says why the interface is modules and options rather than functions.

## What Is Deliberately Absent

- **A framework.** flake-parts and the dendritic pattern were tried and left
  out; [decision 0001][2] says why.
- **Discovery.** Nothing reads the file tree to find modules. A file that
  `flake.nix` doesn't name is not part of the library.
- **`enable` switches on modules.** Importing a module is choosing it;
  [decision 0002][3] says why.
- **Functions that build machines.** Machine repositories call `nixosSystem`
  and `darwinSystem` themselves, so every NixOS and nix-darwin document
  applies as written; [decision 0005][4] says why.

[1]: ../flake.nix
[2]: decisions/0001-plain-flake.md
[3]: decisions/0002-choosing-is-importing.md
[4]: decisions/0005-modules-and-options-are-the-interface.md
[5]: hardware.md
