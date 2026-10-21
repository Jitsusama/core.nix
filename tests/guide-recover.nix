# A machine recovered from a repair stick, its data left in place: the
# stick, written by the guide with nothing of the machine's on it, boots
# into the guide, which finds the machine's disk by core's layout, opens it
# with the recovery key, mounts every subvolume and the ESP, and hands over
# a root shell inside the installed system. The machine's disk is one
# disko's image builder laid out and installed, as vm-secure-boot-installs
# builds its own.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  # Standing in for the recovery key: what opens the disk's passphrase slot
  # is what the recovery key's slot takes, typed text.
  recovery = "the machines recovery key";
  guide = pkgs.callPackage ../modules/guide/package.nix { };
in
pkgs.testers.runNixOSTest {
  name = "guide-recover";
  globalTimeout = 30 * 60;

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
          "-device usb-kbd,bus=xhci.0"
        ];
      };
    };

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
    in
    ''
      import os, re, subprocess, tempfile

      disk = tempfile.NamedTemporaryFile(suffix=".qcow2")
      subprocess.run([
          "${nodes.machine.virtualisation.qemu.package}/bin/qemu-img", "create", "-f", "qcow2",
          "-b", "${nodes.machine.system.build.diskoImages}/main.qcow2", "-F", "qcow2", disk.name,
      ], check=True)
      os.environ["NIX_DISK_IMAGE"] = disk.name
      # The stick writes to an overlay, since the image is in the store.
      stick = tempfile.NamedTemporaryFile(suffix=".qcow2")
      subprocess.run([
          "${nodes.machine.virtualisation.qemu.package}/bin/qemu-img", "create", "-f", "qcow2",
          "-b", "${written}/stick.raw", "-F", "raw", stick.name,
      ], check=True)
      os.environ["STICK_IMAGE"] = stick.name

      def screen():
          return machine.get_tty_text("1")

      def on_screen(pattern, timeout=120):
          def shown(_):
              text = screen()
              if re.search(r"(?i)(try again|look again)", text):
                  raise Exception("the guide stopped:\n" + text)
              return re.search("(?i)" + pattern, text) is not None
          with machine.nested(f"waiting for {pattern} on the guide's screen"):
              try:
                  retry(shown, timeout)
              except Exception:
                  print("SCREEN\n" + screen())
                  print("LOG\n" + machine.execute("tail -n 40 /run/guide/log")[1])
                  raise

      def type(text):
          machine.send_chars(text + "\n")

      machine.start()

      with subtest("a repair stick boots into the guide's recovery"):
          on_screen("Recover a machine")
          on_screen("Check the keyboard")
          type("keyboard")

      with subtest("the machine's disk is found by its layout and opened with the recovery key"):
          on_screen("holds the machine")
          t.assertIn("named system", screen())
          type("")
          on_screen("Plug in a YubiKey")
          type("")
          on_screen("the recovery key")
          type("${recovery}")

      with subtest("everything the machine mounts is mounted under /mnt"):
          on_screen("disk is open")
          for at in ["/mnt", "/mnt/nix", "/mnt/home", "/mnt/var/log", "/mnt/boot"]:
              machine.succeed(f"findmnt {at}")
          machine.succeed("test -e /mnt/etc/NIXOS")
          # What bootctl lists from outside the machine, for the guide's
          # fixtures (qemu-canokey).
          listed = machine.succeed("bootctl --esp-path=/mnt/boot list --json=short")
          print("FIXTURE bootctl-list-from-the-stick.json " + listed.replace("\n", "\\n"))

      with subtest("a generation chosen is set to boot next, and the menu comes back"):
          on_screen("What should be done")
          machine.send_key("down")
          type("")
          on_screen("Which should it boot next")
          t.assertIn("Generation 1", screen())
          type("")
          on_screen("What should be done")
          t.assertIn("boots generation 1 next", screen())
          # The firmware's own default, which systemd-boot reads first.
          machine.succeed("test -e /sys/firmware/efi/efivars/LoaderEntryDefault-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f")

      with subtest("Return gives a root shell inside the installed system"):
          machine.send_key("up")
          type("")
          machine.sleep(3)
          type("cat /etc/hostname > /recovered && nixos-version >> /recovered")
          machine.wait_until_succeeds("test -s /mnt/recovered", timeout=30)
          said = machine.succeed("cat /mnt/recovered")
          print(said)
          t.assertIn("machine", said.splitlines()[0])
    '';
}
