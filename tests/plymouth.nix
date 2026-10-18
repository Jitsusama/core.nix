# Boots the graphical role from an encrypted disk in a virtual machine and
# answers the disk's passphrase on the keyboard, as Joel types his PIN: the
# theme's screen asks for it, shows a dot for each key, says try again after
# a wrong one in the alert frame, then lets the boot through. The test reads
# the screen with OCR, so each step waits on what's drawn rather than on a
# timer, and photographs each state for a person to judge.
{ self, pkgs }:
let
  account = "jitsusama";
  passphrase = "4815";
in
pkgs.testers.runNixOSTest {
  name = "plymouth";
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
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
        emptyDiskImages = [ 512 ];
        useBootLoader = true;
        useEFIBoot = true;
        # Booting off the encrypted disk needs the store from the host.
        mountHostNixStore = true;
      };
      boot.loader.systemd-boot.enable = true;
      boot.initrd.systemd.enable = true;
      # The GPU's driver in the initrd, as a machine's hardware module puts
      # it, so Plymouth has a screen to draw on before the disk opens.
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
      alert = nodes.machine.home-manager.users.${account}.jitsusama.theme.roles.alert;
      inherit (nodes.machine.console) font;
    in
    ''
      import filecmp
      import os
      import subprocess

      # A photograph of the screen, kept for a person to judge.
      def photographed(name):
          machine.screenshot(name)
          return os.path.join(machine.out_dir, f"{name}.png")

      # How many of a photograph's pixels are the theme's alert colour,
      # which only the frame of a refused secret is drawn in. The muted
      # hint inside it is too faint on the dark panel for OCR to read.
      def alerting(path):
          return int(subprocess.check_output([
              "${pkgs.imagemagick}/bin/magick", path, "-fuzz", "4%",
              "-fill", "black", "+opaque", "${alert}",
              "-fill", "white", "-opaque", "${alert}",
              "-format", "%[fx:round(mean*w*h)]", "info:",
          ], text=True))

      # The frame is two panels' widths and two heights of hairline, so a
      # few hundred pixels at the least.
      framed = 400

      # Waits for the screen to differ from a photograph, as it does once
      # Plymouth has drawn what a key changed.
      def changed_from(before, name, timeout=30):
          def differs(_):
              return not filecmp.cmp(before, photographed(name), shallow=False)
          retry(differs, timeout)

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

      with subtest("the theme asks for the disk's passphrase"):
          machine.wait_for_text("disk is locked", timeout=120)
          asked = photographed("asks")
          t.assertLess(alerting(asked), framed // 10, "the first ask is framed in alert")

      with subtest("each key shows as a dot"):
          machine.send_chars("48")
          changed_from(asked, "typing")
          for _ in range(2):
              machine.send_key("backspace")

      with subtest("a wrong one is refused in the alert frame"):
          machine.send_chars("0000\n")
          retry(lambda _: alerting(photographed("wrong")) >= framed, 60)

      with subtest("the right one opens the disk and the boot goes on"):
          machine.send_chars("${passphrase}\n")
          machine.wait_for_unit("multi-user.target")
          t.assertIn("/dev/mapper/root on / ", machine.succeed("mount"))

      with subtest("the console draws in the theme's face"):
          # The glyphs the kernel holds are the font's own, bitmap for
          # bitmap, as many as its header counts at its size, even when
          # the console it's switched from has lost the face, as a splash
          # that outlasts the graphics driver leaves it: a new console
          # takes that one's font until its login prompt sets its own.
          machine.succeed("setfont -C /dev/tty1 default8x16")
          machine.send_key("ctrl-alt-f3")
          machine.wait_until_tty_matches("3", "login: ")
          machine.succeed(
              "setfont -C /dev/tty3 -O /tmp/held.psf",
              "word() { od -An -t u4 -j $(( $2 * 4 )) -N 4 $1 | tr -d ' '; }; "
              "cmp -n $(( $(word ${font} 4) * $(word ${font} 5) )) "
              "<(tail -c +33 /tmp/held.psf) <(tail -c +33 ${font})",
          )
          photographed("console")
    '';
}
