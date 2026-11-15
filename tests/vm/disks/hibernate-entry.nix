# Hibernates a machine started through systemd-boot from an entry other
# than its default, as a laptop is between a switch to a new initrd and its
# next reboot, and shows the image is read back through the entry it
# started from: hibernate-unlock sets systemd-boot's one-shot entry before
# the image is made, and unsets it after (modules/hibernate/unlock/src/
# oneshot.rs). The default and the entry differ by their initrds alone,
# the other entry being a specialisation of the same machine with a file
# more in its initrd. It swaps to a disk of its own, with no LUKS or TPM,
# which hibernate.nix covers. The tests are hibernate-entry-vm-test.py,
# which pytest runs inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "hibernate-entry";

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.hibernate ];
      # Started by the firmware through systemd-boot, from the entries it
      # keeps, rather than by QEMU handing it a kernel.
      virtualisation = {
        useBootLoader = true;
        useEFIBoot = true;
        # The image is written to and read from the machine's own disks,
        # and a store shared from the host isn't one.
        useNixStoreImage = true;
        sharedDirectories = lib.mkForce { };
        memorySize = 1024;
        emptyDiskImages = [ 2048 ];
      };
      boot.loader.systemd-boot.enable = true;
      boot.loader.efi.canTouchEfiVariables = true;
      # systemd's initrd reads where the image went from the EFI variable
      # systemd-hibernate keeps, so nothing names the swap's disk to it. The
      # empty disk comes third, after the boot disk and the store's image.
      swapDevices = lib.mkOverride 0 [
        {
          device = "/dev/vdc";
          options = [ "x-systemd.makefs" ];
        }
      ];
      # Another entry of the same machine, whose initrd holds a file more.
      specialisation.other.configuration = {
        boot.initrd.systemd.contents."/etc/another-initrd".text = "another initrd";
      };
      # The driver's backdoor is stopped before the image is made and
      # started again after, or the driver would talk to a shell that was
      # frozen mid-word.
      powerManagement.powerDownCommands = "systemctl --no-block stop backdoor.service";
      powerManagement.resumeCommands = "systemctl --no-block restart backdoor.service";
    };

  tests = ./hibernate-entry-vm-test.py;
}
