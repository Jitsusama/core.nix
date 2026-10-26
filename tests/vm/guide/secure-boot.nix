# Installs core's disk layout and Secure Boot onto a virtual machine with a
# TPM, then walks docs/installing.md the way a laptop does: lanzaboote makes
# and enrols the keys on the first boot, the disk is bound to the TPM's
# measured-boot policy with a PIN, a recovery key is added and the install
# passphrase wiped, and the disk opens with the PIN alone. A changed PCR makes
# the TPM refuse. The TPM keeps no NvPCRs, so no boot or login waits on them.
# Each generation is counted as it boots and blessed once it's up, and one
# whose tries are spent is passed over for the last good one, the TPM still
# opening the disk, with a word to the session. The tests are
# secure-boot-vm-test.py, run through pytest.
{ self, pkgs }:
let
  passphrase = "install-passphrase";
  pin = "2468";
  notices = pkgs.writeShellApplication {
    name = "stand-in-notices";
    text = ''
      exec ${
        pkgs.python3.withPackages (python: [ python.dbus-fast ])
      }/bin/python3 ${../../stand-in-notices.py} "$@"
    '';
  };
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "secure-boot";
  globalTimeout = 40 * 60;

  nodes.machine =
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

      environment.systemPackages = [
        notices
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
    };

  # The driver runs on the host, so what the tests need from Nix, the image
  # to boot, the tool to put an overlay on it, the systems to install and the
  # secrets to type, reaches them as their values fixture rather than on the
  # machine's path.
  tests = ./secure-boot-vm-test.py;
  values =
    { nodes, ... }:
    let
      inherit (nodes.machine.system.build)
        diskoImages
        later
        optionRomCheck
        toplevel
        ;
      values = pkgs.writeText "secure-boot-values.json" (
        builtins.toJSON {
          qemuImg = "${nodes.machine.virtualisation.qemu.package}/bin/qemu-img";
          image = "${diskoImages}/main.qcow2";
          optionRomCheck = "${optionRomCheck}/bin/option-rom-check";
          real = "${toplevel}";
          later = "${later}";
          inherit passphrase pin;
        }
      );
    in
    values;
}
