# Tools

The programs the dev shell gives whoever is changing core.nix, each in
a folder of its own with its tests beside it. None is part of a
machine, and none is a test: the checks that prove them are in the
root `checks.nix`. [AGENTS.md][agents] says when to use each.

| Folder            | Holds                                                         |
| ----------------- | ------------------------------------------------------------- |
| `changed-checks/` | the checks a change touched, found by comparing derivations   |
| `run-subtests/`   | a VM test's named subtests, run on its driver in seconds      |
| `shell-preview/`  | the shell from the working copy, in a nested niri window      |
| `shots-changed/`  | which of a VM test's pictures changed, and how, between runs  |

A program goes here only if it serves the work of changing this
repository. Something a machine runs is a module, and a script a
check runs lives beside the check.

[agents]: ../AGENTS.md
