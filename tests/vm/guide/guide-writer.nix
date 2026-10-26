# A stick written on Linux by the guide, as the person signed in
# there and not root: the one USB stick plugged in is named, a partition a
# desktop mounted from it is unmounted, and UDisks2 opens its whole disk,
# which the guide writes with a machine's payload sealed to a passphrase of
# seven words it made, shown alone for a phone and typed back, then reads
# back. The rescue bundles the old stick carried, read from its vault
# mounted only for reading before it's written over, and the writer's own
# machine's, go onto the new stick and stay with the person. The stick is
# the writer's own machine's, so the bundle it keeps for itself, sealed to a
# real FIDO2 key, is opened with that key's PIN and signs the stick with
# the machine's db key. The key is a canokey (canokeys.nix), its PIN and
# recipient made at boot as a person makes them (make-recipient.py).
# Written over again, the stick keeps the passphrase its vault has: the
# person types it back, proven against the old vault, and no new words are
# made.
# polkit lets the person open a removable disk without the password a
# desktop's agent would ask for, since a test has no agent to answer it.
# The tests are guide-writer-vm-test.py, run through pytest.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  guide = pkgs.callPackage ../../../modules/guide/package.nix { };
  # One key, made by QEMU at its start, standing in for the Nano.
  inherit (import ./canokeys.nix { inherit pkgs; }) qemu setPin;
  pin = "123456";
  # A recipient made as a person makes one, the plugin's questions
  # answered as its README says.
  makeRecipient = pkgs.writers.writePython3Bin "make-recipient" { doCheck = false; } (
    builtins.replaceStrings [ "@plugin@" ] [ (lib.getExe pkgs.age-plugin-fido2-hmac) ] (
      builtins.readFile ./make-recipient.py
    )
  );
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
  payload = pkgs.callPackage ../../../modules/guide/payload.nix { } {
    config = machine;
    yubikeys = [ "the Nano" ];
    account = "alice";
  };
  # What opens the bundle the writer keeps for itself.
  bundlePassphrase = "the writer's own bundle's passphrase";
  # Rescue bundles as far as a writer reads one it carries on: which
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
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "guide-writer";
  qemu.package = qemu;

  nodes.writer = {
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
    # The writer is the machine the stick is for, and keeps its bundle as
    # its install, or `guide seal-keys`, leaves one for anyone to read,
    # before the person signs in.
    systemd.services.seal-own-bundle = {
      wantedBy = [ "multi-user.target" ];
      before = [ "getty@tty1.service" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
      };
      path = [
        pkgs.libfido2
        pkgs.systemd
      ];
      script = ''
        ${lib.getExe pkgs.sbctl} create-keys --export /var/lib/sbctl/keys \
          --database-path /var/lib/sbctl/GUID
        mkdir -p /var/lib/guide
        # The key, once udev has it, given its PIN and a recipient, which
        # the machine's target names as the Nano's.
        for _ in $(seq 60); do
          udevadm settle
          [ -n "$(fido2-token -L)" ] && break
          sleep 1
        done
        ${lib.getExe setPin} ${pin}
        recipient=$(${lib.getExe makeRecipient} ${pin})
        ${lib.getExe pkgs.jq} --arg recipient "$recipient" '.recipients = [$recipient]' \
          ${payload}/target.json > /run/target.json
        echo ${lib.escapeShellArg bundlePassphrase} |
          ${lib.getExe guide} seal-keys /run/target.json /var/lib/guide/rescue-bundle.json
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
      # The same write again, for the stick written once to be written
      # over, keeping its passphrase.
      (pkgs.writeShellScriptBin "write-the-stick-again" ''
        exec ${lib.getExe guide} host ${image} --install ${payload}
      '')
    ];
    virtualisation = {
      memorySize = 2048;
      qemu.options = [
        "-device qemu-xhci,id=xhci"
        "-device canokey,id=key,bus=xhci.0,file=/tmp/canokey-writer"
        "-drive if=none,id=stick,format=qcow2,file=$STICK_IMAGE"
        "-device usb-storage,bus=xhci.0,drive=stick"
      ];
    };
  };

  # The driver runs beside the machine, not on it, so what the tests need
  # from Nix, the used stick, the tool to put an overlay on it, the old
  # stick's bundle and the passphrase of the writer's own, reaches them as
  # their values fixture rather than on the machine's path.
  tests = ./guide-writer-vm-test.py;
  values =
    let
      values = pkgs.writeText "guide-writer-values.json" (
        builtins.toJSON {
          qemuImg = "${pkgs.qemu}/bin/qemu-img";
          used = "${used}/stick.raw";
          desktopBundle = "${bundle "desktop"}";
          inherit bundlePassphrase pin;
        }
      );
    in
    values;
}
