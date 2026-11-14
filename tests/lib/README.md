# What Checks Share

The pieces more than one check is built from, so a check's own files say
only what it checks. Most are for VM tests, and those are proven by checks
of their own in `checks.nix`, against stand-ins, without booting a machine.
The rest are proven by the checks that use them.

For VM tests:

| File                         | Holds                                                          |
| ---------------------------- | -------------------------------------------------------------- |
| `pytest-vm-test.nix`         | a VM test whose checks are pytest tests, as each test calls it |
| `vm_pytest.py`               | the plugin that runs those tests inside the NixOS test driver  |
| `vm_helpers.py`              | the helpers a test's script calls on its machines              |
| `vm-helpers.nix`             | both Python files, as a package the driver's Python takes      |
| `driver_names.pyi`           | what the driver gives a test's script, for type checkers only  |
| `qemu-after-its-sockets.nix` | the qemu tests run, which waits for the sockets it connects to |
| `qemu-after-its-sockets.sh`  | the wait itself, run before each qemu-system program           |
| `*-test.py`                  | the tests of the files above, by the name of what they test    |

For other checks:

| File                         | Holds                                                             |
| ---------------------------- | ----------------------------------------------------------------- |
| `Expect.js`                  | what a QML test says of its cases, as `expect`, `same` and `fail` |
| `Frames.js`                  | a stand-in launcher's afterFrame, run as a test says a frame is   |
| `HeardWindows.js`            | a stand-in niri's windows, told as the windows daemon tells them  |
| `Watch.js`                   | a change a stand-in daemon tells the watch the shell holds on it  |
| `Memory.js`                  | a QML test's own anonymous memory, read from /proc               |
| `Until.qml`                  | a QML test's wait for what a daemon it runs has yet to do         |
| `qml-in-time.sh`             | a QML test that times itself, judged by its fastest of five runs  |
| `qmllint-counted.{nix,sh}`   | qmllint, printing every message it counted                        |
| `shell-folder-lint.{nix,sh}` | qmllint over a check's laid-out shell, against Quickshell's types |
| `says-what-it-has-open.sh`   | a program opened on a file, asked what it has open                |
| `page.nix`                   | the smallest PDF a viewer opens, for checks that open documents   |

What belongs here is shared by more than one check. A helper one check needs
stays in that check's own files.
