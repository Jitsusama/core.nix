# A stick's vault sealed to a YubiKey by age-plugin-fido2-hmac and opened
# on the stick's console with that key's PIN and a touch, then, the key
# taken out, with the recovery passphrase. The key is a canokey with a PIN,
# its recipient made on the machine as a person makes one, with the
# plugin's own questions answered; the stick is a small image the guide
# writes into a file, looped so its vault is a partition as on a stick.
{ pkgs }:
let
  inherit (pkgs) lib;
  pin = "123456";
  passphrase = "a recovery passphrase for the stick";
  guide = pkgs.callPackage ../modules/guide/package.nix { };

  # canokey keeps one global state and makes working keys only at QEMU's
  # start, as tests/guide-install.nix says; one key is all this needs.
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

  setPin = pkgs.writers.writePython3Bin "set-pin" { libraries = [ pkgs.python3Packages.fido2 ]; } ''
    import sys
    from fido2.hid import CtapHidDevice
    from fido2.ctap2 import Ctap2, ClientPin

    device = next(iter(CtapHidDevice.list_devices()))
    ClientPin(Ctap2(device)).set_pin(sys.argv[1])
  '';

  # A recipient made as a person makes one, the plugin's questions
  # answered as its README says: the PIN, a PIN for decrypting too, and no
  # separate identity, so the key alone opens what's sealed to it. age's
  # terminal takes a choice as 1 or 2 (filippo.io/age/plugin/tui.go), and
  # a minute's silence ends it with what it said.
  makeRecipient = pkgs.writers.writePython3Bin "make-recipient" { flakeIgnore = [ "E501" ]; } ''
    import os
    import pty
    import re
    import select
    import sys

    pid, fd = pty.fork()
    if pid == 0:
        os.execvp("${lib.getExe pkgs.age-plugin-fido2-hmac}", ["age-plugin-fido2-hmac", "-g"])

    said = ""
    answered = set()
    answers = [
        ("enter your PIN", sys.argv[1]),
        ("require a PIN", "1"),
        ("separate identity", "2"),
    ]
    while True:
        if not select.select([fd], [], [], 60)[0]:
            sys.exit("the plugin went quiet, having said:\n" + said)
        try:
            chunk = os.read(fd, 1024).decode(errors="replace")
        except OSError:
            break
        if not chunk:
            break
        said += chunk
        for prompt, answer in answers:
            if prompt in said and prompt not in answered:
                answered.add(prompt)
                # A choice is one key; a value ends with Return.
                os.write(fd, (answer if answer in "12" else answer + "\n").encode())
    os.waitpid(pid, 0)
    sys.stderr.write(said)
    print(re.findall(r"age1fido2-hmac1[0-9a-z]+", said)[-1])
  '';
in
pkgs.testers.runNixOSTest {
  name = "guide-vault";
  qemu.package = qemu;

  nodes.machine = {
    services.getty.autologinUser = "root";
    environment.systemPackages = [
      guide
      makeRecipient
      setPin
      pkgs.libfido2
      pkgs.util-linux
    ];
    virtualisation = {
      memorySize = 2048;
      qemu.virtioKeyboard = false;
      qemu.options = [
        "-device qemu-xhci,id=xhci"
        "-device canokey,id=key,bus=xhci.0,file=/tmp/canokey-vault"
      ];
    };
  };

  testScript = ''
    import json, re

    def screen():
        return machine.get_tty_text("1")

    def on_screen(pattern, timeout=60):
        def shown(_):
            text = screen()
            if re.search(r"(?i)(try again)", text):
                raise Exception("the guide stopped:\n" + text)
            return re.search("(?i)" + pattern, text) is not None
        with machine.nested(f"waiting for {pattern} on the guide's screen"):
            try:
                retry(shown, timeout)
            except Exception:
                print("SCREEN\n" + screen())
                print("PROCESSES\n" + machine.execute("ps -eo pid,stat,wchan:20,args | grep -i -e guide -e age-plugin")[1])
                print("LOG\n" + machine.execute("tail -n 40 /run/guide/log")[1])
                raise

    def type(text):
        machine.send_chars(text + "\n")

    machine.wait_for_unit("multi-user.target")
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"')

    with subtest("the key gets a PIN and a recipient, as a person makes one"):
        machine.succeed("set-pin ${pin}")
        recipient = machine.succeed("make-recipient ${pin}").strip()
        assert recipient.startswith("age1fido2-hmac1"), recipient

    with subtest("the guide seals a stick's vault to it, needing no touch"):
        # A small image with an ESP and one more partition, as core's has.
        machine.succeed(
            "truncate -s 16M /root/image.raw && "
            "printf 'label: gpt\\nsize=4M, type=uefi, name=esp\\nsize=4M, name=store\\n' | sfdisk /root/image.raw"
        )
        machine.succeed("mkdir -p /root/payload/cache && touch /root/payload/layout.sh")
        target = {
            "machine": "laptop", "disk": "/dev/disk/by-id/nvme-the-laptops", "disk_name": "system",
            "install_key": "/run/guide/install-key", "layout_packages": [], "system": "/nix/store/abc",
            "account": "alice", "yubikeys": ["the Nano"], "recipients": [recipient],
        }
        machine.succeed(f"echo '{json.dumps(target)}' > /root/payload/target.json")
        machine.succeed("truncate -s 120M /root/stick.raw")
        print(machine.succeed(
            "echo '${passphrase}' | guide write-stick /root/image.raw /root/stick.raw --install /root/payload"
        ))
        machine.succeed("losetup --find --partscan --show /root/stick.raw && udevadm settle")
        machine.succeed("test -e /dev/disk/by-partlabel/vault")

    with subtest("on the stick, the key and its PIN open the vault"):
        machine.wait_until_succeeds("pgrep -u root -t tty1 bash")
        type("guide install")
        on_screen("Check the keyboard")
        type("keyboard")
        on_screen("Plug in the Nano")
        type("")
        on_screen("the PIN of the Nano")
        type("${pin}")
        on_screen("the vault is open")

    with subtest("with the key taken out, the recovery passphrase opens it"):
        machine.send_key("ctrl-c")
        machine.succeed("rm /run/guide/vault-key")
        machine.send_monitor_command("device_del key")
        machine.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=30)
        type("guide install")
        on_screen("Plug in the Nano")
        type("")
        on_screen("the recovery passphrase")
        type(${builtins.toJSON passphrase})
        on_screen("the vault is open")
  '';
}
