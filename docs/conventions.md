# Conventions

The rules core.nix is written to. The machine repositories follow the same ones,
so code reads the same wherever it lives.

## Every Connection Can Be Followed

A reader who opens any file must be able to find out how it fits by opening
other files. So:

- `flake.nix` names every module and role, and nothing is found by reading
  the file tree.
- A file that needs a flake input, or core's own modules, takes them by name
  as an outer function, and `flake.nix` is the only place that gives them,
  through `moduleFrom`. There is no `specialArgs`.
- Modules are referred to by path or by a name `flake.nix` defines, never by
  a string looked up somewhere else.
- No framework, helper library or wrapper stands between a machine and the
  stock builders.

## Names

- One word per concept, used the same way in code, directories and docs. The
  [vocabulary][3] lists them.
- A module is named after the program or subsystem it configures (`kitty`,
  `zsh`, `fonts`), never after an abstraction.
- A role is named after what a machine is for: `base`, `workstation`,
  `graphical`, `laptop`.
- No prefixes. The output and the directory already say what kind of thing a
  name is, and two names in `flake.nix` can't collide.

## A Directory per Tool, a File per Module System

Each tool has one directory under `modules/`, holding a module for each
system it configures and its own configuration files. The file name says
which system: `home.nix` for home-manager, `darwin.nix` for nix-darwin,
`nixos.nix` for NixOS, and `system.nix` for one module that works on both
NixOS and nix-darwin. Roles follow the same naming under `roles/`.

A module configures one tool and nothing else, apart from the small hooks
that tool needs in another, such as a zsh plugin or a git ignore pattern.

## Choosing Is Importing

A machine gets a tool by importing its module, usually through a role.
Modules have no `enable` switch of their own: an `imports` line is the
choice, and leaving the line out is how a machine goes without.

Options are declared only for settings that differ between machines, such as
identity, under `jitsusama.*`. They are the extension points a machine
repository is meant to set. A machine repository's own options live under a
namespace of its own.

## Roles Only Import

A role is a module whose body is an `imports` list, plus, for a system role,
the `home-manager.sharedModules` that give every account its home-manager
half. Settings belong in a module, so that every setting has exactly one home
and a role can be read at a glance. A role builds on a smaller one by
importing it: `workstation` imports `base`.

## Order Is Never Assumed

The module system merges lists in an order that depends on how modules are
nested, so moving a module into a role, or a role into another, can reorder a
list without changing anything else. No module may rely on that order. When
order matters, for start-up code that needs another tool on the path first,
say, the module says so with `lib.mkBefore`, `lib.mkAfter` or `lib.mkOrder`.

## Config Files Stay Files

An application's configuration stays in its own format beside its module
(`init.lua`, `config.kdl`, `glow.yml`), read in with `builtins.readFile` or
linked as a source. Translating it into Nix attribute sets hides it from the
application's documentation and from its own validators.

## Applications That Write Their Own Settings

Some applications change their own configuration, and some share a file with
other tools. Five rules keep both working:

1. A file nothing else writes is linked as a plain file.
2. JSON, TOML or YAML that something else also writes is merged in on each
   switch, through the module's `mutableSettings` or home-manager's impure
   config merger, and stays writable.
3. Any other format uses the application's own include, so Nix owns one file
   and the application owns the rest.
4. Link the entries of a folder, never the folder, so the application can add
   files beside them.
5. Secrets and application state are never Nix values: anything in a Nix value
   ends up readable in the Nix store.

A key another tool owns is never declared here at all.

## One Look Everywhere

Everything Joel sees draws itself from `jitsusama.theme`: its colours, its
font and its shape. A module writes those into its program's own format, so
the terminal, the compositor's borders, a PIN prompt, a notification and the
lock screen all match, and changing one option changes them all. A program
that can't take the theme's values doesn't get on a screen.

One program draws the desktop: Quickshell, in `modules/quickshell/`. Its
launcher on Super+Space, notifications, lock screen, polkit prompt, volume and
brightness bars, and the status in the overview's backdrop all read `Theme.qml`,
which `home.nix` renders from the theme. A prompt that has to be a program of
its own, such as the PIN prompt ssh-tpm-agent starts, is bemenu, which
`modules/bemenu/home.nix` styles the same way. GTK programs, the file chooser
among them, take the theme's colours from a stylesheet `modules/gtk/home.nix`
writes, and `modules/fontconfig/home.nix` makes the theme's font the one any
program gets when it asks for monospace.

## Projects Bring Their Own Toolchains

A project says which compiler it builds with, in its flake, its
`rust-toolchain.toml` or its `go.mod`, so it builds the same on every machine
and in CI. core installs no compiler or language runtime for projects to
lean on. It may set defaults that make every project's builds faster, such as
mold as Rust's linker, as long as a project's own settings still win. A tool
is installed system-wide only when it's worth having outside any project.

## Nix Is Code

- Code reads top-down in domain language, and vertical space is earned: no
  intermediate variable that only renames an expression.
- Comments say why, in complete sentences. A comment above a file says what
  it is for.
- `nix fmt` decides layout. Lines stop at 100 characters.
- deadnix and statix run with the formatter, so unused bindings and their
  usual mistakes never land.
- A check has to earn its place in the loop; see [Testing][1].

## Checks Before Switches

Nothing is applied to a machine until `nix flake check` passes locally. CI
runs the same checks as a backstop, and nothing waits on it. Before switching,
compare what changes:

```sh
nix store diff-closures /run/current-system ./result
```

## Native Commands Only

Machines are built and applied with `nixos-rebuild`, `darwin-rebuild` and `nix`.
There is no wrapper script, so every Nix document applies as written.

## Commits

Commits follow [Conventional Commits][2]. The scope is the module, role or
document touched: `feat(kitty):`, `fix(zsh):`, `refactor(workstation):`,
`docs(architecture):`. Renaming or removing an output or a `jitsusama.*`
option breaks the machines that use it, so it is marked with `!`.

[1]: testing.md
[2]: https://www.conventionalcommits.org
[3]: architecture.md#vocabulary
