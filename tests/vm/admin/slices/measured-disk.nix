# Asks the kernel how a disk named with a model is shared, as a measured
# machine's is: io.cost holding its model and qos, no slice holding a
# latency target, one left from before cleared, and the sharing handed back
# when it stops. The tests are measured-disk-vm-test.py, which pytest runs
# inside the driver.
{ self, pkgs }:
import ../../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "measured-disk";

  # The numbers only have to be ones the kernel takes: a virtual disk's
  # speed says nothing of a real one's.
  nodes.machine = {
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

  tests = ./measured-disk-vm-test.py;
  values = pkgs.writeText "measured-disk-values.json" (
    builtins.toJSON { sleep = "${pkgs.coreutils}/bin/sleep"; }
  );
}
