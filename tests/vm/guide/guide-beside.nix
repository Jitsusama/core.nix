# A machine installed by the guide beside another system's partitions, as
# an Apple silicon Mac is beside macOS: its disk laid out first as Asahi's
# installer leaves a Mac given "UEFI environment only" (Apple's boot
# container, macOS's, Asahi's stub and its EFI system partition holding
# m1n1's files, free space, then the recovery container at the end), and
# the machine with no TPM and no Secure Boot, as a Mac has neither. The
# guide makes the machine's partition in the free space with one YubiKey
# and the recovery words, and installs; the machine boots its own disk,
# opened by the key's PIN, then by the words. Every other partition is the
# same byte for byte, the table's other entries are unchanged, and the
# boot partition keeps Asahi's files beside systemd-boot's.
#
# It's x86-64, since no builder runs an aarch64 VM at full speed; what's
# the Mac's own, its kernel and boot chain, is proven by building the
# Apple stick and example-apple-laptop, and at the Mac. The tests are
# guide-beside-vm-test.py, run through pytest.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  pin = "123456";
  password = "a password for alice";
  passphrase = "the stick's own passphrase";

  canokeys = import ./canokeys.nix { inherit pkgs; };
  inherit (canokeys) qemu;
  keys = canokeys.keys [ "nano" ];
  guide = pkgs.callPackage ../../../modules/guide/package.nix { };

  # The disk as Asahi's installer leaves a Mac, scaled to a VM: the
  # partitions in the Mac's order and numbering, each of macOS's filled
  # with bytes of its own to be proven untouched, and the boot partition
  # holding what m1n1 and the firmware's copy would. Room enough for the
  # guide's least, 32 GiB, and a qcow2 so only what's written takes space.
  asahi = pkgs.runCommand "asahi-shaped-disk" {
    nativeBuildInputs = [
      pkgs.coreutils
      pkgs.dosfstools
      pkgs.mtools
      pkgs.util-linux
      qemu
    ];
  } ''bash ${./asahi-shaped-disk.sh} "$out"'';
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "guide-beside";
  globalTimeout = 60 * 60;
  qemu.package = qemu;

  nodes.stick = import ./stick.nix { inherit self pkgs; };

  # The machine, as its own repository would describe a Mac.
  nodes.machine =
    { lib, pkgs, ... }:
    {
      imports = [
        self.nixosModules.disko
        self.nixosModules.firmware-settings
        self.nixosModules.guide
      ];
      jitsusama.disk = {
        device = "/dev/disk/by-id/virtio-root";
        swapSize = "64M";
        installKey = "/run/guide/install-key";
        shared = true;
      };
      jitsusama.firmware = {
        secureBoot = false;
        tpm = false;
      };
      # As a Mac's U-Boot, which keeps no variables, has it.
      boot.loader.systemd-boot.enable = true;
      boot.loader.efi.canTouchEfiVariables = false;
      boot.loader.timeout = 0;
      # The disk's FIDO2 slot is opened by systemd in the initrd.
      boot.initrd.systemd.enable = true;
      networking.hostName = "machine";
      users.users.alice = {
        isNormalUser = true;
        uid = 1000;
      };
      environment.systemPackages = [ pkgs.libfido2 ];
      systemd.settings.Manager.ShowStatus = lib.mkForce true;
      system.stateVersion = "26.05";
      virtualisation = {
        directBoot.enable = false;
        mountHostNixStore = false;
        useDefaultFilesystems = false;
        fileSystems = lib.mkForce { };
        useEFIBoot = true;
        # The disk is Asahi's, made before QEMU starts (the test's rig).
        diskSize = 40 * 1024;
        memorySize = 6144;
        qemu.virtioKeyboard = false;
        qemu.options = [
          "-device qemu-xhci,id=xhci"
          "-drive if=none,id=stick,format=qcow2,file=$STICK_IMAGE"
          "-device usb-storage,bus=xhci.0,drive=stick,id=stickdev,bootindex=0"
          "-device canokey,id=key,bus=xhci.0,file=$KEYS_DIR/nano"
          "-device usb-kbd,bus=xhci.0"
        ];
      };
    };

  tests = ./guide-beside-vm-test.py;
  values =
    { nodes, ... }:
    let
      payload = pkgs.callPackage ../../../modules/guide/payload.nix { } {
        config = nodes.machine;
        yubikeys = [ "the Nano" ];
        account = "alice";
      };
      image = "${nodes.stick.system.build.image}/${nodes.stick.image.filePath}";
      written =
        pkgs.runCommand "machine-stick"
          {
            nativeBuildInputs = [
              guide
              pkgs.coreutils
            ];
          }
          ''
            mkdir $out
            room=$(( ($(du -sbL ${payload} | cut -f1) >> 20) + 1 ))
            truncate -s $(( $(stat -c %s ${image}) + (room << 20) + (512 << 20) + 7 * 512 )) $out/stick.raw
            echo ${lib.escapeShellArg passphrase} | guide write-stick ${image} $out/stick.raw --install ${payload}
          '';
    in
    pkgs.writeText "guide-beside-values.json" (
      builtins.toJSON {
        canokeys = "${keys}";
        qemuImg = "${qemu}/bin/qemu-img";
        written = "${written}/stick.raw";
        asahi = "${asahi}";
        inherit
          pin
          password
          passphrase
          ;
      }
    );
}
