# Asks the kernel what each slice was given, the account's and the
# system's: the disk latency, memory floors and CPU weights, where agents'
# work goes, and how it shares the CPU, by quest and by tier.
# The latency target is written by systemd as a slice starts, for the drive
# it finds behind /home, and the kernel takes it only for a drive whose
# queue it can watch, so reading the module's settings back would prove
# neither. A measured disk's sharing is measured-disk.nix's. The tests are
# slices-vm-test.py, which pytest runs inside the driver.
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
      wakes = "${./slices-foreground-wakes.sh}";
      awk = "${pkgs.gawk}/bin/awk";
    }
  );
in
import ../../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "slices";

  # Room enough for the door to let a few of thirteen heavy starts in, and
  # too little for all of them at once.
  nodes.machine = {
    imports = [ self.nixosModules.slices ];
    virtualisation.memorySize = 2048;
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
  };

  # Its agents' work is ranked into three tiers, the second heading no
  # quest, so a tier's weight is seen before any work is in it.
  nodes.ranked = {
    imports = [ self.nixosModules.slices ];
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    jitsusama.slices.tiers = [
      {
        name = "first";
        quests = [ "QEST-20261024-FIRST0" ];
      }
      { name = "second"; }
      {
        name = "third";
        quests = [ "QEST-20261024-THIRD0" ];
      }
    ];
  };

  tests = ./slices-vm-test.py;
  inherit values;
}
