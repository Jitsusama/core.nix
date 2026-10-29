# One stick for every machine: a repair stick, written by the guide with
# its boot program signed by two machines' db keys, boots with Secure Boot
# enforced under each machine's firmware, which holds only its own keys,
# and a third machine, whose firmware holds neither, refuses it. UEFI
# boots a program when any one of its signatures is by a key its db
# holds, so this is the firmware's own word that the two signatures sit
# side by side as it reads them. Each machine is one virtual machine's
# firmware variables, made before QEMU starts, and the tests boot the same
# machine under each in turn. The tests are guide-one-stick-vm-test.py, run
# through pytest.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  guide = pkgs.callPackage ../../../modules/guide/package.nix { };
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "guide-one-stick";
  globalTimeout = 20 * 60;

  # The stick's system, built as a test node so the driver can reach its
  # shell, and never started itself: the machine boots its image from USB.
  nodes.stick = {
    imports = [ self.nixosModules.guide-stick ];
    virtualisation = {
      fileSystems = lib.mkVMOverride { };
      directBoot.enable = false;
      mountHostNixStore = false;
      useEFIBoot = true;
    };
  };

  # The machine the stick boots on, whose own system never starts.
  nodes.machine = {
    fileSystems."/" = {
      device = "none";
      fsType = "tmpfs";
    };
    boot.loader.grub.enable = false;
    system.stateVersion = "26.05";
    virtualisation = {
      directBoot.enable = false;
      mountHostNixStore = false;
      useDefaultFilesystems = false;
      fileSystems = lib.mkForce { };
      useEFIBoot = true;
      memorySize = 2048;
      qemu.options = [
        "-device qemu-xhci,id=xhci"
        "-drive if=none,id=stick,format=qcow2,file=$STICK_IMAGE"
        "-device usb-storage,bus=xhci.0,drive=stick,bootindex=0"
      ];
    };
  };

  tests = ./guide-one-stick-vm-test.py;
  values =
    { nodes, ... }:
    let
      machines = [
        "first"
        "second"
        "neither"
      ];
      # Each machine's own Secure Boot keys, as sbctl lays out a db key's
      # directory, and firmware variables holding only them, enforcing.
      firmware =
        pkgs.runCommand "machines-firmware"
          {
            nativeBuildInputs = [
              pkgs.openssl
              pkgs.python3Packages.virt-firmware
            ];
          }
          ''
            # Who owns each key, which the firmware keeps and never checks.
            owner=00000000-0000-4000-8000-000000000001
            for machine in ${lib.escapeShellArgs machines}; do
              mkdir -p $out/$machine
              for key in PK KEK db; do
                openssl req -new -x509 -newkey rsa:2048 -nodes -days 3650 \
                  -subj "/CN=$machine $key" \
                  -keyout $out/$machine/$key.key -out $out/$machine/$key.pem
              done
              virt-fw-vars --input ${nodes.machine.virtualisation.efi.variables} \
                --output $out/$machine.fd \
                --set-pk $owner $out/$machine/PK.pem \
                --add-kek $owner $out/$machine/KEK.pem \
                --add-db $owner $out/$machine/db.pem \
                --secure-boot
            done
          '';
      image = "${nodes.stick.system.build.image}/${nodes.stick.image.filePath}";
      # A repair stick signed for the first and second machines.
      written = pkgs.runCommand "stick-for-two-machines" { nativeBuildInputs = [ guide ]; } ''
        mkdir $out
        truncate -s $(( $(stat -c %s ${image}) + (128 << 20) + 7 * 512 )) $out/stick.raw
        guide write-stick ${image} $out/stick.raw \
          --sign-with ${firmware}/first --sign-with ${firmware}/second
      '';
    in
    pkgs.writeText "guide-one-stick-values.json" (
      builtins.toJSON {
        qemuImg = "${nodes.machine.virtualisation.qemu.package}/bin/qemu-img";
        stick = "${written}/stick.raw";
        firmware = "${firmware}";
      }
    );
}
