# The stick's own system: a read-only, dm-verity protected store under a
# root in memory, booting straight into the guide's install on the first
# console as root. Its UKI is the boot partition's removable-disk boot file, which the
# writer signs for the machine it's meant for, and its
# vault is a partition of its own the guide writes (src/stick/).
#
# The keyboard and the network are core's own, so the stick types and joins
# Wi-Fi as every machine does: Colemak Mod-DH on the built-in keyboard of
# every machine core describes, since the same stick boots on any of them.
{ nixosModules, nixpkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Every machine core describes, by the hardware module that says so, and
  # each one's own keyboard as its hardware names it. A module given its
  # inputs through flake.nix's moduleFrom is a set keyed by its file, so
  # it's known by that key, as a plain one is by its path.
  file = module: if builtins.isPath module then toString module else module.key or "";
  hardware = lib.filterAttrs (_: module: lib.hasInfix "/hardware/" (file module)) nixosModules;
  # The keyboard's option comes too, for a model that names no keyboard
  # yet and so doesn't import it.
  builtIn =
    module:
    (nixpkgs.lib.nixosSystem {
      modules = [
        module
        ../keyboard/nixos.nix
      ];
    }).config.jitsusama.keyboard.builtIn;

  guide = pkgs.callPackage ./package.nix { };
  theme = import ./theme.nix { inherit lib; };
in
{
  imports = [
    "${nixpkgs}/nixos/modules/image/repart.nix"
    # wifid gives each account home-manager keeps its sign-in window. The
    # stick keeps none, so this only gives wifid the option it reads.
    nixosModules.home-manager
    nixosModules.keyd
    nixosModules.wifid
  ];

  options.jitsusama.stick.appleRestores = lib.mkOption {
    type = lib.types.listOf lib.types.package;
    default = [ ];
    description = ''
      Apple's restore images, each built by apple-restore.nix, that the
      stick carries as partitions of their own after its system's, for
      `guide revive` to put macOS back on a Mac with Apple silicon. Each
      is tens of gigabytes, so only a machine repository's stick that asks
      carries one, through `lib.guide`'s `appleRestores`.
    '';
  };

  config = {
    jitsusama.keyboard.builtIn = lib.unique (lib.concatMap builtIn (lib.attrValues hardware));

    image.repart = {
      enable = true;
      name = "stick";
      # systemd-repart 261.3 gives mkfs.erofs the image's 512-byte sector size
      # as its block size (src/shared/mkfs-util.c), and libblkid can't check
      # the superblock checksum of an erofs with blocks no bigger than its
      # 1024-byte offset (libblkid/src/superblocks/erofs.c), so udev finds no
      # file system on the store, marks /dev/mapper/usr not ready, and the
      # boot waits for it for ever. Options here come after repart's own, and
      # mkfs.erofs takes the last -b, so the store keeps 4096-byte blocks.
      mkfsOptions.erofs = [ "-b4096" ];
      verityStore = {
        enable = true;
        # Where firmware looks for a removable disk's boot program on this
        # architecture: BOOTX64.EFI on a PC, BOOTAA64.EFI under an Apple
        # silicon Mac's U-Boot.
        ukiPath = "/EFI/BOOT/BOOT${lib.toUpper pkgs.stdenv.hostPlatform.efiArch}.EFI";
      };
      partitions = {
        "00-esp".repartConfig = {
          Type = "esp";
          Format = "vfat";
          # Room for the UKI twice over, so it can grow by a signature.
          SizeMinBytes = "256M";
        };
      }
      # After the store's, which the UKI finds by their GUIDs, so they stay
      # where they are; the writer adds the vault after the last. Each is
      # its erofs copied in whole, and named for the zip it holds, less its
      # `_Restore.ipsw`, as `guide revive` finds it (src/revive.rs).
      // lib.listToAttrs (
        lib.imap0 (at: restore: {
          name = "90-apple-restore-${toString at}";
          value.repartConfig = {
            Type = "linux-generic";
            Label = lib.removeSuffix "_Restore.ipsw" restore.file;
            CopyBlocks = "${restore}";
            ReadOnly = true;
          };
        }) config.jitsusama.stick.appleRestores
      );
    };
    system.image = {
      id = "stick";
      version = "1";
    };

    fileSystems."/" = {
      fsType = "tmpfs";
      options = [ "mode=0755" ];
    };

    # systemd finds the boot partition the stick booted from and mounts it at /boot
    # when anything looks there, as sbctl does. Nothing on the stick needs it,
    # since its boot program has already run, and with the stick out for a
    # YubiKey the mount would wait for a disk that isn't there, stopping
    # whatever looked.
    systemd.units."boot.automount".enable = false;
    systemd.units."boot.mount".enable = false;

    boot = {
      loader.grub.enable = false;
      # A Mac's stick runs Asahi's kernel instead (stick-apple-silicon.nix).
      kernelPackages = lib.mkDefault pkgs.linuxPackages_latest;
      # The stick's own disk, and the keyboards that reach the system through
      # a controller of their own, such as the Surface Laptop 4's. On a Mac,
      # the Air's kernel module names the controllers and usb_storage, and NixOS's
      # own defaults sd_mod, so only UAS, which a USB SSD speaks, is added:
      # Asahi's kernel has none of the Surface's modules.
      initrd.availableKernelModules =
        if pkgs.stdenv.hostPlatform.isx86_64 then
          [
            "xhci_pci"
            "usb_storage"
            "uas"
            "sd_mod"
            "nvme"
            # pinctrl_amd, which the Surface's keyboard also needs, is built in.
            "8250_dw"
            "surface_aggregator"
            "surface_aggregator_registry"
            "surface_aggregator_hub"
            "surface_hid_core"
            "surface_hid"
          ]
        else
          [ "uas" ];
    };

    hardware.enableRedistributableFirmware = true;

    # The guide owns the console, so the kernel writes only its errors
    # there, and a YubiKey plugged in doesn't write over the guide's screen.
    # NixOS's tests turn it up to 7 (testing/test-instrumentation.nix), and
    # this holds above them, a step stronger than a plain setting.
    boot.consoleLogLevel = lib.mkOverride 90 3;

    # The theme's sixteen colours, which the guide's roles name by slot, and
    # a large font, since the console's own is tiny on a laptop's screen.
    console = {
      colors = theme.colours;
      font = "ter-v32n";
      packages = [ pkgs.terminus_font ];
      earlySetup = true;
    };

    # The guide's install, on the first console as root, and a root shell
    # once it's left, as the last way in.
    services.getty.autologinUser = "root";
    environment.loginShellInit = ''
      if [ "$(tty)" = /dev/tty1 ]; then
        guide install
        echo "To start the guide again, type guide install, then press Return."
        ${lib.optionalString (config.jitsusama.stick.appleRestores != [ ]) ''
          echo "To put macOS back on a Mac, type guide revive, then press Return."
        ''}
      fi
    '';

    # A Mac being revived starts again into a system of Apple's that
    # idevicerestore reaches through usbmuxd. usbmuxd's own service checks
    # pairing with each device first, which has failed restores of M2 to M4
    # Macs (idevicerestore issues 782 and 755), so `guide revive` starts
    # this one, which doesn't, on a socket of its own, only while it
    # revives (src/revive.rs).
    systemd.services.guide-usbmuxd = lib.mkIf (config.jitsusama.stick.appleRestores != [ ]) {
      description = "usbmuxd for guide revive, without pairing first";
      serviceConfig.ExecStart = "${lib.getExe' pkgs.usbmuxd "usbmuxd"} --foreground --no-preflight --socket /run/guide/usbmuxd.sock";
    };

    environment.systemPackages = [
      guide
      # For a person at the root shell, a repair the guide doesn't offer yet.
      pkgs.age
      pkgs.age-plugin-fido2-hmac
      pkgs.btrfs-progs
      pkgs.cryptsetup
      pkgs.dosfstools
      pkgs.efibootmgr
      pkgs.libfido2
      pkgs.nvme-cli
      pkgs.sbctl
      pkgs.smartmontools
      pkgs.tpm2-tools
    ];

    networking.hostName = "stick";
    # A read-only /usr can't take /usr/bin/env.
    system.activationScripts.usrbinenv = lib.mkForce "";
    system.stateVersion = "26.05";
  };
}
