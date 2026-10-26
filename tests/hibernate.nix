# Hibernates a machine laid out as the laptops are: systemd in the
# initrd, booted through UEFI, its swap file on btrfs in a subvolume of its
# own inside LUKS, zram in front of it, and no resume= on the kernel's
# command line. systemd-hibernate writes where the image went into the EFI
# variable HibernateLocation, and the initrd resumes from it once LUKS is
# open, so nothing about the disk's layout is kept in Nix. The tests are
# hibernate-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  # Where a laptop's disk takes its TPM and PIN, this one takes a key file.
  key = pkgs.writeText "disk.key" "a key for a test disk";
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "hibernate";

  nodes.machine =
    { config, lib, ... }:
    {
      imports = [ self.nixosModules.memory ];
      virtualisation = {
        useEFIBoot = true;
        memorySize = 1024;
        # The disk LUKS and btrfs are made on, in the tests, with room for a
        # swap file as big as memory. It's /dev/vdc: the root disk is
        # /dev/vda, and the store's image /dev/vdb.
        emptyDiskImages = [ (2 * config.virtualisation.memorySize) ];
        # The image is written to and read from the machine's own disks,
        # and a store shared from the host isn't one.
        useNixStoreImage = true;
        sharedDirectories = lib.mkForce { };
      };
      boot.initrd.systemd.enable = true;
      # A laptop's initrd has btrfs for its root. Without it, udev can't ask
      # whether the swap file's filesystem is whole, so its device never
      # shows and the resume waits it out.
      boot.initrd.supportedFilesystems.btrfs = true;
      # Opened in the initrd, as a laptop's disk is. It's empty until the
      # tests make it, so the first boot carries on without it. The test VM
      # empties the disks to open at this priority, and sets at the same one
      # are joined.
      boot.initrd.luks.devices = lib.mkVMOverride {
        disk = {
          device = "/dev/vdc";
          keyFile = "/etc/disk.key";
          crypttabExtraOpts = [ "nofail" ];
        };
      };
      boot.initrd.systemd.contents."/etc/disk.key".source = key;
      environment.etc."disk.key".source = key;
      environment.systemPackages = [
        pkgs.btrfs-progs
        pkgs.cryptsetup
      ];
      # The driver's backdoor is stopped before the image is made and
      # started again after, or the driver would talk to a shell that was
      # frozen mid-word.
      powerManagement.powerDownCommands = "systemctl --no-block stop backdoor.service";
      powerManagement.resumeCommands = "systemctl --no-block restart backdoor.service";
    };

  tests = ./hibernate-vm-test.py;
}
