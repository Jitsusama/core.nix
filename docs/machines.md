# Machines

Machines live in the machine repositories that build on core.nix, never in
core.nix itself. This is how one is defined, changed and applied.
[`examples/`][1] has a whole machine of each kind, evaluated by
`nix flake check`.

## Defining One

A machine is a call to the stock builder in its repository's `flake.nix`,
with the roles it plays, the repository's own modules and a directory of the
machine's own settings:

```nix
nixosConfigurations.laptop = nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    self.nixosModules.personal
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

## The Repository's Own Modules

A machine repository's own modules are what it adds to every machine of its
own, such as an identity or the apps no role brings. They're imported beside
core's roles, named for the machines they're for (`personal` here), and they
set core's options rather than repeating what core does:

```nix
# personal/home.nix
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

A machine or its repository's own modules change what core sets with the
module system alone, in this order of preference:

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
as lines, so a machine adds its own after core's with `lib.mkAfter`. A
screen's settings are the exception: niri reads only the first `output` block
that names a screen, so a machine sets them in `jitsusama.niri.outputs`, where
they join core's in one block, and a second block for a screen is refused:

```nix
home-manager.users.jitsusama.jitsusama.niri.outputs."eDP-1" = [
  "scale 2"
  "position x=560 y=1440"
];
```

## Applying It

```sh
nix flake check --keep-going
nixos-rebuild build --flake .#laptop
nix store diff-closures /run/current-system ./result
sudo nixos-rebuild switch --flake .#laptop
```

On a Mac, `darwin-rebuild` takes the same arguments, with `sudo` only for the
switch. home-manager is part of the system, so one switch applies the
machine and the account together.

On a NixOS workstation, an account in `wheel` can switch to a system it built
without a password: `switch-system ./result` after `nixos-rebuild build`.

A machine that imports `update-to-main` updates itself instead, with nobody
at it: one of its `jitsusama.update-to-main.askers` runs
`update-to-main ask`, and root fetches the repository's main, builds the
system only if a key in `signers` signed it and it descends from what the
machine runs, switches, and goes back if the machine isn't whole after. The
ask takes no arguments, so whoever asks can't choose what runs. How the
last update ended is kept as its line, for the askers to read, in
`/run/update-to-main/last.json` on NixOS and
`/var/run/update-to-main.last.json` on a Mac, so it's there though the
switch restarted whoever asked:

```nix
jitsusama.update-to-main = {
  repository = "https://github.com/someone/machines";
  output = "nixosConfigurations.laptop.config.system.build.toplevel";
  signers.openpgp = [ ./forge-merges.asc ];
  askers = [ "someone" ];
  health = [ ];
};
```

A repository whose host signs nothing is followed by what the host names
instead. With `hosted` set, root asks the host for main over TLS with a
token the account's `token` command prints, reads that commit's objects from
the account's checkout, which git checks against it, and builds the flake in
`directory`. The account fetches main into its checkout when the commit
isn't there yet, and fetches each of `inputs`, the private ones, by its
lock. The machine then trusts the host and takes no signers:

```nix
jitsusama.update-to-main = {
  repository = "https://forge.example/someone/everything.git";
  output = "nixosConfigurations.laptop.config.system.build.toplevel";
  hosted = {
    user = "someone";
    checkout = "/home/someone/everything";
    directory = "machines";
    token = [ "/home/someone/bin/forge-token" ];
    inputs = [ "private-modules" ];
  };
  askers = [ "someone" ];
  health = [ ];
};
```

An input the machine should run at its own main, rather than wait for the
flake's lock to move, is named in `follows` with its repository and
signers. Root fetches that main, checks its signature and that it descends
from the lock's commit and from what it last followed, and builds with the
input overridden to it; an ask with main where it was builds again when a
followed main moved. With `elsewhere = true`, the machine builds only the
pieces nixpkgs marks light itself and sends every other to the Nix
daemon's builders, failing rather than build one when none takes it.

Code that may go to some builders and not others is named in `kept`, by
patterns over its pieces' names. Those pieces go only to `kept.builders`,
and when it names none an update needing one fails. A piece built from a
kept one, such as a wrapper, a unit or the system itself, would carry it
to whichever builder took it, so it's built here, where the kept one now
is, with no builders at all:

```nix
jitsusama.update-to-main = {
  follows.core = {
    repository = "https://github.com/someone/core";
    signers.openpgp = [ ./forge-merges.asc ];
  };
  elsewhere = true;
  kept = {
    names = [ "secret-tool" "secret-service*" ];
    builders = "@/run/kept-builders";
  };
};
```

## Formatting and Linting

A machine repository formats and lints its code as core does by giving
treefmt-nix core's linters beside its own `treefmt.nix`, which adds only what
it alone has:

```nix
# flake.nix
treefmt =
  pkgs:
  treefmt-nix.lib.evalModule pkgs {
    imports = [
      core.treefmtModules.treefmt
      ./treefmt.nix
    ];
  };
```

Each linter reads core's own settings, so `nix fmt` there holds code to
exactly what it holds core's to. Lua is read as the program that runs it does,
which the repository says in a `.luacheckrc` of its own. A VM test's script,
kept in a `*-vm-test.py` file, imports the test driver's names from
`driver_names` under `if TYPE_CHECKING:`, as [testing.md][3] says, and
mypy reads them and the driver's own types from core.

Its Rust meets the same clippy lints as core's crates, through a check it
names in its own checks:

```nix
my-tool-passes-clippy = (core.lib.clippy pkgs).passes {
  name = "my-tool-passes-clippy";
  package = pkgs.callPackage ./modules/my-tool/package.nix { };
};
```

clippy reads core's `clippy.toml` unless `settings` names a folder holding
the repository's own, for names of things its docs use that core's don't.

Its docs name only what exists, as core's do, through one more:

```nix
docs-name-what-exists = core.lib.docsNameWhatExists pkgs {
  src = ./.;
  # Each mention of something deliberately absent, as `file path`, with why.
  exempt = [ ];
};
```

It fails on a relative link that resolves to nothing, and on a backticked
path starting with one of the repository's own top folders that names
nothing, read from the root or the doc's folder. Code blocks aren't read.

Every folder says what it holds, as core's do, through another:

```nix
every-folder-says-what-it-holds = core.lib.everyFolderSaysWhatItHolds pkgs {
  src = ./.;
  # Each folder still waiting for its note, with why.
  waiting = [ ];
};
```

A folder says what it holds with a README.md or AGENTS.md, an entry file
such as `home.nix` or `Cargo.toml` whose first line is a comment, or a line
of its parent's README opening with its name. A top folder may instead have
a line in the README's map, its name and then a comment, as
`machines/   # one folder per machine`. A waiting folder that has its note,
or is gone, fails too, so the list only shrinks.

Its names say the glossary's words, as core's do, through a check that reads
the glossary of the core it locks:

```nix
no-name-says-a-replaced-word = core.lib.namesSayTheGlossary pkgs {
  names = [ "my-tool-passes-clippy" ];
  otherSenses."vm-apps-come-first" = "app is systemd's app.slice, by its name";
  waiting."the-line-stays-quick" = "Repos: line to connection";
};
```

`names` is every check's and option's name. A name kept as another sense or
as a waiting rename says why, and one no longer found fails, so what's kept
only shrinks. The [glossary][4] says what each word means.

## Taking New Pins

The machine repository follows core.nix's pins, so updating is one input:

```sh
nix flake update core
```

[1]: ../examples/
[2]: hardware.md
[3]: testing.md
[4]: glossary.md
