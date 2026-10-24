# Asks the kernel what disk latency and weight each slice was given, the
# account's and the system's, on a machine that names no disk and on one
# that names its own with a model, as a measured machine does.
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
      chrt = "${pkgs.util-linux}/bin/chrt";
      taskset = "${pkgs.util-linux}/bin/taskset";
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

  # The numbers only have to be ones the kernel takes: a virtual disk's
  # speed says nothing of a real one's.
  nodes.ordered = {
    imports = [ self.nixosModules.slices ];
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    jitsusama.slices.disks.vda = {
      model = {
        rbps = 1000000000;
        rseqiops = 100000;
        rrandiops = 50000;
        wbps = 800000000;
        wseqiops = 80000;
        wrandiops = 40000;
      };
      qos = {
        rpct = 95;
        rlat = 5000;
        wpct = 95;
        wlat = 5000;
      };
    };
  };

  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./slices-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
