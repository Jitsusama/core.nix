# Joel's shell keeps drawing on time while agents' work bursts for memory
# (modules/slices, modules/memory). It replays 12 November's 16:33 freeze
# on an eighth of the laptop's memory: agents' sessions and work each under
# its slice's ceiling, then a fetch and a grep bursting until together
# they pass memory, while a stand-in shell draws a frame every 16 ms. The
# tests are vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  values = pkgs.writeText "joel-first-values.json" (
    builtins.toJSON {
      python = "${pkgs.python3}/bin/python3";
      frames = "${./stand-in-shell.py}";
      heap = "${../swap-in-heap.py}";
      work = "${./stand-in-work.py}";
    }
  );
in
import ../../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "joel-first";

  nodes.machine =
    { lib, ... }:
    {
      imports = [
        self.nixosModules.memory
        self.nixosModules.slices
      ];
      # An eighth of the laptop's 62 GiB, and enough processors that the
      # burst and the shell never wait on each other for one.
      virtualisation.memorySize = 8192;
      virtualisation.cores = 4;
      # Swap and the files the work reads, 3.4 GiB, beside the system.
      virtualisation.diskSize = 12288;
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

  tests = ./vm-test.py;
  inherit values;
}
