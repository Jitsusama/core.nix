# Glossary

What every thing on Joel's machines and in their tooling is called: one word
for each thing, and one thing for each word, used the same way in code,
directories, docs, on screen and in what agents say. [The design][design] says
how things look and behave; this says what they're called. A word that isn't
here yet hasn't been settled, and a word under [Still to Settle](#still-to-settle)
means two things today.

Each table has the same columns:

- **Word**: the thing's name, as it's written in prose and shown on screen.
- **Means**: what it is, in a few plain words.
- **Also**: other words that find it, as when typed into the launcher, but are
  never written as its name.
- **Not**: names it replaces, which are never written for it.
- **In code**: how code spells it, where code has to spell it otherwise, such
  as a program's name for a module or NixOS's own option.

The-glossary-names-each-thing-once holds every table to that: no word is the
Word, an Also or a Not of two things.

## The Library

The words core.nix is built from.

| Word               | Means                                          | Also | Not  | In code                                                 |
| ------------------ | ---------------------------------------------- | ---- | ---- | ------------------------------------------------------- |
| machine            | one computer                                   |      | host | `nixosConfigurations.laptop`; `hostName`, which NixOS requires |
| machine repository | a repository of machines built on core.nix     |      |      | any flake with core.nix as an input                     |
| module             | one program or subsystem, configured           |      |      | `modules/bat/` becomes `homeModules.bat`                |
| role               | what a machine is for                          |      |      | `roles/workstation/` becomes `nixosModules.workstation` |
| hardware           | one machine model, and what it needs           |      |      | `nixosModules.dell-xps-14-da14260`                      |
| layer              | what a machine repository adds to its machines |      |      | the machine repository's own modules                    |
| identity           | who Joel is on a machine                       |      |      | `jitsusama.identity.email`                              |
| theme              | the look every program Joel sees shares        |      |      | `jitsusama.theme.roles.accent`                          |
| account            | the user home-manager configures               |      |      | `home-manager.users.<account>`                          |

## Still to Settle

Words that mean two things today. Each waits on one choice, and moves into a
table above once it's made.

- **shell**: zsh, as this glossary used to say, and the desktop's own windows
  (the launcher, notices and the margin), as most of the code and [the
  design][design] say.
- **layer**: what a machine repository adds, and a keyboard's layer.
- **role**: what a machine is for, and what a colour means (`jitsusama.theme.roles`).
- **identity**: who Joel is, and an age identity, the file that opens what was
  sealed to it.
- **module**: one of core's modules, a kernel module, and the camera module.
- **machine**: one computer here, and `Computer` in the launcher.

[design]: design.md
