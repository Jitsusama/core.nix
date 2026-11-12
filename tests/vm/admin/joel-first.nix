# Joel's shell keeps drawing on time while agents' work bursts for memory
# (modules/slices, modules/memory). It replays 12 November's 16:33 freeze
# at an eighth of optimus's size: agents' sessions and work holding as much
# of memory as they did, then a fetch and a grep wanting a quarter more of
# it within seconds, while a stand-in shell draws a frame every 16 ms. The
# tests are joel-first-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  values = pkgs.writeText "joel-first-values.json" (
    builtins.toJSON {
      python = "${pkgs.python3}/bin/python3";
      frames = "${./joel-first-frames.py}";
      heap = "${./swap-in-heap.py}";
      work = "${./joel-first-work.py}";
    }
  );
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "joel-first";

  nodes.machine =
    { lib, ... }:
    {
      imports = [
        self.nixosModules.memory
        self.nixosModules.slices
      ];
      # An eighth of optimus's 62 GiB, and enough processors that the
      # burst and the shell never wait on each other for one.
      virtualisation.memorySize = 8192;
      virtualisation.cores = 4;
      virtualisation.diskSize = 8192;
      # The test VM empties swapDevices at this priority, and lists set at
      # the same one are joined.
      swapDevices = lib.mkVMOverride [
        {
          device = "/var/swapfile";
          size = 4096;
        }
      ];
      users.users.alice = {
        isNormalUser = true;
        linger = true;
      };
    };

  tests = ./joel-first-vm-test.py;
  inherit values;
}
