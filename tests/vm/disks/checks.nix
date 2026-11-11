# The disks VM tests' checks, which checks.nix gathers with every other
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
  # Only a running machine attaches a loop device, mounts it, and says
  # whether Btrfs copies the image on write.
  vm-cargo-builds-on-their-own-filesystem = import ./cargo-builds.nix {
    inherit self pkgs;
  };
  vm-disk-guard-keeps-the-disk = import ./disk-guard.nix { inherit self pkgs; };
  # Whether the kernel panics once a disk is gone, and only then, shows
  # only with a disk to take away.
  vm-disk-loss-panic-panics-once-the-disk-is-gone = import ./disk-loss-panic.nix {
    inherit self pkgs;
  };
  # Whether the swap comes down while udev probes its device shows only with
  # the device there and held open.
  # Whether systemd-oomd ends the work that filled swap before the kernel's
  # OOM killer picks a program shows only with swap filling for real.
  vm-full-swap-ends-what-filled-it = import ./full-swap.nix { inherit self pkgs; };
  # Whether a machine comes back from hibernating with no resume= shows only
  # with systemd's initrd opening LUKS and reading the image it wrote.
  vm-hibernation-comes-back = import ./hibernate.nix { inherit self pkgs; };
}
