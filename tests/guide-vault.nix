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

  # Gives the key a PIN (set-pin.py), held to ruff and mypy through
  # `nix fmt` rather than the writers' flake8.
  setPin = pkgs.writers.writePython3Bin "set-pin" {
    libraries = [ pkgs.python3Packages.fido2 ];
    doCheck = false;
  } (builtins.readFile ./set-pin.py);

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
