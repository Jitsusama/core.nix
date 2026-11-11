# The devices VM tests' checks, which checks.nix gathers with every other
# folder's; README.md beside this says what each test proves.
{
  self,
  onLinux,
  vmHostPkgs,
  ...
}:
let
  pkgs = vmHostPkgs;
in
onLinux {
  # A light plugged back in is uhid, hidraw, udev's nodes in /dev and the
  # lights' watches together, which only a running machine has.
  vm-the-desk-lights-come-back = import ./desk-lights.nix { inherit self pkgs; };
  # A card plugged in and pulled out is the kernel's uevents and its USB
  # audio driver together, which only a running machine has.
  vm-desk-facts-follow-a-replug = import ./desk-facts.nix { inherit self pkgs; };
  # Whether the IR camera opens only while it's read, and hands each reader
  # its frames and none of the last one's, shows only with cameras there.
  vm-ir-camera-opens-for-readers = import ./ir-camera.nix { inherit self pkgs; };
  # Whether the programs holding a loopback camera are named shows only with
  # a loopback there and another account's programs holding it.
  vm-loopback-says-who-holds-it = import ./loopback-holders.nix {
    inherit self pkgs;
  };
  # Whether logind and the compositor hear the lid's own switch or
  # lid-hinge's shows only with udev, uinput and logind there.
  vm-lid-shuts-with-the-hinge = import ./lid-hinge.nix { inherit self pkgs; };
  # What logind makes of a shut lid shows only with logind watching one.
  vm-lid-stays-at-the-desk = import ./lid.nix { inherit self pkgs; };
  # What logind makes of an idle machine shows only with logind counting
  # its sessions, its locks and the charger.
  vm-idle-sleep-comes-only-on-battery = import ./idle-sleep.nix {
    inherit self pkgs;
  };
  # Whether udev turns a root hub's wakeup on shows only with a USB
  # controller and udev there.
  vm-plugging-in-wakes-it = import ./usb-wake.nix { inherit self pkgs; };
}
