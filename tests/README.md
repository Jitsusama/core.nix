# Tests

The checks that need a machine of their own, or read every file in the
repository, and the pieces they're built from. A check that proves one
module without booting anything lives beside that module instead, in its
folder's `checks.nix`. [docs/testing.md][1] says which tier a case goes in
and why.

## What Goes Where

| Kind                                        | Where                                     |
| ------------------------------------------- | ----------------------------------------- |
| a VM test, `<name>.nix` and `<name>-vm-test.py` | `vm/` by area, or here until it moves |
| a VM test split into areas                  | a folder named for it                     |
| a rule held over every file, and its list   | `rules/`                                  |
| a check reading the repository as a whole   | `repository/`                             |
| what several checks are built from          | `lib/`                                    |
| a test on a nested niri, in the sandbox     | beside the shell, or here until it moves  |
| a program's or device's stand-in, its patch | beside its test, or `stand-ins/`          |
| a patch a test's machine takes, not a stand-in's | beside its test, `<what>.patch`      |
| a shell case not moved yet                  | here, until it moves                      |

## Its Folders

- `lib/`, `repository/`, `rules/`, `stand-ins/` and `vm/` say what they hold
  in their own READMEs.
- `desktop/`: the desktop VM test's areas, one `-vm-test.py` each, with the
  launcher's areas in `desktop/launcher/`, the looks areas in
  `desktop/looks/` and the programs its machine runs to drive the desktop
  in `desktop/stand-ins/`, as its README says.

## Moving Out

Most of the shell's own cases are beside it in `modules/quickshell/tests/`, and
those still here follow in later changes. The VM tests go into areas under
`vm/`, each as its owner frees it.

[1]: ../docs/testing.md
