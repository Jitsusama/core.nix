# A virtual machine installed as a laptop is: core's disk layout and
# lanzaboote on an image written as the install writes it, a TPM, UEFI
# firmware and an install passphrase, with a later generation to install.
# Its first boot makes and enrols the keys, and the real system, with
# measured boot, is carried for a test to switch to, since the image
# builder has no TPM to predict a policy with. secure-boot.nix walks its
# install; tests that need a machine booting as an installed laptop does
# start from it too. The packages given go on its path.
{
  self,
  pkgs,
  passphrase,
  packages ? [ ],
}:
{
  config,
  extendModules,
  lib,
  ...
}:
{
  imports = [
    self.nixosModules.disko
    self.nixosModules.lanzaboote
  ];

  jitsusama.disk = {
    device = "/dev/vda";
    swapSize = "64M";
  };
  # OVMF runs the option ROM of QEMU's network card, so the keys carry its
  # checksum, as a machine with a ROM of its own sets them to.
  boot.lanzaboote.autoEnrollKeys.includeChecksumsFromTPM = true;
  # The machine runs the check it would get without that, to see it
  # refuse.
  system.extraDependencies = [ config.system.build.optionRomCheck ];
  # Only the image builder reads it, to format the disk unattended.
  disko.devices.disk.main.content.partitions.system.content.passwordFile = toString (
    pkgs.writeText "passphrase" passphrase
  );
  disko.devices.disk.main.imageSize = "4G";
  # qcow2 keeps only what the install wrote, where a raw image would put
  # four sparse gigabytes in the store.
  disko.imageBuilder.imageFormat = "qcow2";
  disko.memSize = 4096;
  # The image builder's VM has no TPM, which measured boot needs to predict
  # its policy, so the image holds the system without it and carries the
  # real one for the test to switch to. A real install runs on the laptop,
  # which has one.
  disko.imageBuilder.extraConfig = {
    boot.lanzaboote.measuredBoot.enable = lib.mkForce false;
    system.extraDependencies = [
      config.system.build.toplevel
      config.system.build.later
    ];
  };
  # A later generation to install, which differs only in what it says.
  system.build.later =
    (extendModules {
      modules = [ { environment.etc.generation.text = "later"; } ];
    }).config.system.build.toplevel;
  # disko's image builder hands vmTools its module tree as the kernel,
  # which current nixpkgs refuses (disko#1298); its new name is
  # kernelModules, beside the real kernel.
  disko.imageBuilder.pkgs = pkgs.extend (
    _final: prev: {
      vmTools = prev.vmTools // {
        override =
          args:
          prev.vmTools.override (
            removeAttrs args [ "kernel" ]
            // {
              kernelModules = args.kernel;
              kernel = config.disko.imageBuilder.kernelPackages.kernel;
            }
          );
      };
    }
  );

  environment.systemPackages = packages ++ [
    pkgs.tpm2-tools
    pkgs.jq
    pkgs.cryptsetup
  ];

  virtualisation = {
    directBoot.enable = false;
    mountHostNixStore = false;
    useDefaultFilesystems = false;
    fileSystems = lib.mkForce { };
    useEFIBoot = true;
    tpm.enable = true;
    # Typed keys go to the PS/2 keyboard, which the initrd always has.
    qemu.virtioKeyboard = false;
    memorySize = 2048;
  };
  networking.hostName = "machine";
  # A user whose first login logind would measure into the login NvPCR.
  users.users.alice = {
    isNormalUser = true;
    uid = 1000;
  };
  boot.loader.timeout = 0;
  # nixos-install puts the boot loader on through it; tests leave it out.
  system.switch.enable = true;
  system.stateVersion = "26.05";
}
