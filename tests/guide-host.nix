# A stick written on a Linux host by the guide, as the person signed in
# there and not root: the one USB stick plugged in is named, a partition a
# desktop mounted from it is unmounted, and UDisks2 opens its whole disk,
# which the guide writes with a machine's payload sealed to a passphrase of
# seven words it made, shown alone for a phone and typed back, then reads
# back. The rescue bundles the old stick carried, read from its vault
# mounted only for reading before it's written over, and the host's own
# machine's, go onto the new stick and stay with the person. polkit lets the person open a
# removable disk without the password a desktop's agent would ask for,
# since a test has no agent to answer it.
{ self, pkgs }:
let
  inherit (pkgs) lib;
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
  # Rescue bundles as far as a host reads one it carries on: which
  # machine's it is, and when it was sealed.
  bundle =
    machine:
    pkgs.writeText "${machine}-rescue-bundle.json" (
      builtins.toJSON {
        format = 1;
        inherit machine;
        opened_by = [ "the Nano" ];
        owner = "22f0175f-c166-49d2-96ab-c61575cddac5";
        db = "ab";
        to_keys = "a";
        to_passphrase = "b";
        keys = "c";
        sealed = 1761100000;
      }
    );
  # A stick used before: a partition of the person's, and a vault carrying
  # another machine's bundle.
  used =
    pkgs.runCommand "used-stick"
      {
        nativeBuildInputs = [
          pkgs.dosfstools
          pkgs.gptfdisk
          pkgs.mtools
        ];
      }
      ''
        mkdir $out
        dd if=/dev/zero of=$out/stick.raw bs=1M count=160
        sgdisk --new=1:2048:+64M --typecode=1:0700 --change-name=1:data \
          --new=2:133120:+64M --typecode=2:0700 --change-name=2:vault $out/stick.raw
        mkfs.vfat -C data.fat 65536
        mkfs.vfat -C vault.fat 65536
        MTOOLS_SKIP_CHECK=1 mcopy -i vault.fat ${bundle "desktop"} ::desktop-rescue-bundle.json
        dd if=data.fat of=$out/stick.raw bs=512 seek=2048 conv=notrunc
        dd if=vault.fat of=$out/stick.raw bs=512 seek=133120 conv=notrunc
      '';
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
    # The bundle this host's own install left for anyone to read.
    systemd.tmpfiles.rules = [
      "d /var/lib/guide 0755 root root -"
      "C /var/lib/guide/rescue-bundle.json 0644 root root - ${bundle "tower"}"
    ];
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
    subprocess.run(["${pkgs.qemu}/bin/qemu-img", "create", "-f", "qcow2", "-b", "${used}/stick.raw", "-F", "raw", stick.name, "6G"], check=True)
    os.environ["STICK_IMAGE"] = stick.name

    def screen():
        return host.get_tty_text("1")

    def on_screen(pattern, timeout=120):
        def shown(_):
            text = screen()
            if re.search(r"(?i)(try again|look again)", text):
                raise Exception("the guide stopped:\n" + text)
            return re.search("(?i)" + pattern, text) is not None
        with host.nested(f"waiting for {pattern} on the guide's screen"):
            retry(shown, timeout)

    def type(text):
        host.send_chars(text + "\n")

    host.wait_for_unit("multi-user.target")

    with subtest("the stick, once used, has a partition the desktop mounted"):
        host.succeed("udevadm settle")
        host.succeed("su alice -c 'udisksctl mount --block-device /dev/sda1'")
        host.succeed("findmnt /dev/sda1")

    with subtest("the one stick is named, by what it is"):
        on_screen("Write laptop's stick")
        on_screen("is the stick to write")
        type("")

    with subtest("the passphrase is seven words shown alone for a phone, then typed back"):
        on_screen("Copy All")
        type("")
        retry(lambda _: len(screen().split()) == 7, 30)
        words = " ".join(screen().split())
        type("")
        on_screen("reading it off your phone")
        # Another's words are refused and left to be put right, and
        # nothing is written.
        other = "abacus abdomen abdominal abide abiding ability ablaze"
        type(other)
        on_screen("that isn't it")
        assert other in screen(), screen()
        host.fail("sfdisk --json /dev/sda | grep -q store-verity")
        for _ in other:
            host.send_key("backspace")
        # The words are read whatever their case.
        type(words.upper())

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

    with subtest("the old stick's bundles and the host's own are kept, and carried on"):
        host.succeed("udevadm settle && mkdir -p /run/vault && mount -o ro /dev/sda4 /run/vault")
        kept = "/home/alice/.local/state/guide/rescue"
        for machine, bundle in [("desktop", "${bundle "desktop"}"), ("tower", "${bundle "tower"}")]:
            host.succeed(f"cmp {bundle} /run/vault/{machine}-rescue-bundle.json")
            host.succeed(f"cmp {bundle} {kept}/{machine}-rescue-bundle.json")
        host.succeed(f"test $(stat -c %U {kept}) = alice")
        host.succeed("umount /run/vault")
        # The old vault was mounted for alice to be read, not its disk
        # opened, and unmounted again: UDisks2 says so only as it takes
        # the mount point away.
        host.succeed("journalctl -u udisks2 --grep 'Cleaning up mount point /run/media/alice/.* \\(device 8:2 is not mounted\\)'")
  '';
}
