# The Rules Held Over Every File

The checks that hold every file in the repository to one rule of the quality
standard. Each is three files: `<rule>.nix` builds the check, `<rule>.py`,
beside it, says what breaks the rule, and `<rule>-waiting.nix` lists the files
that broke it before the rule came in. A list only shrinks: fixing a file takes
it off, and nothing new goes on. `checks.nix` runs all ten on core itself.
`daemons-agree` reads an evaluated account's units rather than the files
alone, so it's given a graphical machine's account too, and runs on Linux.
`commands-agree` runs each command on the terminal kit for its help, so it's
given those commands built, and runs on Linux, where the first builds.
Every rule is also given to other repositories built on core, through
`core.lib` in `flake.nix`, with a waiting list of their own.

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
| `daemons-agree`                                  | every daemon keeps docs/daemons.md's convention               |
| `commands-agree`                                 | every command keeps docs/commands.md's convention             |

merge-net knows a `<rule>-waiting.nix` here by its path: when main moves one
under a change it's merging, it reruns only that rule's check rather than
every check the change touches.
