# A focused session window's memory in swap is read back when an
# administrator's sorter asks, through memory-swap-in@, and nobody else's
# (modules/slices). The tests are swap-in-vm-test.py, which pytest runs
# inside the driver (tests/lib/vm_pytest.py), with a stand-in niri, a
# stand-in kitty drawing many windows and stand-in heaps beside it.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "swap-in";

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.slices ];
      virtualisation.memorySize = 1024;
      virtualisation.diskSize = 2048;
      # The test VM empties swapDevices at this priority, and lists set at
      # the same one are joined.
      swapDevices = lib.mkVMOverride [
        {
          device = "/var/swapfile";
          size = 512;
        }
      ];
      users.users.someone = {
        isNormalUser = true;
        extraGroups = [ "wheel" ];
        # Its runtime folder, where niri's socket is, stands from boot.
        linger = true;
      };
      users.users.other = {
        isNormalUser = true;
        linger = true;
      };
      environment.systemPackages = [ pkgs.python3 ];
      environment.etc."swap-in/niri.py".source = ./swap-in-niri.py;
      environment.etc."swap-in/heap.py".source = ./swap-in-heap.py;
      environment.etc."swap-in/kitty.py".source = ./swap-in-kitty.py;
    };

  tests = ./swap-in-vm-test.py;
}
