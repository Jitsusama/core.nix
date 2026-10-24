# Asks the kernel what disk latency each slice was given, the account's and
# the system's.
# The target is written by systemd as a slice starts, for the drive it finds
# behind /home, and the kernel takes it only for a drive whose queue it can
# watch, so reading the module's settings back would prove neither. The
# script is slices-vm-test.py.
{ self, pkgs }:
let
  # The programs the script's units run, which it reads from this file,
  # swapped in for its @values@ marker.
  values = pkgs.writeText "slices-values.json" (
    builtins.toJSON {
      sleep = "${pkgs.coreutils}/bin/sleep";
      bash = "${pkgs.bash}/bin/bash";
    }
  );
in
pkgs.testers.runNixOSTest {
  name = "slices";

  nodes.machine = {
    imports = [ self.nixosModules.slices ];
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
  };

  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./slices-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
