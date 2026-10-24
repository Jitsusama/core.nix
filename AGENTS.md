# AGENTS.md

core.nix is the shared library behind Joel's machines: NixOS, nix-darwin and
home-manager modules, and the roles that group them. Machines live in the
repositories that build on it, which core.nix never names. It is plain Nix:
no framework, and nothing found by scanning the file tree.

## Map

| Path                | Holds                                                     |
| ------------------- | --------------------------------------------------------- |
| `flake.nix`         | the index: every module, role and input, by module system |
| `modules/<tool>/`   | one tool: a module per system, its config files, checks   |
| `roles/<role>/`     | what a machine is for: imports only, one file per system  |
| `hardware/<model>/` | one machine model's needs, and the kernel it runs         |
| `examples/`         | machines built only from the outputs, as a consumer would |
| `tests/`            | VM tests, the shell's QML cases, check scripts, stand-ins, dev tools |
| `checks.nix`        | what `nix flake check` runs: every folder's `checks.nix`  |
| `check-descriptions.nix` | what each check proves, by its name                  |
| `treefmt.nix`       | what `nix fmt` runs                                       |
| `docs/`             | architecture, conventions, guides and decision records    |
| `.agents/skills/`   | how to do this repository's recurring jobs, step by step  |

A file's name says its module system: `home.nix` (home-manager), `darwin.nix`
(nix-darwin), `nixos.nix` (NixOS) or `system.nix` (both NixOS and
nix-darwin). To learn how any file fits, search `flake.nix` for its path. The
words machine, role, module, hardware, layer, identity and account have one
meaning each; [docs/architecture.md][2] defines them.

## Rules

- Every connection must be followable by opening files. Name each new module,
  role or hardware in `flake.nix`, in alphabetical order within its group.
  Never discover files, look modules up by string, or use `specialArgs`.
- A file that needs a flake input, or core's own modules, takes them by name
  as an outer function, `{ wallpapers }: { pkgs, ... }: { ... }`, and
  `flake.nix` gives them through `moduleFrom`, so the result can be imported
  twice and named in `disabledModules`.
- A module configures one tool. A role only imports, and a system role adds
  its home-manager half with `home-manager.sharedModules`.
- No `enable` options on modules: importing is choosing. Options are only for
  settings that differ between machines, under `jitsusama.<name>`.
- The outputs and the `jitsusama.*` options are the public interface. Never
  add a function that builds a machine; machine repositories call
  `nixosSystem` and `darwinSystem` themselves.
- Never rely on the order modules are imported in. When order matters, say so
  with `lib.mkBefore`, `lib.mkAfter` or `lib.mkOrder`.
- Keep an application's config in its own format beside its module and read
  it in; don't translate it into Nix. A script past a few lines, a VM test's
  included, is a file in its own language too, linted and tested on its own.
- Never declare a setting another tool writes, and never put a secret or
  application state in a Nix value.
- Name nothing that builds on core.nix, anywhere in it: no machine, no machine
  repository, no employer. They name core.nix instead. `stands-alone` fails on
  it.
- Anything that puts something on a screen, or that Joel types or presses,
  follows [docs/design.md][9] and isn't done until it meets its Quality Bar:
  load the `aesthetic` skill ([modules/theme/SKILL.md][10]) and read the
  principles before designing it, and judge it by them rather than asking
  Joel to look. Surfaces name the
  theme's roles and faces, never a colour or a font, and a new program goes
  through its checklist. A theme from elsewhere is
  translated with the `translate-a-theme` skill.
- Comments say why, in full sentences. A file opens with a comment saying
  what it is for when its path alone doesn't.
- Lines stop at 100 characters; `nix fmt` decides the rest of the layout.

## Commands

```sh
nix fmt                                # format, and lint every language strictly
nix build .#checks.x86_64-linux.<name> # the checks a change touches, as you go
run-subtests vm-desktop-<area> <words> # a VM test's subtests named, in seconds
shell-preview --type <words>           # the shell from the working copy, in a niri window
changed-checks --build --no-vm         # every check the change touched, VM tests aside
nix flake check --keep-going           # everything, once, before pushing
```

Prove a change with the cheapest check that shows it while editing, and run
everything once, before pushing, never after each edit:

| Changed         | While editing, in seconds                         | Before pushing             |
| --------------- | ------------------------------------------------- | -------------------------- |
| Shell logic     | its own check, `quickshell-passes-qmllint`        | the VM tests it changes    |
| A shell surface | `shell-preview`, `quickshell-passes-qmllint`, `run-subtests` | its `vm-desktop-<area>`, `shots-changed` |
| A Rust crate    | `cargo test`, then `<crate>-passes-clippy`        | its VM test, if any        |
| A theme         | `a-faint-theme-is-refused`, `a-theme-changes-everything` | the VM tests it changes |
| The kernel      | `kernel-settings-hold-<model>`                    | the machine's kernel       |
| A module        | `example-nixos`, `example-laptop`, `example-darwin` |                          |
| Any script      | `nix fmt`, its own check                          |                            |

`changed-checks` names each check whose derivation the change moved from its
merge base with origin's main, and `nix build --dry-run` on a check says what
it would build, so a change that reaches a VM test is seen before it's paid
for. Watching a check fail is done
the same narrow way, never by copying the tree and running everything.

A change is done when `nix flake check` passes locally, and its machine builds
with core pointed at the checkout. There's no CI, and nothing runs on a
schedule. A refactor is done when the machine repositories' machines evaluate
to the same derivations as before. A new check has to earn its place. All of
it is in [docs/testing.md][1].

## Commits

Conventional Commits, scoped to the module, role or document touched:
`feat(kitty): ...`, `fix(zsh): ...`, `docs(architecture): ...`. Subjects are
imperative and at most 50 characters; bodies say what and why, wrapped at 72.
Renaming or removing an output or a `jitsusama.*` option is breaking: mark it
`!`.

## More

- [docs/architecture.md][2]: the vocabulary, how the pieces connect, how to
  trace them
- [docs/conventions.md][3]: the rules, with their reasons
- [docs/modules.md][4]: adding a module or a role, and changing one from a
  machine repository
- [docs/machines.md][5]: building a machine from core.nix
- [docs/kernel.md][7]: the kernel, its patches and settings, and changing
  them
- [docs/hardware.md][8]: each machine model, where its workarounds come from,
  and what's left
- [modules/theme/SKILL.md][10]: Joel's aesthetic, and how to judge a design
  by it
- [docs/design.md][9]: how everything on a screen looks, reads and behaves
- [docs/decisions/][6]: why the design is what it is

[1]: docs/testing.md
[2]: docs/architecture.md
[3]: docs/conventions.md
[4]: docs/modules.md
[5]: docs/machines.md
[6]: docs/decisions/
[7]: docs/kernel.md
[8]: docs/hardware.md
[9]: docs/design.md
[10]: modules/theme/SKILL.md
