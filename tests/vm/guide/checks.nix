# The guide VM tests' checks, which checks.nix gathers with every other
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
  # The disk layout and Secure Boot decide whether a machine starts at all.
  vm-secure-boot-installs = import ./secure-boot.nix { inherit self pkgs; };
  # The guide's install is a stick, a disk, FIDO2 keys, the firmware and
  # a TPM together, driven by its keys as a person drives it.
  vm-guide-installs = import ./guide-install.nix { inherit self pkgs; };
  # Whether the guide leaves another system's partitions as they were is
  # a real disk's table, the kernel's view of it and the firmware's boot.
  vm-guide-installs-beside-another-system = import ./guide-beside.nix {
    inherit self pkgs;
  };
  # Writing a stick on a host is UDisks2, polkit and a USB disk together,
  # as the person rather than root, and a bundle sealed to a FIDO2 key
  # opens only where the plugin, the key and its PIN meet.
  vm-guide-writes-a-stick = import ./guide-writer.nix { inherit self pkgs; };
  # Recovering is a stick, a laid-out disk and its subvolumes together,
  # with nothing of the machine's on the stick.
  vm-guide-recovers = import ./guide-recover.nix { inherit self pkgs; };
  # Whether firmware boots a program signed by several machines' keys is
  # the firmware's own to say, and only a running one says it.
  vm-guide-signs-a-stick-for-two-machines = import ./guide-one-stick.nix {
    inherit self pkgs;
  };
}
