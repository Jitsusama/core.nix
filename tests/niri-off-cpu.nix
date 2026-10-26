# An account in wheel traces its own niri off the CPU with no password, and
# nobody else can (modules/niri-off-cpu). A stand-in named as niri's session
# is runs as that account (niri-off-cpu/stand-in-niri.py). The tests are
# niri-off-cpu-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  standIn = pkgs.writers.writePython3Bin "niri" {
    doCheck = false;
  } (builtins.readFile ./niri-off-cpu/stand-in-niri.py);
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "niri-off-cpu";

  nodes.machine = {
    imports = [ self.nixosModules.niri-off-cpu ];
    # The graphics memory manager, with no graphics card to drive, so the
    # count of buffers it reads back has the types it reads.
    boot.kernelModules = [ "ttm" ];
    # The graphics memory manager, with no graphics card to drive, so the
    # count of buffers it reads back has the types it reads.
    users.users.someone = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };
    users.users.other.isNormalUser = true;
  };

  tests = ./niri-off-cpu-vm-test.py;
  values = pkgs.writeText "niri-off-cpu-values.json" (
    builtins.toJSON { stand_in = "${standIn}/bin/niri"; }
  );
}
