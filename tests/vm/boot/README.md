# The Boot VM Tests

Starting and stopping a machine, on machines booted in QEMU: the disk's ask
in the boot theme, on one screen and on two, with a Bluetooth keyboard, and
back from hibernation; the greeter that signs in; and a reboot that can't
hang. Each test's opening comment says what it types, reads or stands in
for.

| File                                           | Holds                                                         |
| ---------------------------------------------- | ------------------------------------------------------------- |
| `bluetooth-disk-prompt.nix`, `-vm-test.py`     | a paired Bluetooth keyboard typing at the disk prompt         |
| `bluetooth-keyboard-stand-in.py`               | the keyboard that test pairs with and types from              |
| `greeter.nix`, `-vm-test.py`                   | the greeter asking, signing in, and showing again after       |
| `plymouth.nix`, `-vm-test.py`                  | the theme asking for the disk, naming jobs, and shutting down |
| `plymouth-screens.nix`, `-vm-test.py`          | the theme's ask shown whole on two screens of different sizes |
| `plymouth-restore.nix`, `-vm-test.py`          | what the screen says while hibernation's image is read back   |
| `shutdown.nix`, `-vm-test.py`                  | a reboot forcing its way past what would hang it              |
| `shutdown-plymouth.sh`, `shutdown-stubborn.sh` | that test's stand-ins for Plymouth and a stubborn program     |
