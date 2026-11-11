# Writing firmware and restarting into the firmware's setup ask an
# administrator, even of one at the seat, and the answer is kept a few
# minutes (modules/fwupd). A YubiKey is named to answer polkit and never
# plugged in, so the password stands in for its touch through the same
# sign-in. The tests are firmware-asks-vm-test.py, which pytest runs
# inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "firmware-asks";

  nodes.machine = {
    imports = [
      self.nixosModules.fwupd
      self.nixosModules.yubikey
    ];
    users.users.someone = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      # Typed to polkit's sign-in, standing in for the person's own.
      initialPassword = "typed";
    };
    # A key enrolled for someone, never plugged in.
    jitsusama.yubikey.administers.someone = [ "aGFuZGxl,cHVibGlj,es256,+presence" ];
    # someone is signed in at the seat, so its session is active and
    # local, the most polkit gives anyone without asking.
    services.getty.autologinUser = "someone";
  };

  tests = ./firmware-asks-vm-test.py;
}
