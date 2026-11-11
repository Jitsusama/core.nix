# The Devices VM Tests

What a laptop's devices do on a running machine, booted in QEMU: a desk's
speakers and lights plugged in, pulled out and back, a lid shut, a machine
left idle, a USB hub allowed to wake it, and the cameras programs read. Each
test's opening comment says what stands in for the device a VM lacks.

| File                                     | Holds                                                       |
| ---------------------------------------- | ----------------------------------------------------------- |
| `desk-facts.nix`, `-vm-test.py`          | desk-facts following a USB sound card through a replug      |
| `desk-lights.nix`, `-vm-test.py`         | the desk's lights told what to do again once plugged back   |
| `desk-light-stand-in.py`                 | that test's desk light, made of uhid                        |
| `idle-sleep.nix`, `-vm-test.py`          | a machine left idle, sleeping only on battery               |
| `lid.nix`, `-vm-test.py`                 | a lid shut on battery and charger, with a monitor and not   |
| `lid-hinge.nix`, `-vm-test.py`           | a lid shut with its hinge coming down, left open, slammed   |
| `lid-hinge-stand-in-hinge.sh`            | that test's stand-in hinge                                  |
| `usb-wake.nix`, `-vm-test.py`            | the USB 2 and 1.1 root hubs allowed to wake it, not USB 3's |
| `ir-camera.nix`, `-vm-test.py`           | a stand-in IR camera relayed only to the programs reading   |
| `loopback-holders.nix`, `-vm-test.py`    | which programs hold a loopback camera open                  |

Both lid tests take their lid from `../../stand-in-lid/`, shared with
others.

- `ir-camera/`: the stand-in infrared camera, and the reader that names each
  frame it saved.
- `loopback-holders/`: the programs that hold a loopback camera open, one
  under a name it's given.
