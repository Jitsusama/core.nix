# A machine recovered from a repair stick, its data left in place: the
# stick, written by the guide with nothing of the machine's on it, boots
# into the guide, which finds the machine's disk by core's layout, opens it
# with a YubiKey's PIN, mounts every subvolume and the boot partition,
# copies the home folders onto a USB drive, sets which generation boots
# next, replaces a lost YubiKey, and hands over a root shell inside the
# installed system. The machine's disk is one disko's
# image builder laid out and installed, as vm-secure-boot-installs builds
# its own; its two YubiKeys are enrolled from the stick before the guide
# starts, as a machine's own install leaves them. The tests are
# guide-recover-vm-test.py, run through pytest.
{ self, pkgs }:
let
  # Standing in for the recovery key: what opens the disk's passphrase slot
  # is what the recovery key's slot takes, typed text.
  recovery = "the machines recovery key";
  pin = "123456";
  guide = pkgs.callPackage ../../../modules/guide/package.nix { };

  # The kept, lost and new keys, one device swapped between their state
  # files (canokeys.nix).
  canokeys = import ./canokeys.nix { inherit pkgs; };
  inherit (canokeys) qemu;
  keys = canokeys.keys [
    "kept"
    "lost"
    "new"
  ];

  # A USB drive to copy the home folders onto, as a person's often is: one
  # exFAT partition in a GPT, made on its own and put in place at 1 MiB.
  drive =
    pkgs.runCommand "copy-drive"
      {
        nativeBuildInputs = [
          pkgs.exfatprogs
          pkgs.gptfdisk
        ];
      }
      ''
        truncate -s 64M partition.raw
        mkfs.exfat partition.raw
        truncate -s 66M $out
        sgdisk --new=1:2048:133119 --typecode=1:0700 $out
        dd if=partition.raw of=$out bs=1M seek=1 conv=notrunc status=none
      '';
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "guide-recover";
  globalTimeout = 30 * 60;
  qemu.package = qemu;

  nodes.stick = import ./stick.nix { inherit self pkgs; };

  nodes.machine =
    { config, lib, ... }:
    {
      imports = [ self.nixosModules.disko ];
      jitsusama.disk = {
        device = "/dev/vda";
        swapSize = "64M";
      };
      disko.devices.disk.main.content.partitions.system.content.passwordFile = toString (
        pkgs.writeText "recovery" recovery
      );
      disko.devices.disk.main.imageSize = "4G";
      disko.imageBuilder.imageFormat = "qcow2";
      disko.memSize = 4096;
      # disko's image builder hands vmTools its module tree as the kernel,
      # which current nixpkgs refuses (disko#1298), as secure-boot.nix beside
      # this says.
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
      boot.loader.systemd-boot.enable = true;
      # nixos-install puts the boot loader on through it; tests leave it out.
      system.switch.enable = true;
      networking.hostName = "machine";
      system.stateVersion = "26.05";
      virtualisation = {
        directBoot.enable = false;
        mountHostNixStore = false;
        useDefaultFilesystems = false;
        fileSystems = lib.mkForce { };
        useEFIBoot = true;
        memorySize = 2048;
        # Typed keys go to the keyboard QEMU added last, a USB one: the PS/2
        # keyboard is 0001:0001, which the stick lays Colemak out on, as the
        # XPS 14's built-in keyboard. The stick boots first, by USB.
        qemu.virtioKeyboard = false;
        qemu.options = [
          "-device qemu-xhci,id=xhci"
          "-drive if=none,id=stick,format=qcow2,file=$STICK_IMAGE"
          "-device usb-storage,bus=xhci.0,drive=stick,bootindex=0"
          "-drive if=none,id=drive,format=qcow2,file=$DRIVE_IMAGE"
          "-device usb-storage,bus=xhci.0,drive=drive"
          "-device canokey,id=key,bus=xhci.0,file=$KEYS_DIR/kept"
          "-device usb-kbd,bus=xhci.0"
        ];
      };
    };

  # The driver runs outside the virtual machines, so what the tests need from Nix, the
  # images to put overlays on, the tool to make them, the keys' state files
  # and the secrets to type, reaches them as their values fixture rather
  # than on the machine's path.
  tests = ./guide-recover-vm-test.py;
  values =
    { nodes, ... }:
    let
      image = "${nodes.stick.system.build.image}/${nodes.stick.image.filePath}";
      # A repair stick: the image and an empty vault, nothing of the
      # machine's.
      written = pkgs.runCommand "repair-stick" { nativeBuildInputs = [ guide ]; } ''
        mkdir $out
        truncate -s $(( $(stat -c %s ${image}) + (128 << 20) + 7 * 512 )) $out/stick.raw
        guide write-stick ${image} $out/stick.raw
      '';
      values = pkgs.writeText "guide-recover-values.json" (
        builtins.toJSON {
          qemuImg = "${qemu}/bin/qemu-img";
          image = "${nodes.machine.system.build.diskoImages}/main.qcow2";
          stick = "${written}/stick.raw";
          drive = "${drive}";
          keys = "${keys}";
          inherit pin recovery;
        }
      );
    in
    values;
}
