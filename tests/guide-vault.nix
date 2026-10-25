# A stick's vault sealed to a YubiKey by age-plugin-fido2-hmac and opened
# on the stick's console with that key's PIN and a touch, then, the key
# taken out, with the recovery passphrase. The key is a canokey with a PIN,
# its recipient made on the machine as a person makes one, with the
# plugin's own questions answered; the stick is a small image the guide
# writes into a file, looped so its vault is a partition as on a stick.
# The tests are guide-vault-vm-test.py, run through pytest.
{ pkgs }:
let
  inherit (pkgs) lib;
  pin = "123456";
  passphrase = "a recovery passphrase for the stick";
  guide = pkgs.callPackage ../modules/guide/package.nix { };

  # One key is all this needs, made by QEMU at its start (canokeys.nix).
  inherit (import ./canokeys.nix { inherit pkgs; }) qemu setPin;

  # A recipient made as a person makes one, the plugin's questions
  # answered as its README says (guide-vault/make-recipient.py).
  makeRecipient = pkgs.writers.writePython3Bin "make-recipient" { doCheck = false; } (
    builtins.replaceStrings [ "@plugin@" ] [ (lib.getExe pkgs.age-plugin-fido2-hmac) ] (
      builtins.readFile ./guide-vault/make-recipient.py
    )
  );
in
pkgs.testers.runNixOSTest {
  name = "guide-vault";
  qemu.package = qemu;
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

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

  # pytest runs the tests inside the driver, with the key's PIN and the
  # stick's recovery passphrase as their values fixture.
  testScript =
    let
      values = pkgs.writeText "guide-vault-values.json" (
        builtins.toJSON {
          inherit pin passphrase;
        }
      );
    in
    ''
      import vm_pytest
      vm_pytest.run(globals(), "${./guide-vault-vm-test.py}", values="${values}")
    '';
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
