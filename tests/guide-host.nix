# A stick written on a Linux host by the guide, as the person signed in
# there and not root: the one USB stick plugged in is named, a partition a
# desktop mounted from it is unmounted, and UDisks2 opens its whole disk,
# which the guide writes with a machine's payload sealed to the recovery
# passphrase typed twice, then reads back. polkit lets the person open a
# removable disk without the password a desktop's agent would ask for,
# since a test has no agent to answer it.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  passphrase = "the stick's recovery passphrase";
  guide = pkgs.callPackage ../modules/guide/package.nix { };
  image = "${self.packages.x86_64-linux.stick}/stick_1.raw";
  # The machine the stick installs, small, as its repository would say it.
  machine =
    (pkgs.nixos {
      imports = [ self.nixosModules.disko ];
      jitsusama.disk = {
        device = "/dev/disk/by-id/nvme-the-machines-disk";
        swapSize = "1G";
        installKey = "/run/guide/install-key";
      };
      networking.hostName = "laptop";
      boot.loader.systemd-boot.enable = true;
      system.stateVersion = "26.05";
    }).config;
  payload = pkgs.callPackage ../modules/guide/payload.nix { } {
    config = machine;
    yubikeys = [ "the Nano" ];
    account = "alice";
  };
in
pkgs.testers.runNixOSTest {
  name = "guide-host";

  nodes.host = {
    services.udisks2.enable = true;
    users.users.alice = {
      isNormalUser = true;
      uid = 1000;
    };
    security.polkit.extraConfig = ''
      polkit.addRule(function(action, subject) {
        if (subject.user == "alice" && (
            action.id == "org.freedesktop.udisks2.open-device" ||
            action.id == "org.freedesktop.udisks2.filesystem-mount")) {
          return polkit.Result.YES;
        }
      });
    '';
    services.getty.autologinUser = "alice";
    environment.loginShellInit = ''
      if [ "$(tty)" = /dev/tty1 ]; then
        ${lib.getExe guide} host ${image} --install ${payload}
      fi
    '';
    environment.systemPackages = [
      pkgs.dosfstools
      pkgs.gptfdisk
    ];
    virtualisation = {
      memorySize = 2048;
      qemu.options = [
        "-device qemu-xhci,id=xhci"
        "-drive if=none,id=stick,format=qcow2,file=$STICK_IMAGE"
        "-device usb-storage,bus=xhci.0,drive=stick"
      ];
    };
  };

  testScript = ''
    import json, os, re, subprocess, tempfile

    stick = tempfile.NamedTemporaryFile(suffix=".qcow2")
    subprocess.run(["${pkgs.qemu}/bin/qemu-img", "create", "-f", "qcow2", stick.name, "6G"], check=True)
    os.environ["STICK_IMAGE"] = stick.name

    def on_screen(pattern, timeout=120):
        def shown(_):
            text = host.get_tty_text("1")
            if re.search(r"(?i)(try again|look again)", text):
                raise Exception("the guide stopped:\n" + text)
            return re.search("(?i)" + pattern, text) is not None
        with host.nested(f"waiting for {pattern} on the guide's screen"):
            retry(shown, timeout)

    def type(text):
        host.send_chars(text + "\n")

    host.wait_for_unit("multi-user.target")

    with subtest("the stick, once used, has a partition the desktop mounted"):
        host.succeed("sgdisk --new=1:0:+64M --typecode=1:0700 /dev/sda && udevadm settle")
        host.succeed("mkfs.vfat /dev/sda1")
        host.succeed("su alice -c 'udisksctl mount --block-device /dev/sda1'")
        host.succeed("findmnt /dev/sda1")

    with subtest("the one stick is named, by what it is"):
        on_screen("Write laptop's stick")
        on_screen("is the stick to write")
        type("")

    with subtest("the recovery passphrase is typed twice"):
        on_screen("recovery passphrase for the stick's vault")
        type(${builtins.toJSON passphrase})
        on_screen("the same passphrase again")
        type(${builtins.toJSON passphrase})

    with subtest("the stick is written as the person, and read back"):
        on_screen("The stick is written", timeout=900)
        host.fail("findmnt /dev/sda1")
        # Written by alice's guide, never as root.
        host.succeed("pgrep -u alice -f 'guide host'")
        host.fail("pgrep -u root -f 'guide host'")
        verified = host.succeed("sfdisk --verify /dev/sda")
        assert "No errors detected" in verified, verified
        # From the disk itself, not udev's record of what was there before.
        listed = json.loads(host.succeed("sfdisk --json /dev/sda"))
        names = [p.get("name") for p in listed["partitiontable"]["partitions"]]
        assert names == ["esp", "store-verity", "store", "vault"], names
  '';
}
