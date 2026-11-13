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
| `tests/`            | VM tests, the shell's QML cases, check scripts, stand-ins |
| `tools/`            | the dev shell's own programs, for changing core.nix       |
| `checks.nix`        | what `nix flake check` runs: every folder's `checks.nix`  |
| `check-descriptions.nix` | what each check proves, by its name                  |
| `treefmt.nix`       | what `nix fmt` runs                                       |
| `docs/`             | architecture, conventions, guides and decision records    |
| `.agents/skills/`   | how to do this repository's recurring jobs, step by step  |

A file's name says its module system: `home.nix` (home-manager), `darwin.nix`
(nix-darwin), `nixos.nix` (NixOS) or `system.nix` (both NixOS and
nix-darwin). To learn how any file fits, search `flake.nix` for its path. The
words machine, role, module, hardware, identity and account have one
meaning each; [docs/glossary.md][15] defines them, and every other thing's
name. A name for something new comes from it, or is added to it.

## Where a Case Is Proven

Checks follow [The Pyramid][16] in docs/testing.md, and every case goes in
its lowest tier that can show it, as [Earning a Check][17] says. Read both
before adding a test.

- Logic is proven in Qt's own engine or offscreen Quickshell, over cases in
  `tests/`, in under a second each ([The Desktop's Logic][18]). A view that
  works something out moves it into a `.js` beside it, where they reach it.
- What the desktop does on niri that needs no system service, a surface
  landing as niri's layer or what a bind runs, is proven on a nested niri in
  the build sandbox, in seconds ([The Desktop on niri][19]).
- A VM test proves wiring alone: one path through each connection, a unit
  starting with its config or a key reaching niri's binds
  ([Virtual Machines][20]). Its many cases go in the tiers above.
- A VM test past about two minutes or twenty subtests splits by area.
  `every-vm-test-keeps-to-the-pyramid` fails on one that grows past twenty,
  and the few past it today may only shrink.
- A check that repeats another's signal goes.

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
  through its checklist. A website, any page or app in a browser, is built
  to the `web-design` skill too ([modules/theme/web/skill/SKILL.md][22]),
  on the theme's web kit. A theme from elsewhere is
  translated with the `translate-a-theme` skill.
- Comments say why, in full sentences. A file opens with a comment saying
  what it is for when its path alone doesn't.
- Lines stop at 100 characters; `nix fmt` decides the rest of the layout.

## Commands

```sh
nix fmt                                # format, and lint every language strictly
nix-offload there --here-when-idle . checks.x86_64-linux.<name>  # a check, as you go
run-subtests vm-desktop-<area> <words> # a VM test's subtests named, in seconds
shell-preview --type <words>           # the shell from the working copy, in a niri window
changed-checks                         # names and builds what the change touched, VM tests aside
merge-net needs                        # whether the change runs VM tests before merging
nix flake check --keep-going           # everything, only for a change every check reads
```

A check built as you go goes to a builder, with the working tree's changes,
and builds here only while this machine is idle; never `nix build` it here,
where each evaluates the flake beside Joel's work.

Prove a change with the cheapest check that shows it while editing, and run
everything once, before pushing, never after each edit:

| Changed         | While editing, in seconds                         | Before pushing             |
| --------------- | ------------------------------------------------- | -------------------------- |
| Shell logic     | its case in Qt's engine or offscreen, `quickshell-passes-qmllint` | `changed-checks`, `run-subtests` on the subtests that reach it |
| A shell surface | `shell-preview`, its nested-niri check, `run-subtests` | `run-subtests --there` on its `vm-desktop-<area>`, `shots-changed` on its gallery |
| A Rust crate    | `cargo test`, then `<crate>-passes-clippy`        | its VM test's subtests, if any |
| A theme         | `a-faint-theme-is-refused`, `a-theme-changes-everything` | the galleries it changes, `shots-changed` |
| The kernel      | `kernel-settings-hold-<model>`                    | the machine's kernel       |
| A module        | `example-nixos`, `example-laptop`, `example-apple-laptop`, `example-darwin` | |
| Any script      | `nix fmt`, its own check                          |                            |

`changed-checks` names each check whose derivation the change moved from its
merge base with origin's main, and `nix build --dry-run` on a check says what
it would build, so a change that reaches a VM test is seen before it's paid
for. Watching a check fail is done
the same narrow way, never by copying the tree and running everything.

A change is done when the checks it touched pass locally: those
`changed-checks` names, VM tests aside; the subtests of each VM test that owns
a file it touches, which `merge-net needs` names, with `run-subtests --there`;
and the VM tests
`merge-net needs` names, whole. Its machine builds with core pointed at the
checkout too. The net runs the other VM tests after the merge and reverts a
merge that breaks one. Only a change every check reads, such as `flake.lock`,
runs `nix flake check`: a whole run evaluates every check, about 15 minutes
and over 20 GB on a machine other work shares. There's no CI, and nothing runs
on a schedule but the net, which follows main ([docs/testing.md][1]). A
refactor is done when the machine repositories' machines evaluate to the same
derivations as before. A new check has to earn its place. All of
it is in [docs/testing.md][1]. How to spend the shared builders, and the
targets that keep the cycle fast, are in [docs/delivery.md][21]: read it
before sending a VM run. Every session holds those targets: a merge on the
machine in a median 18 minutes, a non-boot change merged 20 minutes after
its last commit, a VM run waiting under 2 for a builder, and a broken merge
caught within the hour.

## Commits

Conventional Commits, scoped to the module, role or document touched:
`feat(kitty): ...`, `fix(zsh): ...`, `docs(architecture): ...`. Subjects are
imperative and at most 50 characters; bodies say what and why, wrapped at 72.
Renaming or removing an output or a `jitsusama.*` option is breaking: mark it
`!`.

## More

- [docs/architecture.md][2]: how the pieces connect, how to trace them
- [docs/glossary.md][15]: what every thing is called, once
- [docs/conventions.md][3]: the rules, with their reasons
- [docs/modules.md][4]: adding a module or a role, and changing one from a
  machine repository
- [docs/daemons.md][25]: the shape every daemon takes, its socket, names and
  interface
- [docs/commands.md][26]: the shape every command takes, for Joel in a
  terminal and for a machine through a pipe
- [docs/machines.md][5]: building a machine from core.nix
- [docs/kernel.md][7]: the kernel, its patches and settings, and changing
  them
- [docs/hardware.md][8]: each machine model, where its workarounds come from,
  and what's left
- [modules/theme/SKILL.md][10]: Joel's aesthetic, and how to judge a design
  by it
- [modules/theme/web/skill/SKILL.md][22]: how a website is built to it, and
  the kit the theme writes for one
- [docs/design.md][9]: how everything on a screen looks, reads and behaves
- [docs/decisions/][6]: why the design is what it is
- The six modules that are programs of their own keep a guide beside their
  code, read before changing them: [the disk guard][11], [the eero][23],
  [the guide][12], [the shell][13], [spend-watch][14] and [WARP][24]

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
[11]: modules/disk-guard/AGENTS.md
[12]: modules/guide/AGENTS.md
[13]: modules/quickshell/AGENTS.md
[14]: modules/spend-watch/AGENTS.md
[15]: docs/glossary.md
[16]: docs/testing.md#the-pyramid
[17]: docs/testing.md#earning-a-check
[18]: docs/testing.md#the-desktops-logic
[19]: docs/testing.md#the-desktop-on-niri
[20]: docs/testing.md#virtual-machines
[21]: docs/delivery.md
[22]: modules/theme/web/skill/SKILL.md
[23]: modules/eero/AGENTS.md
[24]: modules/warp/AGENTS.md
[25]: docs/daemons.md
[26]: docs/commands.md
