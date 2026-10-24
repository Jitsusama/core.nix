# A machine installed from its stick by the guide, driven by the keys a
# person would press and read off its console, as a laptop is: the stick
# written by `guide write-stick --install` with the machine's payload,
# plugged in by USB and booted first, two FIDO2 keys enrolled by name, a
# recovery key typed back, Secure Boot keys made and the system installed
# from the vault with no network. Then the stick comes out, and the machine
# boots its own disk, opened by a key's PIN, and enrols its Secure Boot keys.
# By then the stick is the machine's own, carrying its keys and signed with
# them, so it boots again with Secure Boot enforced and reinstalls the
# machine with its keys put back, the firmware never visited. Once the disk
# is sealed to the TPM, the guide makes the account's SSH keys there too.
#
# One machine plays the whole of it, as a laptop does, since lanzaboote
# predicts the TPM's policy as it installs and the stick runs on the
# machine's own TPM. Its firmware starts in setup mode.
#
# The FIDO2 keys are canokeys, one QEMU device swapped between two state
# files, made before QEMU starts (spikes in QEST-20261011-2HN63P): canokey
# keeps one global state, so a second device crashes it, a state file made
# on a hot-plug answers every makeCredential with 0xF1, and its random
# numbers come from a fixed seed, so every key it made would hold the same
# secrets. A getrandom-backed random32 makes each key its own. The script
# is guide-install-vm-test.py.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  pin = "123456";
  tpmPin = "24681357";
  # Spaces inside, which ssh-tpm-keygen keeps.
  keyPin = "the keys PIN";
  password = "a password for alice";
  passphrase = "the stick's recovery passphrase";
  # Typed into a shell as it is, so a word with nothing to quote.
  bundlePassphrase = "the-bundles-recovery-passphrase";

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
    for name in nano fivec; do
      echo quit | qemu-system-x86_64 -machine q35 -accel tcg -nodefaults -display none \
        -S -monitor stdio -device qemu-xhci,id=usb -device canokey,bus=usb.0,file=$out/$name
      test -s $out/$name
    done
  '';
  # A fresh key has no PIN, and the guide enrols with one, as a person's
  # YubiKeys have.
  setPin =
    pkgs.writers.writePython3Bin "set-pin"
      {
        libraries = [ pkgs.python3Packages.fido2 ];
      }
      ''
        import sys
        from fido2.hid import CtapHidDevice
        from fido2.ctap2 import Ctap2, ClientPin

        device = next(iter(CtapHidDevice.list_devices()))
        ClientPin(Ctap2(device)).set_pin(sys.argv[1])
      '';
  guide = pkgs.callPackage ../modules/guide/package.nix { };
in
pkgs.testers.runNixOSTest {
  name = "guide-install";
  globalTimeout = 60 * 60;
  qemu.package = qemu;

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
    environment.systemPackages = [ setPin ];
  };

  # The machine, as its own repository would describe it.
  nodes.machine =
    { lib, pkgs, ... }:
    {
      imports = [
        self.nixosModules.disko
        self.nixosModules.guide
        self.nixosModules.lanzaboote
      ];
      jitsusama.disk = {
        device = "/dev/disk/by-id/virtio-root";
        swapSize = "64M";
        installKey = "/run/guide/install-key";
      };
      # OVMF runs its network card's option ROM, so the keys carry its
      # checksum, as a machine with a ROM of its own sets them to.
      boot.lanzaboote.autoEnrollKeys.includeChecksumsFromTPM = true;
      networking.hostName = "machine";
      users.users.alice = {
        isNormalUser = true;
        uid = 1000;
        # Every account reaches the TPM, as core's tpm module lets it.
        extraGroups = [ "tss" ];
      };
      security.tpm2.enable = true;
      # fido2-token, which the test watches a YubiKey come and go by, as
      # core's yubikey module gives a machine.
      environment.systemPackages = [ pkgs.libfido2 ];
      # Two SSH keys in the TPM, one asking for a PIN and one not, as
      # ssh-tpm-agent's and tpm-signing's modules name them.
      jitsusama.guide.keys = [
        {
          account = "alice";
          file = "/home/alice/.ssh/id_ecdsa";
          comment = "alice@machine";
          pin = true;
        }
        {
          account = "alice";
          file = "/home/alice/.ssh/agent-signing";
          comment = "alice@machine agents";
          pin = false;
        }
      ];
      # And a spare on each YubiKey, as ssh-tpm-agent's module asks.
      jitsusama.guide.spares = [
        {
          account = "alice";
          folder = "/home/alice/.ssh";
          comment = "alice@yubikey";
        }
      ];
      # Nothing starts the guide but core's module, and nothing lets anyone
      # past polkit: what a machine of its own would have.
      boot.loader.timeout = 0;
      # The test driver turns systemd's status lines off, which a laptop
      # has on, and which would land across the guide where the console
      # is its screen.
      systemd.settings.Manager.ShowStatus = lib.mkForce true;
      system.switch.enable = true;
      system.stateVersion = "26.05";
      virtualisation = {
        directBoot.enable = false;
        mountHostNixStore = false;
        useDefaultFilesystems = false;
        fileSystems = lib.mkForce { };
        useEFIBoot = true;
        tpm.enable = true;
        diskSize = 8192;
        # Room for the stick's system in memory beside the install.
        memorySize = 6144;
        # Typed keys go to the keyboard QEMU added last, a USB one keyd
        # doesn't lay out, and the disk's prompt is answered on the serial
        # console.
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

  testScript =
    { nodes, ... }:
    let
      payload = pkgs.callPackage ../modules/guide/payload.nix { } {
        config = nodes.machine;
        yubikeys = [
          "the Nano"
          "the 5C"
        ];
        account = "alice";
      };
      image = "${nodes.stick.system.build.image}/${nodes.stick.image.filePath}";
      # The stick as `guide write-stick` writes it, onto a stick with room
      # for the machine's whole system.
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
            # The payload's size in whole MiB, as a stick's is whole sectors.
            room=$(( ($(du -sbL ${payload} | cut -f1) >> 20) + 1 ))
            truncate -s $(( $(stat -c %s ${image}) + (room << 20) + (512 << 20) + 7 * 512 )) $out/stick.raw
            echo ${lib.escapeShellArg passphrase} | guide write-stick ${image} $out/stick.raw --install ${payload}
          '';
      # The driver runs on the host, so what the script needs from Nix, the
      # keys' state, the sticks' images, the programs it runs and the secrets
      # to type, reaches it in one file. tpm2_clear is run on the machine by
      # its place in the store, rather than put on the machine's path, which
      # would change the system the guide installs.
      values = pkgs.writeText "guide-install-values.json" (
        builtins.toJSON {
          canokeys = "${keys}";
          qemuImg = "${qemu}/bin/qemu-img";
          written = "${written}/stick.raw";
          toplevel = "${nodes.machine.system.build.toplevel}";
          tpm2Clear = "${pkgs.tpm2-tools}/bin/tpm2_clear";
          keygen = "${pkgs.ssh-tpm-agent}/bin/ssh-tpm-keygen";
          payload = "${payload}";
          guide = "${guide}/bin/guide";
          inherit
            image
            pin
            tpmPin
            keyPin
            password
            passphrase
            bundlePassphrase
            ;
        }
      );
      script = builtins.readFile ./guide-install-vm-test.py;
    in
    builtins.replaceStrings [ "@values@" ] [ "${values}" ] script;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
