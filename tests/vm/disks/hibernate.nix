# Hibernates a machine laid out as the laptops are: systemd in the
# initrd, booted through UEFI, its swap file on btrfs in a subvolume of its
# own inside LUKS, zswap in front of it, and no resume= on the kernel's
# command line. systemd-hibernate writes where the image went into the EFI
# variable HibernateLocation, and the initrd resumes from it once LUKS is
# open, so nothing about the disk's layout is kept in Nix. The disk opens as
# a laptop's does, with the TPM and a PIN typed at the initrd's prompt, so
# coming back asks there once and the lock lets go by itself. It keeps a
# reserve for drivers as the Surface Laptop 4 does, and can be made to roll a
# hibernation back, short of memory or with its swap file gone, so the
# notice's reasons are seen end to end. A slice holds memory under a hard
# floor, as the laptops' slices do, past what the image may hold, to prove
# the floors are set aside while it's made. Its battery, charger and lid are
# stand-ins, the kernel's test_power and stand-in-lid, starting on battery
# with the lid open, so a battery run out with Keep Awake's locks held is
# seen to hibernate it as UPower has it. The tests are
# hibernate-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  # What the disk is made with, before the TPM and its PIN are enrolled.
  # The initrd isn't given it, so it asks for the PIN.
  key = pkgs.writeText "disk.key" "a key for a test disk";
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "hibernate";

  nodes.machine =
    { config, lib, ... }:
    {
      imports = [
        self.nixosModules.hibernate
        self.nixosModules.memory
        self.nixosModules.power-profiles-daemon
      ];
      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ../../stand-in-lid { })
      ];
      boot.kernelModules = [
        "stand-in-lid"
        "test_power"
      ];
      # Its USB supply would count as a charger too.
      boot.extraModprobeConfig = ''
        options test_power ac_online=off usb_online=off
      '';
      virtualisation = {
        useEFIBoot = true;
        # A software TPM, whose state outlives each start, as a laptop's
        # TPM does.
        tpm.enable = true;
        # Firmware without TPM support. The full firmware puts a small ACPI
        # region somewhere new on every start, and the kernel refuses an image
        # written under a different memory map; a laptop's firmware keeps its
        # map. The kernel starts the TPM itself.
        efi.OVMF = pkgs.OVMF;
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
          crypttabExtraOpts = [
            "nofail"
            "tpm2-device=auto"
          ];
        };
      };
      # QEMU's TPM, for the initrd to open the disk with.
      boot.initrd.availableKernelModules = [ "tpm_tis" ];
      environment.etc."disk.key".source = key;
      environment.systemPackages = [
        pkgs.btrfs-progs
        pkgs.cryptsetup
      ];
      # An eighth of memory, about the share the Surface Laptop 4 keeps of
      # its own, which the tests read back and hibernate a loaded machine
      # with.
      jitsusama.hibernate.reserve = 128 * 1024 * 1024;
      # The driver's backdoor is stopped before the image is made and
      # started again after, or the driver would talk to a shell that was
      # frozen mid-word. A hibernation the tests make roll back keeps it,
      # since the shell only freezes and thaws with everything else, and a
      # new one would leave the driver reading its greeting as an answer.
      powerManagement.powerDownCommands = ''
        [ -e /run/rolls-back ] || systemctl --no-block stop backdoor.service
      '';
      powerManagement.resumeCommands = ''
        [ -e /run/rolls-back ] || systemctl --no-block restart backdoor.service
      '';
      # systemd-sleep tells the kernel where the swap file is before it runs
      # the hooks, so taking the file away here leaves the kernel nowhere
      # to write the image.
      environment.etc."systemd/system-sleep/takes-the-swap-file-away".source =
        pkgs.writeShellScript "takes-the-swap-file-away" ''
          if [ "$1" = pre ] && [ -e /run/no-swap-file ]; then
            ${pkgs.util-linux}/bin/swapoff /swap/swapfile
          fi
        '';
    };

  tests = ./hibernate-vm-test.py;
}
