# Boots a machine whose firmware hands it a platform health table, as the
# XPS 14's does, and reads the boot's copy of it as an account would. QEMU
# adds the table, PHAT, with a line of the test's own as its body; it has no
# Boot Error Record Table, so the boot must still pass with that one absent.
# The script is firmware-errors-vm-test.py.
{ self, pkgs }:
let
  body = pkgs.writeText "phat-body" "firmware-errors stand-in";
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "firmware-errors";

  nodes.machine = {
    imports = [ self.nixosModules.firmware-errors ];
    virtualisation.qemu.options = [ "-acpitable sig=PHAT,data=${body}" ];
    users.users.alice.isNormalUser = true;
  };

  tests = ./firmware-errors-vm-test.py;
}
