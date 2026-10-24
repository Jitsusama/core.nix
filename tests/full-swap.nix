# Fills memory and swap the way a git repack does, steadily and without
# stalling, beside a small program that holds a little swap of its own, and
# looks for systemd-oomd ending the one that filled it before the kernel's
# OOM killer picks one. The script is full-swap-vm-test.py.
{ self, pkgs }:
let
  # Takes another 16 MiB every half second, up to the MiB it's told, and
  # keeps it. Each page is half random and half zeros, so it compresses
  # about two to one, as a program's memory does: data that doesn't
  # compress costs zram a page of memory for each page it takes, and
  # memory runs out with swap half used, before oomd may act.
  grow = pkgs.writers.writePython3Bin "grow" { } ''
    import os
    import sys
    import time

    cap = int(sys.argv[1])
    held = []
    while len(held) * 16 < cap:
        pages = (os.urandom(2048) + bytes(2048) for _ in range(4096))
        held.append(b"".join(pages))
        time.sleep(0.5)
    while True:
        time.sleep(1)
  '';
in
pkgs.testers.runNixOSTest {
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

  testScript = builtins.readFile ./full-swap-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
