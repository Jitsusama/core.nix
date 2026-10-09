# Machines

Machines live in the machine repositories that build on core.nix, never in
core.nix itself. This is how one is defined, changed and applied.
[`examples/`][1] has a whole machine of each kind, evaluated by
`nix flake check`.

## Defining One

A machine is a call to the stock builder in its repository's `flake.nix`,
with the roles it plays, the repository's layer and a directory of the
machine's own settings:

```nix
nixosConfigurations.laptop = nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    self.nixosModules.layer
    ./machines/laptop
  ];
};
```

Each system role brings home-manager in and gives every account its
home-manager half, so the account needs no imports of its own. A machine
whose model core.nix knows also imports that hardware, such as
`core.nixosModules.dell-xps-14-da14260`; [the hardware guide][2] says what
each brings. A Mac is the same with `nix-darwin.lib.darwinSystem` and
`core.darwinModules`.

## The Layer

The layer is what a machine repository adds to every machine of its own,
such as an identity or the apps no role brings. It's a set of modules in the
machine repository, imported beside core's roles, and it sets core's options
rather than repeating what core does:

```nix
# layer/home.nix
{
  jitsusama.identity.email = "joel@example.com";
}
```

## The Machine's Own Directory

It holds what belongs to that machine alone:

```nix
# machines/laptop/default.nix
{
  imports = [ ./disks.nix ];

  networking.hostName = "laptop";
  nixpkgs.hostPlatform = "x86_64-linux";
  users.users.jitsusama.isNormalUser = true;
  system.stateVersion = "25.05";
  home-manager.users.jitsusama.home.stateVersion = "25.05";
}
```

Keep it short. Anything another machine could use becomes a module instead,
and a `README.md` beside it records the hardware, where things live, and what
to check after installing.

## Changing What core Sets

A machine or a layer changes what core sets with the module system alone, in
this order of preference:

| To                                    | Write                                         |
| ------------------------------------- | --------------------------------------------- |
| Add something                         | Just set it; lists and attribute sets merge   |
| Change a setting core expects to vary | Set its `jitsusama.*` option                  |
| Change any other value core set       | `lib.mkForce` on that one value               |
| Remove a module a role brought in     | `disabledModules = [ core.homeModules.bat ];` |
| Go without a role                     | Don't import it; import the modules you want  |

When more than one machine has to `lib.mkForce` the same value, core gets an
option for it instead.

A configuration file core writes as text, such as niri's `config.kdl`, merges
as lines, so a machine adds its own after core's with `lib.mkAfter`. niri lets
a later setting override an earlier one, so a machine can set its panel's
scale:

```nix
home-manager.users.jitsusama.xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
  output "eDP-1" {
      scale 2
  }
'';
```

## Applying It

```sh
nix flake check
nixos-rebuild build --flake .#laptop
nix store diff-closures /run/current-system ./result
sudo nixos-rebuild switch --flake .#laptop
```

On a Mac, `darwin-rebuild` takes the same arguments, with `sudo` only for the
switch. home-manager is part of the system, so one switch applies the
machine and the account together.

## Taking New Pins

The machine repository follows core.nix's pins, so updating is one input:

```sh
nix flake update core
```

[1]: ../examples/
[2]: hardware.md
