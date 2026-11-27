# The Rules Held Over Every File

The checks that hold every file in the repository to one rule of the quality
standard. Each is three files: `<rule>.nix` builds the check, `<rule>.py`,
beside it, says what breaks the rule, and `<rule>-waiting.nix` lists the files
that broke it before the rule came in. A list only shrinks: fixing a file takes
it off, and nothing new goes on. `every-lock-agrees-with-its-manifests`,
`every-ruff-exception-hides-a-finding` and
`every-script-finds-what-it-imports` have no list, since nothing broke them
when they came in. `comments-name-what-exists` has no `.py`: code-health's
stale-paths measure says what breaks it, so the figure it counts each hour and
the check held at each change are one rule, and its
`comments-name-what-exists.sh` first sees that measure refuse a planted tree
both ways, for a comment naming what isn't there and a waiting line nothing
matches, each named.
`checks.nix` runs all fifteen on core itself.
Many entries because each rule keeps its three files side by side, where a
reader opening one finds the others, rather than in a folder of its own.
`rust_source.py` is no rule: it reads Rust as code alone, past its strings and
comments, for `every-assert-says-what-was-wrong` and `every-fixed-wait-says-why`,
whose checks each take it beside the rule as one folder. The second first sees
itself name just the bare sleeps of a planted Rust test,
`every-fixed-wait-says-why-planted.rs.in`, which isn't named `.rs` so the rule
doesn't count it among core's own tests.
`every-script-finds-what-it-imports` first sees itself fail a planted tree
and pass it once what it imports is on mypy's path, which
`every-script-finds-what-it-imports.sh` writes, and runs on Linux, as the
formatting check does, since core's mypy reads Linux's Python.
`every-ruff-exception-hides-a-finding` keeps its planted tree's config in
`every-ruff-exception-hides-a-finding-planted.toml`, a config it must refuse
in `every-ruff-exception-hides-a-finding-refused.toml` and one ruff can't read
in `every-ruff-exception-hides-a-finding-broken.toml`, and first sees itself
fail on all three and on a config that isn't TOML, which
`every-ruff-exception-hides-a-finding.sh` writes, as it says.
`daemons-agree` reads an evaluated account's units rather than the files
alone, so it's given a graphical machine's account too, and runs on Linux.
`commands-agree` runs each command on the terminal kit for its help, so it's
given those commands built, and runs on Linux, where the first builds. It
reads the scripts Nix wrappers build as well as the crates' programs.
Every rule but `every-ruff-exception-hides-a-finding` is also given to other
repositories built on core, through `core.lib` in `flake.nix`, each but
`every-script-finds-what-it-imports` with a waiting list of their own; that
one judges only what their treefmt hands mypy, so it phases in by their
excludes, and nothing fails an exclude that's no longer needed. Those
repositories lint with core's ruff.toml, which
`every-ruff-exception-hides-a-finding` reads. That rule reads core's tree
alone, so an exception it tells you to take off may still hide a finding in
theirs, since each pattern matches their files from their own root: such a
removal shows in their treefmt check once they take core.

| Rule                                             | Holds                                                         |
| ------------------------------------------------ | ------------------------------------------------------------- |
| `every-long-file-says-why`                       | a code file past 800 lines or folder past 40 entries says why |
| `programs-live-in-their-own-files`               | no program past five lines is a string in Nix                 |
| `every-fixed-wait-says-why`                      | a sleep in a test says what it waits through                  |
| `every-failed-read-says-why`                     | a file read whose failure becomes nothing says why            |
| `every-assert-says-what-was-wrong`               | an assertion that can't show each side says what was wrong    |
| `every-test-runs-or-fails`                       | a test that can't run fails rather than passing               |
| `every-suppression-says-why`                     | a lint suppression says why, where it's made                  |
| `every-script-meets-shellcheck-at-its-strictest` | a shell script meets shellcheck's optional fault checks       |
| `every-test-function-lives-once`                 | no function in test code is copied into another file          |
| `every-lock-agrees-with-its-manifests`           | a Cargo.lock names what its crates' manifests take            |
| `every-ruff-exception-hides-a-finding`           | each per-file exception in ruff.toml hides a finding          |
| `every-script-finds-what-it-imports`             | each folder's scripts pass mypy handed to it alone            |
| `daemons-agree`                                  | every daemon keeps docs/daemons.md's convention               |
| `commands-agree`                                 | every command keeps docs/commands.md's convention             |
| `comments-name-what-exists`                      | a comment's path from a top folder names what's there         |

merge-net knows a `<rule>-waiting.nix` here by its path: when main moves one
under a change it's merging, it reruns only that rule's check rather than
every check the change touches.
