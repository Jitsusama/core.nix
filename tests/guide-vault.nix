# A stick's vault sealed to a YubiKey by age-plugin-fido2-hmac and opened
# on the stick's console with that key's PIN and a touch, then, the key
# taken out, with the recovery passphrase. The key is a canokey with a PIN,
# its recipient made on the machine as a person makes one, with the
# plugin's own questions answered; the stick is a small image the guide
# writes into a file, looped so its vault is a partition as on a stick.
# The script is guide-vault-vm-test.py.
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

  # The key's PIN and the stick's recovery passphrase, which the script types
  # and hands to the guide, reach it in one file it reads at its top.
  testScript =
    let
      values = pkgs.writeText "guide-vault-values.json" (
        builtins.toJSON {
          inherit pin passphrase;
        }
      );
      script = builtins.readFile ./guide-vault-vm-test.py;
    in
    builtins.replaceStrings [ "@values@" ] [ "${values}" ] script;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
