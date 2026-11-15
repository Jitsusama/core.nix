# Asks the kernel where a pi host's busy command goes: a host's unit sits in
# agents.slice beside the sessions Joel types into, and every command its
# sessions start runs in it, so one runaway there once slowed them all. The
# tests are hosts-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
let
  # The programs the tests' units run, which they read from this file as
  # their values fixture.
  values = pkgs.writeText "hosts-values.json" (
    builtins.toJSON {
      sleep = "${pkgs.coreutils}/bin/sleep";
      bash = "${pkgs.bash}/bin/bash";
      setsid = "${pkgs.util-linux}/bin/setsid";
      wakes = "${./slices-foreground-wakes.sh}";
    }
  );
in
import ../../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "hosts";

  nodes.machine = {
    imports = [ self.nixosModules.slices ];
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
  };

  tests = ./hosts-vm-test.py;
  inherit values;
}
