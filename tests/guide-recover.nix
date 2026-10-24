# A machine recovered from a repair stick, its data left in place: the
# stick, written by the guide with nothing of the machine's on it, boots
# into the guide, which finds the machine's disk by core's layout, opens it
# with a YubiKey's PIN, mounts every subvolume and the ESP, sets which
# generation boots next, replaces a lost YubiKey, and hands over a root
# shell inside the installed system. The machine's disk is one disko's
# image builder laid out and installed, as vm-secure-boot-installs builds
# its own; its two YubiKeys are enrolled from the stick before the guide
# starts, as a machine's own install leaves them. The script is
# guide-recover-vm-test.py.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  # Standing in for the recovery key: what opens the disk's passphrase slot
  # is what the recovery key's slot takes, typed text.
  recovery = "the machines recovery key";
  pin = "123456";
  guide = pkgs.callPackage ../modules/guide/package.nix { };

  # canokeys as tests/guide-install.nix makes them: one device swapped
  # between state files made before QEMU starts, each its own randomness.
  canokey = pkgs.canokey-qemu.overrideAttrs (old: {
    postPatch = old.postPatch + ''
      cat >> Src/qemu.c <<'EOF'

      #include <sys/random.h>
      uint32_t random32(void) {
        uint32_t r;
        if (getrandom(&r, sizeof(r), 0) != sizeof(r)) abort();
        return r;
      }
      EOF
    '';
  });
  qemu = pkgs.qemu_test.override {
    canokeySupport = true;
    canokey-qemu = canokey;
  };
  keys = pkgs.runCommand "canokey-keys" { nativeBuildInputs = [ qemu ]; } ''
    mkdir $out
    for name in kept lost new; do
      echo quit | qemu-system-x86_64 -machine q35 -accel tcg -nodefaults -display none \
        -S -monitor stdio -device qemu-xhci,id=usb -device canokey,bus=usb.0,file=$out/$name
      test -s $out/$name
    done
  '';
  setPin = pkgs.writers.writePython3Bin "set-pin" { libraries = [ pkgs.python3Packages.fido2 ]; } ''
    import sys
    from fido2.hid import CtapHidDevice
    from fido2.ctap2 import Ctap2, ClientPin

    device = next(iter(CtapHidDevice.list_devices()))
    ClientPin(Ctap2(device)).set_pin(sys.argv[1])
  '';
in
pkgs.testers.runNixOSTest {
  name = "guide-recover";
  globalTimeout = 30 * 60;
  qemu.package = qemu;

  # The stick's system, built as a test node so the driver can reach its
  # shell, and never started itself: the machine boots its image from USB.
  nodes.stick = {
    imports = [ self.nixosModules.guide-stick ];
    environment.systemPackages = [ setPin ];
    virtualisation = {
      fileSystems = lib.mkVMOverride { };
      directBoot.enable = false;
      mountHostNixStore = false;
      useEFIBoot = true;
    };
  };

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
      # which current nixpkgs refuses (disko#1298), as tests/secure-boot.nix
      # says.
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
          "-device canokey,id=key,bus=xhci.0,file=$KEYS_DIR/kept"
          "-device usb-kbd,bus=xhci.0"
        ];
      };
    };

  # The driver runs on the host, so what the script needs from Nix, the
  # images to put overlays on, the tool to make them, the keys' state files
  # and the secrets to type, reaches it in one file rather than on the
  # machine's path.
  testScript =
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
          keys = "${keys}";
          inherit pin recovery;
        }
      );
      script = builtins.readFile ./guide-recover-vm-test.py;
    in
    builtins.replaceStrings [ "@values@" ] [ "${values}" ] script;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
