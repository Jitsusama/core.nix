# Boots a machine whose firmware hands it a platform health table and a
# boot error record, as the XPS 14's does, and reads the boot's copies as
# an administrator and as an account that isn't one. QEMU adds both tables:
# PHAT with a line of the test's own as its body, and BERT pointing at 64
# bytes of the BIOS's own memory as its error region, so the kernel lays
# that region out in sysfs as it does a real one. The script is
# firmware-errors-vm-test.py.
{ self, pkgs }:
let
  phat = pkgs.writeText "phat-body" "firmware-errors stand-in";
  # BERT's body: the region's length, 4 bytes, then its address, 8, both
  # little-endian. 0xF0000 is the BIOS area, which every QEMU machine has.
  bert = pkgs.runCommand "bert-body" { } ''
    printf '\x40\x00\x00\x00\x00\x00\x0f\x00\x00\x00\x00\x00' > $out
  '';
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "firmware-errors";

  nodes.machine = {
    imports = [ self.nixosModules.firmware-errors ];
    virtualisation.qemu.options = [
      "-acpitable sig=PHAT,data=${phat}"
      "-acpitable sig=BERT,data=${bert}"
    ];
    users.users.alice = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };
    users.users.bob.isNormalUser = true;
  };

  tests = ./firmware-errors-vm-test.py;
}
