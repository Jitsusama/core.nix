# Hibernates the graphical role, its swap file on btrfs inside LUKS as the
# laptops' is, and starts it from its image, the disk's passphrase typed at
# the theme's ask. The kernel stops every program before it reads the image
# back, Plymouth with them, so what the screen shows while it reads is the
# last frame Plymouth drew: the test photographs the screen until the
# kernel says it has read it all, and reads the photographs taken while it
# was reading for the theme's words for a restore. Once the machine is back,
# the hook says how long the restore took after the disk opened. The
# machine holds incompressible bytes in memory first, so the image takes
# seconds to read rather than the moment a bare machine's does. The script is
# plymouth-restore-vm-test.py.
{ self, pkgs }:
let
  account = "jitsusama";
  passphrase = "4815";
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "plymouth-restore";
  enableOCR = true;
  # The base role sets nixpkgs' own settings.
  node.pkgsReadOnly = false;

  nodes.machine =
    { config, lib, ... }:
    {
      imports = [
        self.nixosModules.graphical
        self.nixosModules.hibernate
      ];
      # The GPU's driver in the initrd, so Plymouth has a screen before the
      # disk opens, and Plymouth drawing though the test reads the machine
      # through a serial console.
      boot.initrd.kernelModules = [ "virtio_gpu" ];
      boot.kernelParams = [ "plymouth.ignore-serial-consoles" ];

      jitsusama.login.account = account;
      users.users.${account}.isNormalUser = true;
      home-manager.users.${account}.home.stateVersion = "26.05";

      virtualisation = {
        memorySize = 3072;
        cores = 4;
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
        useEFIBoot = true;
        # The hook knows the machine started from its image by the TPM's
        # counts, as hibernate.nix's does, and the firmware without TPM
        # support keeps its memory map from start to start, which the
        # kernel needs to take an image back.
        tpm.enable = true;
        efi.OVMF = pkgs.OVMF;
        # The disk LUKS and btrfs are made on, in the tests, as /dev/vdc,
        # with room for a swap file as big as memory.
        emptyDiskImages = [ (config.virtualisation.memorySize + 1024) ];
        # The image is written to and read from the machine's own disks,
        # and a store shared from the host isn't one.
        useNixStoreImage = true;
        sharedDirectories = lib.mkForce { };
      };
      boot.initrd.systemd.enable = true;
      boot.initrd.supportedFilesystems.btrfs = true;
      # Opened in the initrd with the passphrase, at the theme's ask. It's
      # empty until the tests make it, so the first boot carries on without
      # it.
      boot.initrd.luks.devices = lib.mkVMOverride {
        disk = {
          device = "/dev/vdc";
          crypttabExtraOpts = [ "nofail" ];
        };
      };
      boot.initrd.availableKernelModules = [ "tpm_tis" ];
      environment.systemPackages = [
        pkgs.btrfs-progs
        pkgs.cryptsetup
      ];
      # The driver's shell is stopped before the image is made and started
      # again after, or the driver would talk to a shell frozen mid-word.
      powerManagement.powerDownCommands = ''
        systemctl --no-block stop backdoor.service
      '';
      powerManagement.resumeCommands = ''
        systemctl --no-block restart backdoor.service
      '';
    };

  tests = ./plymouth-restore-vm-test.py;
  values = _: pkgs.writeText "plymouth-restore-values.json" (builtins.toJSON { inherit passphrase; });
}
