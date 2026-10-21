# A machine installed from its stick by the guide, driven by the keys a
# person would press and read off its console, as a laptop is: the stick
# written by `guide write-stick --install` with the machine's payload,
# plugged in by USB and booted first, two FIDO2 keys enrolled by name, a
# recovery key typed back, Secure Boot keys made and the system installed
# from the vault with no network. Then the stick comes out, and the machine
# boots its own disk, opened by a key's PIN, and enrols its Secure Boot keys.
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
# secrets. A getrandom-backed random32 makes each key its own.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  pin = "123456";
  password = "a password for alice";
  passphrase = "the stick's recovery passphrase";

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
    { lib, ... }:
    {
      imports = [
        self.nixosModules.disko
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
      };
      boot.loader.timeout = 0;
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
        memorySize = 4096;
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
    in
    ''
      import os, re, shutil, subprocess, tempfile

      keys = tempfile.mkdtemp()
      for name in ["nano", "fivec"]:
          shutil.copy("${keys}/" + name, f"{keys}/{name}")
          os.chmod(f"{keys}/{name}", 0o600)
      os.environ["KEYS_DIR"] = keys
      stick = tempfile.NamedTemporaryFile(suffix=".qcow2")
      subprocess.run([
          "${qemu}/bin/qemu-img", "create", "-f", "qcow2",
          "-b", "${written}/stick.raw", "-F", "raw", stick.name,
      ], check=True)
      os.environ["STICK_IMAGE"] = stick.name

      def screen():
          return machine.get_tty_text("1")

      # The screen as a person reads it, whatever's capitalised; phrases
      # short enough that no wrap splits them. A step that stops says so at
      # once, rather than after the wait for what it would have shown.
      def on_screen(pattern, timeout=120):
          def shown(_):
              text = screen()
              if re.search(r"(?i)(try again|look again)", text):
                  raise Exception("the guide stopped:\n" + text)
              return re.search("(?i)" + pattern, text) is not None
          with machine.nested(f"waiting for {pattern} on the guide's screen"):
              retry(shown, timeout)

      def type(text):
          machine.send_chars(text + "\n")

      def plug(name):
          machine.send_monitor_command("device_del key")
          machine.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=30)
          machine.send_monitor_command(f"device_add canokey,id=key,bus=xhci.0,file={keys}/{name}")
          machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=30)

      machine.start(allow_reboot=True)

      with subtest("the stick boots into the guide, which checks the keyboard first"):
          on_screen("Check the keyboard types what you mean")
          type("keyboard")

      with subtest("the vault opens with the recovery passphrase"):
          on_screen("recovery passphrase")
          type(${builtins.toJSON passphrase})

      # What the tools print here, for the guide's fixtures (qemu-canokey).
      def fixture(name, command):
          print(f"FIXTURE {name} " + machine.succeed(command).replace("\n", "\\n"))

      with subtest("the disk is shown and the machine's name typed back"):
          on_screen("Confirm the disk to wipe")
          on_screen("wipes")
          fixture("lsblk-blank.json", "lsblk --json --output-all --bytes")
          type("machine")

      for name, state in [("the Nano", "nano"), ("the 5C", "fivec")]:
          with subtest(f"{name} is enrolled by name, and opens the disk"):
              on_screen(f"Plug in {name}", timeout=300)
              if state == "nano":
                  fixture("lsblk-laid-out.json", "lsblk --json --output-all --bytes")
                  fixture("fido2-token-L.txt", "fido2-token -L")
              else:
                  plug(state)
              machine.succeed("set-pin ${pin}")
              type("")
              on_screen(f"{name}'s PIN")
              type("${pin}")

      with subtest("the recovery key is shown, typed back, and opens the disk"):
          on_screen("Keep the recovery key", timeout=300)
          # Read as a person reads it, its groups perhaps on two lines.
          (key,) = re.findall(r"(?:[a-z]{8}-){7}[a-z]{8}", "".join(screen().split()))
          type(key)

      with subtest("the system installs from the vault, and the account gets its password"):
          on_screen("a password for alice", timeout=1800)
          type(${builtins.toJSON password})
          on_screen("the same password again")
          type(${builtins.toJSON password})

      with subtest("only the keys and the recovery key open the disk once it's done"):
          # The reboot card's own key, which nothing before it shows.
          on_screen("shut down", timeout=300)
          fixture("luks-dump-installed.json", "cryptsetup luksDump --dump-json-metadata /dev/disk/by-partlabel/system")
          system = machine.succeed("readlink -f /mnt/nix/var/nix/profiles/system").strip()
          assert system == "${nodes.machine.system.build.toplevel}", system
          machine.fail("test -e /run/guide/install-key")
          machine.succeed("test -s /mnt/var/lib/sbctl/keys/db/db.key")

      with subtest("the card says what the disk will ask, and the machine shuts down"):
          on_screen("security token PIN")
          type("")
          machine.wait_for_shutdown()

      with subtest("the stick comes out, and the machine boots its own disk with a key's PIN"):
          # A blank stick stands where it was, which the firmware passes over.
          blank = tempfile.NamedTemporaryFile(suffix=".qcow2")
          subprocess.run(["${qemu}/bin/qemu-img", "create", "-f", "qcow2", blank.name, "64M"], check=True)
          os.environ["STICK_IMAGE"] = blank.name
          # systemd-cryptsetup asks through libcryptsetup's token plugin
          # first and then its own FIDO2 code, and a key that's not there
          # yet at the first leaves the second asking, so each prompt is
          # answered until the disk opens.
          def unlock():
              machine.wait_for_console_text("Please enter")
              for _ in range(4):
                  machine.send_console("${pin}\n")
                  try:
                      machine.wait_for_console_text("Finished .*Cryptography Setup for system", timeout=30)
                      return
                  except Exception:
                      pass
              raise Exception("the disk never opened with the key's PIN")

          machine.start(allow_reboot=True)
          unlock()
          machine.connect()
          # lanzaboote enrols the keys the guide made and restarts with
          # Secure Boot on.
          machine.wait_for_console_text("Linux version", timeout=300)
          machine.connected = False
          unlock()
          machine.connect()
          machine.wait_for_unit("multi-user.target")
          status = machine.succeed("bootctl status")
          print(status)
          t.assertIn("Secure Boot: enabled (user)", status)
          machine.succeed("grep -q '^alice:\\$' /etc/shadow")
    '';
}
