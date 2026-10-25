# Asks the kernel what disk latency and weight each slice was given, the
# account's and the system's, on a machine that names no disk and on one
# that names its own with a model, as a measured machine does.
# The target is written by systemd as a slice starts, for the drive it finds
# behind /home, and the kernel takes it only for a drive whose queue it can
# watch, so reading the module's settings back would prove neither. The
# tests are slices-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
let
  # The programs the tests' units run, which they read from this file as
  # their values fixture.
  values = pkgs.writeText "slices-values.json" (
    builtins.toJSON {
      sleep = "${pkgs.coreutils}/bin/sleep";
      bash = "${pkgs.bash}/bin/bash";
      chrt = "${pkgs.util-linux}/bin/chrt";
      taskset = "${pkgs.util-linux}/bin/taskset";
    }
  );
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
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

  tests = ./slices-vm-test.py;
  inherit values;
}
