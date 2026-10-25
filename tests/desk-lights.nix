# Plugs the desk's lights into a virtual machine, pulls them out and plugs
# them in again, and watches what their programs tell them each time, as at
# a desk left and come back to. The Litra must light again while a camera
# streams, and the Pebble X Plus's lighting must go out at a lock after it's
# back. A VM has neither, so stand-ins made of the kernel's uhid take their
# USB ids and answer as they do (tests/desk-light-stand-in.py). The script is
# desk-lights-vm-test.py.
{ self, pkgs }:
let
  # What the tests need of the above, as their values fixture.
  values = pkgs.writeText "desk-lights-values.json" (
    builtins.toJSON { standIn = "${./desk-light-stand-in.py}"; }
  );
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "desk-lights";

  nodes.machine = {
    imports = [
      self.nixosModules.litra
      self.nixosModules.creative-pebble-x-plus
    ];
    boot.kernelModules = [ "uhid" ];
    environment.systemPackages = [ pkgs.python3 ];
  };

  tests = ./desk-lights-vm-test.py;
  inherit values;
}
