# Boots the graphical role from an encrypted disk in a virtual machine with
# two screens of different sizes, as a laptop's panel and a monitor are, and
# checks the theme's ask shows whole on both. Plymouth draws one canvas as
# large as the largest screen and shows each screen the part of it centred
# on its size, so a layout made for the largest screen, or for whichever
# came first, leaves the smaller screen without its edges. The bottom left's
# line saying what's booting is the first thing lost, so it's read from each
# screen, by the word Linux in it, which OCR reads more surely than NixOS,
# whose O it takes for a nought. A screen coming while the splash is up can't be made to happen on
# cue here, so whether the second screen comes before the theme starts or
# after is left to the boot; either way the layout must cover both. The
# script is plymouth-screens-vm-test.py.
{ self, pkgs }:
let
  account = "jitsusama";
  passphrase = "4815";
  # The first is the console the test driver reads; the second is larger in
  # both directions, so the first is the one the canvas is cropped for.
  small = {
    width = 1280;
    height = 800;
  };
  large = {
    width = 1920;
    height = 1200;
  };
in
pkgs.testers.runNixOSTest {
  name = "plymouth-screens";
  enableOCR = true;
  # The base role sets nixpkgs' own settings.
  node.pkgsReadOnly = false;

  # The first boot only lays the encrypted disk out and hands over, so it
  # carries little, since useBootLoader copies its whole closure into the
  # boot disk's image; the graphical system booted from the encrypted disk is
  # evaluated from it as a specialisation would be but kept out of that
  # closure, as tests/plymouth.nix does.
  nodes.machine =
    {
      extendModules,
      lib,
      pkgs,
      ...
    }:
    {
      environment.systemPackages = [ pkgs.cryptsetup ];

      virtualisation = {
        memorySize = 3072;
        cores = 4;
        qemu.options = [
          "-vga none"
          "-device virtio-gpu-pci,xres=${toString small.width},yres=${toString small.height}"
          "-device virtio-gpu-pci,id=large,xres=${toString large.width},yres=${toString large.height}"
        ];
        emptyDiskImages = [ 512 ];
        useBootLoader = true;
        useEFIBoot = true;
        # Booting off the encrypted disk needs the store from the host.
        mountHostNixStore = true;
      };
      boot.loader.systemd-boot.enable = true;
      boot.initrd.systemd.enable = true;

      system.build.encrypted =
        (extendModules {
          modules = [
            {
              imports = [ self.nixosModules.graphical ];
              boot.initrd.kernelModules = [ "virtio_gpu" ];
              # The test reads the machine through a serial console, and Plymouth
              # keeps to text where there's one unless told it needn't.
              boot.kernelParams = [ "plymouth.ignore-serial-consoles" ];

              jitsusama.login.account = account;
              users.users.${account}.isNormalUser = true;
              home-manager.users.${account}.home.stateVersion = "26.05";

              boot.initrd.luks.devices = lib.mkVMOverride {
                root.device = "/dev/vdb";
              };
              virtualisation.rootDevice = "/dev/mapper/root";
            }
          ];
        }).config.system.build.toplevel;
    };

  testScript =
    { nodes, ... }:
    let
      inherit (nodes.machine.system.build) encrypted;
      # What the tests need from Nix, as their values fixture. ImageMagick and Tesseract run beside the driver, not on the
      # machine, so they come as paths rather than on its PATH.
      values = pkgs.writeText "plymouth-screens-values.json" (
        builtins.toJSON {
          inherit passphrase small large;
          encrypted = "${encrypted}";
          magick = "${pkgs.imagemagick}/bin/magick";
          tesseract = "${pkgs.tesseract}/bin/tesseract";
        }
      );
    in
    ''
      import vm_pytest
      vm_pytest.run(globals(), "${./plymouth-screens-vm-test.py}", values="${values}")
    '';
  # What VM tests share (tests/lib), pytest and its runner among them.
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
