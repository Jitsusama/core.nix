# The stick's own system: a read-only, dm-verity protected store under a
# root in memory, booting straight into the guide's install on the first
# console as root. Its UKI is the ESP's removable-disk boot file, which the
# host writing the stick signs for the machine it's meant for, and its
# vault is a partition of its own the guide writes (src/stick/).
#
# The keyboard and the network are core's own, so the stick types and joins
# Wi-Fi as every machine does: Colemak Mod-DH on the built-in keyboard of
# every machine core describes, since the same stick boots on any of them.
{ nixosModules, nixpkgs }:
{ lib, pkgs, ... }:
let
  # Every machine core describes, by the hardware module that says so, and
  # each one's own keyboard as its hardware names it.
  hardware = lib.filterAttrs (
    _: module: builtins.isPath module && lib.hasInfix "/hardware/" (toString module)
  ) nixosModules;
  builtIn =
    module: (nixpkgs.lib.nixosSystem { modules = [ module ]; }).config.jitsusama.keyboard.builtIn;

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
      ukiPath = "/EFI/BOOT/BOOTX64.EFI";
    };
    partitions."00-esp".repartConfig = {
      Type = "esp";
      Format = "vfat";
      # Room for the UKI twice over, so it can grow by a signature.
      SizeMinBytes = "256M";
    };
  };
  system.image = {
    id = "stick";
    version = "1";
  };

  fileSystems."/" = {
    fsType = "tmpfs";
    options = [ "mode=0755" ];
  };

  boot = {
    loader.grub.enable = false;
    initrd.systemd.enable = true;
    kernelPackages = pkgs.linuxPackages_latest;
    # The stick's own disk, and the keyboards that reach the system through
    # a controller of their own, such as the Surface Laptop 4's.
    initrd.availableKernelModules = [
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
    ];
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
    fi
  '';

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
}
