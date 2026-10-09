# Modules

How to add a tool to the machines, or change which machines get it.

## Where It Belongs

A module belongs here when any machine built on core.nix could use it.
Anything only some machines want, such as one person's apps or an employer's
tools, belongs in the machine repository those machines live in. The
`stands-alone` check holds core.nix to that: `nix flake check` fails when any
file here names a machine, a machine repository or an employer.

## Adding a Module

1. Make the tool's directory and write an ordinary module, named for the
   system it configures. Most tools only need home-manager:

   ```nix
   # modules/jq/home.nix
   { programs.jq.enable = true; }
   ```

2. Name it in `flake.nix`, keeping the list in alphabetical order:

   ```nix
   homeModules = {
     # ...
     jq = ./modules/jq/home.nix;
     # ...
   };
   ```

3. Import it from the role for the machines that should have it:

   ```nix
   # roles/base/home.nix
   imports = [
     # ...
     homeModules.jq
   ];
   ```

4. Run `nix fmt` and `nix flake check`. The checks pick the new module up
   from `flake.nix`, so it is evaluated on its own without anything more.

Keep the tool's configuration in its own format beside the module, such as
`modules/glow/glow.yml`, and read it in rather than translating it to Nix.

## A Module That Needs an Input

Take the input by name as an outer function, and give it in `flake.nix`
through `moduleFrom`, which keeps the result importable twice and nameable in
`disabledModules`:

```nix
# modules/wezterm/home.nix
{ wallpapers }:
{ pkgs, ... }:
{
  # ...
}
```

```nix
# flake.nix
wezterm = moduleFrom ./modules/wezterm/home.nix { inherit wallpapers; };
```

Name the input in the arguments of `outputs` if it isn't there already.

## A Tool on More Than One System

Write one file per system in the same directory, and name each in its own
output. zsh, for example:

```nix
homeModules.zsh = ./modules/zsh/home.nix;
darwinModules.zsh = ./modules/zsh/darwin.nix;
```

When the setting is spelled the same on NixOS and nix-darwin, as fonts are,
write it once as `system.nix` and name that one file in both `nixosModules`
and `darwinModules`.

## Adding a Role

A role says what a machine is for, and only imports. Its home-manager file
takes the modules it draws from:

```nix
# roles/<role>/home.nix
# What a machine playing this role is for, in one sentence.
{ homeModules }:
{
  imports = [
    homeModules.base
    homeModules.neovim
  ];
}
```

Its system files import the system modules, build on `base`, and give every
account the home-manager half:

```nix
# roles/<role>/nixos.nix
{ nixosModules, homeModules }:
{
  imports = [ nixosModules.base ];
  home-manager.sharedModules = [ homeModules.<role> ];
}
```

Write only the files that have something in them, and name each in
`flake.nix` after the modules of its module system:

```nix
<role> = moduleFrom ./roles/<role>/nixos.nix { inherit (self) nixosModules homeModules; };
```

## Giving a Module Options

When a module needs a setting that differs between machines, declare it under
`jitsusama.<name>`, with a type, a description and a default where one makes
sense. Options several modules read get a module of their own that the
readers import by path, as `modules/identity/home.nix` is imported by git. A
module never gets an `enable` option of its own: the machines that shouldn't
have it don't import it.

## Changing a core.nix Module from a Machine Repository

[Machines][1] lists the ways, from setting an option to removing a module.
When a change suits every machine, it belongs in core.nix instead.

[1]: machines.md#changing-what-core-sets
