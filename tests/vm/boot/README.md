# The Boot VM Tests

Starting and stopping a machine, on machines booted in QEMU: the disk's ask
in the boot theme, on one screen and on two, with a Bluetooth keyboard, and
back from hibernation; the greeter that signs in; the firmware's records
kept for an administrator; a reboot that can't hang; and a hang that
leaves a record. Each test's opening comment says what it types, reads or
stands in for.

| File                                           | Holds                                                         |
| ---------------------------------------------- | ------------------------------------------------------------- |
| `bluetooth-disk-prompt.nix`, `-vm-test.py`     | a paired Bluetooth keyboard typing at the disk prompt         |
| `bluetooth-keyboard-stand-in.py`               | the keyboard that test pairs with and types from              |
| `firmware-errors.nix`, `-vm-test.py`           | the firmware's records copied where an administrator reads them |
| `greeter.nix`, `-vm-test.py`                   | the greeter asking, signing in, and showing again after       |
| `hang.nix`, `-vm-test.py`                      | a hang's watchdog and panic, and the kernel's log heard away  |
| `plymouth.nix`, `-vm-test.py`                  | the theme asking for the disk, naming jobs, and shutting down |
| `plymouth-screens.nix`, `-vm-test.py`          | the theme's ask shown whole on two screens of different sizes |
| `plymouth-restore.nix`, `-vm-test.py`          | what the screen says while hibernation's image is read back   |
| `shutdown.nix`, `-vm-test.py`                  | a reboot forcing its way past what would hang it              |
| `shutdown-plymouth.sh`, `shutdown-stubborn.sh` | that test's stand-ins for Plymouth and a stubborn program     |
