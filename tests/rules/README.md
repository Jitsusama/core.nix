# The Rules Held Over Every File

The checks that hold every file in the repository to one rule of the quality
standard. Each is three files: `<rule>.nix` builds the check, `<rule>.py`,
beside it, says what breaks the rule, and `<rule>-waiting.nix` lists the files
that broke it before the rule came in. A list only shrinks: fixing a file takes
it off, and nothing new goes on. `every-lock-agrees-with-its-manifests` and
`every-ruff-exception-hides-a-finding` have no list, since nothing broke them
when they came in. `checks.nix` runs all twelve on core itself.
`every-ruff-exception-hides-a-finding` keeps its planted tree's config in
`every-ruff-exception-hides-a-finding-planted.toml` and a config it must refuse
in `every-ruff-exception-hides-a-finding-refused.toml`, and first sees itself
fail on both.
`daemons-agree` reads an evaluated account's units rather than the files
alone, so it's given a graphical machine's account too, and runs on Linux.
`commands-agree` runs each command on the terminal kit for its help, so it's
given those commands built, and runs on Linux, where the first builds. It
reads the scripts Nix wrappers build as well as the crates' programs.
Every rule but `every-ruff-exception-hides-a-finding` is also given to other
repositories built on core, through `core.lib` in `flake.nix`, with a waiting
list of their own; they lint with core's ruff.toml, which that one reads.
It reads core's tree alone, so an exception it tells you to take off may still
hide a finding in theirs, as `*-test.py` and `*-vm-test.py` match files
anywhere: such a removal shows in their treefmt check once they take core.

| Rule                                             | Holds                                                         |
| ------------------------------------------------ | ------------------------------------------------------------- |
| `every-long-file-says-why`                       | a code file past 800 lines or folder past 40 entries says why |
| `programs-live-in-their-own-files`               | no program past five lines is a string in Nix                 |
| `every-fixed-wait-says-why`                      | a sleep in a test says what it waits through                  |
| `every-assert-says-what-was-wrong`               | an assertion that can't show each side says what was wrong    |
| `every-test-runs-or-fails`                       | a test that can't run fails rather than passing               |
| `every-suppression-says-why`                     | a lint suppression says why, where it's made                  |
| `every-script-meets-shellcheck-at-its-strictest` | a shell script meets shellcheck's optional fault checks       |
| `every-test-function-lives-once`                 | no function in test code is copied into another file          |
| `every-lock-agrees-with-its-manifests`           | a Cargo.lock names what its crates' manifests take            |
| `every-ruff-exception-hides-a-finding`           | each per-file exception in ruff.toml hides a finding          |
| `daemons-agree`                                  | every daemon keeps docs/daemons.md's convention               |
| `commands-agree`                                 | every command keeps docs/commands.md's convention             |

merge-net knows a `<rule>-waiting.nix` here by its path: when main moves one
under a change it's merging, it reruns only that rule's check rather than
every check the change touches.
