# What VM Tests Share

The pieces every VM test is built from, so a test's own files say only its
machines and what it checks. Each is proven by a check of its own, against
stand-ins, without booting a machine.

| File                             | Holds                                                         |
| -------------------------------- | ------------------------------------------------------------- |
| `pytest-vm-test.nix`             | a VM test whose checks are pytest tests, as each test calls it |
| `vm_pytest.py`                   | the plugin that runs those tests inside the NixOS test driver |
| `vm_helpers.py`                  | the helpers a test's script calls on its machines             |
| `vm-helpers.nix`                 | both Python files, as a package the driver's Python takes     |
| `qemu-after-its-sockets.nix`     | the qemu tests run, which waits for the sockets it connects to |
| `qemu-after-its-sockets.sh`      | the wait itself, run before each qemu-system program          |
| `*-test.py`                      | the tests of the files above, by the name of what they test   |

What belongs here is shared by more than one VM test. A helper one test
needs stays in that test's own files.
