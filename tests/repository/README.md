# What the Whole Repository Is Held To

The checks that read core as a repository rather than as a machine: its docs,
its folders, its names and its path patterns. Several are also given to other
repositories built on core, through `core.lib` in `flake.nix`.

| File                                       | Holds                                        |
| ------------------------------------------ | -------------------------------------------- |
| `docs-name-what-exists.{nix,sh}`           | each path the docs name exists               |
| `every-folder-says-what-it-holds.{nix,sh}` | each folder has a note saying what it holds  |
| `names.{nix,py}`                           | no name says a word the glossary replaces    |
| `names-kept.nix`                           | the names that check keeps, each with why    |
| `glossary.py`                              | the glossary gives each thing one word       |
| `every-vm-test-keeps-to-the-pyramid.py`    | no VM test grows past twenty subtests        |
| `every-pattern-names-a-file.py`            | every tool's path pattern still names a file |
| `nothing-names-its-own-look.sh`            | nothing but the theme names a colour or face |
| `nothing-spells-a-mark.sh`                 | nothing but the theme spells a mark          |
| `the-desktop-speaks-its-javascript.sh`     | the shell calls no method its engine lacks   |
| `checks.nix`                               | each of these, run on core itself            |

The checks that hold every file to a rule, with a shrinking list of the files
still waiting on it, live in `tests/rules/`.
