# A stick written on a Linux host by the guide, as the person signed in
# there and not root: the one USB stick plugged in is named, a partition a
# desktop mounted from it is unmounted, and UDisks2 opens its whole disk,
# which the guide writes with a machine's payload sealed to a passphrase of
# seven words it made, shown alone for a phone and typed back, then reads
# back. The rescue bundles the old stick carried, read from its vault
# mounted only for reading before it's written over, and the host's own
# machine's, go onto the new stick and stay with the person. The stick is
# the host's own machine's, so the bundle it keeps for itself is opened
# with its passphrase and signs the stick with the machine's db key.
# polkit lets the person open a removable disk without the password a
# desktop's agent would ask for, since a test has no agent to answer it.
# The script is guide-host-vm-test.py.
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
  # What opens the bundle the host keeps for itself.
  bundlePassphrase = "the host's own bundle's passphrase";
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
    # The host is the machine the stick is for, and keeps its bundle as
    # its install, or `guide seal-keys`, leaves one for anyone to read,
    # before the person signs in.
    systemd.services.seal-own-bundle = {
      wantedBy = [ "multi-user.target" ];
      before = [ "getty@tty1.service" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
      };
      script = ''
        ${lib.getExe pkgs.sbctl} create-keys --export /var/lib/sbctl/keys \
          --database-path /var/lib/sbctl/GUID
        mkdir -p /var/lib/guide
        echo ${lib.escapeShellArg bundlePassphrase} |
          ${lib.getExe guide} seal-keys ${payload}/target.json /var/lib/guide/rescue-bundle.json
        chmod 0644 /var/lib/guide/rescue-bundle.json
      '';
    };
    services.getty.autologinUser = "alice";
    environment.loginShellInit = ''
      if [ "$(tty)" = /dev/tty1 ]; then
        ${lib.getExe guide} host ${image} --install ${payload}
      fi
    '';
    environment.systemPackages = [
      pkgs.dosfstools
      pkgs.gptfdisk
      pkgs.sbsigntool
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

  # The driver runs beside the machine, not on it, so what the script needs
  # from Nix, the used stick, the tool to put an overlay on it, the old
  # stick's bundle and the passphrase of the host's own, reaches it in one
  # file rather than on the machine's path.
  testScript =
    let
      values = pkgs.writeText "guide-host-values.json" (
        builtins.toJSON {
          qemuImg = "${pkgs.qemu}/bin/qemu-img";
          used = "${used}/stick.raw";
          desktopBundle = "${bundle "desktop"}";
          inherit bundlePassphrase;
        }
      );
      script = builtins.readFile ./guide-host-vm-test.py;
    in
    builtins.replaceStrings [ "@values@" ] [ "${values}" ] script;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
