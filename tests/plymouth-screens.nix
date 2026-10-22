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
# after is left to the boot; either way the layout must cover both.
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

  nodes.machine =
    { lib, pkgs, ... }:
    {
      imports = [ self.nixosModules.graphical ];
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
      boot.initrd.kernelModules = [ "virtio_gpu" ];
      # The test reads the machine through a serial console, and Plymouth
      # keeps to text where there's one unless told it needn't.
      boot.kernelParams = [ "plymouth.ignore-serial-consoles" ];

      jitsusama.login.account = account;
      users.users.${account}.isNormalUser = true;
      home-manager.users.${account}.home.stateVersion = "26.05";

      specialisation.encrypted.configuration = {
        boot.initrd.luks.devices = lib.mkVMOverride {
          root.device = "/dev/vdb";
        };
        virtualisation.rootDevice = "/dev/mapper/root";
      };
    };

  testScript =
    { nodes, ... }:
    let
      encrypted = nodes.machine.specialisation.encrypted.configuration.system.build.toplevel;
    in
    ''
      import os
      import subprocess

      # Each screen photographed, the first as the driver does and the
      # second by its device's name.
      def photographed(name, device=None):
          path = os.path.join(machine.out_dir, f"{name}.png")
          if device is None:
              machine.screenshot(name)
              return path
          if os.path.exists(path):
              os.unlink(path)
          said = machine.send_monitor_command(f"screendump {path} -f png {device}")
          # QEMU writes the picture after it answers, so it's waited for
          # until it reads whole.
          def written(_):
              return subprocess.run(
                  ["${pkgs.imagemagick}/bin/magick", "identify", path], capture_output=True
              ).returncode == 0
          retry(written, 10)
          print(said)
          return path

      # The words in a screen's bottom left quarter, enlarged for OCR, which
      # can't read the line at its own small size.
      def bottom_left(path, width, height):
          crop = f"{width // 2}x{height // 4}+0+{height - height // 4}"
          enlarged = path.replace(".png", "-bottom-left.png")
          subprocess.run([
              "${pkgs.imagemagick}/bin/magick", path, "-crop", crop, "+repage",
              "-resize", "300%", "-colorspace", "Gray", "-negate", enlarged,
          ], check=True)
          return subprocess.check_output(
              ["${pkgs.tesseract}/bin/tesseract", enlarged, "-"], text=True
          )

      machine.wait_for_unit("multi-user.target")
      machine.succeed(
          "echo -n ${passphrase} | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -",
          "echo -n ${passphrase} | cryptsetup luksOpen -q /dev/vdb root",
          "mkfs.ext4 -q /dev/mapper/root",
          "${encrypted}/bin/switch-to-configuration boot",
          "sync",
      )
      machine.crash()
      machine.start()
      machine.wait_for_text("disk is locked", timeout=120)

      with subtest("the smaller screen shows the whole ask, its edges too"):
          def whole_on_small(_):
              said = bottom_left(photographed("small"), ${toString small.width}, ${toString small.height})
              return "Linux" in said
          retry(whole_on_small, 60)

      with subtest("the larger screen shows the whole ask too"):
          def whole_on_large(_):
              shot = photographed("large", "large")
              said = bottom_left(shot, ${toString large.width}, ${toString large.height})
              text = subprocess.check_output(
                  ["${pkgs.tesseract}/bin/tesseract", shot, "-"], text=True
              )
              return "Linux" in said and "locked" in text
          retry(whole_on_large, 60)

      with subtest("the passphrase still opens the disk"):
          machine.send_chars("${passphrase}\n")
          machine.wait_for_unit("multi-user.target")
          t.assertIn("/dev/mapper/root on / ", machine.succeed("mount"))
    '';
}
