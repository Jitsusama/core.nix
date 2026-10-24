# Plugs the desk's lights into a virtual machine, pulls them out and plugs
# them in again, and watches what their programs tell them each time, as at
# a desk left and come back to. The Litra must light again while a camera
# streams, and the Pebble X Plus's lighting must go out at a lock after it's
# back. A VM has neither, so stand-ins made of the kernel's uhid take their
# USB ids and answer as they do (tests/desk-light-stand-in.py). The script is
# desk-lights-vm-test.py.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "desk-lights";

  nodes.machine = {
    imports = [
      self.nixosModules.litra
      self.nixosModules.creative-pebble-x-plus
    ];
    boot.kernelModules = [ "uhid" ];
    environment.systemPackages = [ pkgs.python3 ];
  };

  # The script reads the stand-in from the file put in place of its marker.
  testScript = builtins.replaceStrings [ "@standIn@" ] [ "${./desk-light-stand-in.py}" ] (
    builtins.readFile ./desk-lights-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
