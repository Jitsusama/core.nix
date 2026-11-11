# The boot VM tests' checks, which checks.nix gathers with every other
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
  # A keyboard at the disk prompt is the TPM, the boot stub, the initrd's
  # BlueZ and the kernel's HID together, before any disk is open.
  vm-bluetooth-types-at-the-disk-prompt = import ./bluetooth-disk-prompt.nix {
    inherit self pkgs;
  };
  # The greeter is all that stands between a logout and the desktop.
  vm-greeter-signs-in = import ./greeter.nix { inherit self pkgs; };
  vm-boot-asks-in-the-theme = import ./plymouth.nix { inherit self pkgs; };
  # Whether the ask shows whole on screens of different sizes shows only
  # with Plymouth drawing on two of them at once.
  vm-boot-asks-on-every-screen = import ./plymouth-screens.nix { inherit self pkgs; };
  # What the screen shows while a hibernation's image is read shows only
  # with Plymouth up and the kernel reading an image it wrote.
  vm-boot-says-it-wakes-from-hibernation = import ./plymouth-restore.nix {
    inherit self pkgs;
  };
  # Whether a reboot gets past what won't stop shows only by rebooting, with
  # a watchdog there to reset the machine.
  vm-reboot-cannot-hang = import ./shutdown.nix { inherit self pkgs; };
}
