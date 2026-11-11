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
| a rule held over every file, and its list   | here, `<rule>.py` and `<rule>-waiting.nix` |
| a check reading the repository as a whole   | `repository/`                             |
| what several checks are built from          | `lib/`                                    |
| a test on a nested niri, in the sandbox     | here, `<what>-on-niri-test.py`            |
| a stand-in for a program or a device        | beside its test, or a folder of its own   |
| a patch a test's machine takes              | here, `<what>.patch`                      |
| a shell QML case not yet beside the shell   | here, `<case>.qml`                        |

The rules held over every file stay at the top, since merge-net reads a
waiting list only as `tests/<rule>-waiting.nix`, and each list only shrinks.

## Its Folders

- `battery-stand-ins/`, `lib/`, `repository/` and `vm/` say what they hold in
  their own READMEs.
- `desktop/`: the desktop VM test's areas, one `-vm-test.py` each, with the
  programs its machine runs to drive the desktop.
- `screens/`: the screens test's areas, as screens come and go, keep their
  places and keep the launcher, and the session they share.
- `niri-off-cpu/`: a stand-in for niri's main thread, waiting and asleep as
  niri does.
- `stand-in-backlight/`, `stand-in-lid/`, `stand-in-light/` and
  `stand-in-monitor/`: kernel modules standing in for a device, each built by
  its `default.nix`.

## Moving Out

The folder holds more than the 40 entries a folder may, and
`every-long-file-says-why` waits on it. It isn't kept that way on purpose: the
QML cases move beside the shell, under `modules/quickshell/`, and the VM tests
into areas under `vm/`, each as its owner frees it.

[1]: ../docs/testing.md
