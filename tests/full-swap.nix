# Fills memory and swap the way a git repack does, steadily and without
# stalling, beside a small program that holds a little swap of its own, and
# looks for systemd-oomd ending the one that filled it before the kernel's
# OOM killer picks one. The tests are full-swap-vm-test.py, which pytest runs
# inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  # Takes memory that compresses about two to one, up to the MiB it's told,
  # and keeps it. ruff and mypy read it through `nix fmt`, so the writers'
  # flake8 is off.
  grow = pkgs.writers.writePython3Bin "grow" { doCheck = false; } (
    builtins.readFile ./full-swap-grow.py
  );
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "full-swap";

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.memory ];
      # Small, so swap fills in about a minute: zram takes as much as memory,
      # and a swapfile on the disk as much again, as on a machine whose swap
      # outgrows zram.
      virtualisation.memorySize = 1024;
      virtualisation.diskSize = 3072;
      # The test VM empties swapDevices at this priority, and lists set at
      # the same one are joined.
      swapDevices = lib.mkVMOverride [
        {
          device = "/var/swapfile";
          size = 1024;
        }
      ];
      # The test driver makes the kernel's OOM killer panic the machine. Here
      # it's what must not happen first, so it's left to say so in its log.
      boot.kernel.sysctl."vm.panic_on_oom" = 0;
      environment.systemPackages = [ grow ];
    };

  tests = ./full-swap-vm-test.py;
}
