# Fills memory and swap the way a git repack does, steadily and without
# stalling, beside a small program that holds a little swap of its own, and
# looks for systemd-oomd ending the one that filled it before the kernel's
# OOM killer picks one. The script is full-swap-vm-test.py.
{ self, pkgs }:
let
  # Takes another 16 MiB of data that doesn't compress every tenth of a
  # second, up to the MiB it's told, and keeps it: random bytes, so zram
  # holds about as much as it's given, as it does a pack's.
  grow = pkgs.writers.writePython3Bin "grow" { } ''
    import os
    import sys
    import time

    cap = int(sys.argv[1])
    held = []
    while len(held) * 16 < cap:
        held.append(os.urandom(16 << 20))
        time.sleep(0.1)
    while True:
        time.sleep(1)
  '';
in
pkgs.testers.runNixOSTest {
  name = "full-swap";

  nodes.machine = {
    imports = [ self.nixosModules.memory ];
    # Small, so swap fills in under a minute: zram takes half of it.
    virtualisation.memorySize = 1024;
    environment.systemPackages = [ grow ];
  };

  testScript = builtins.readFile ./full-swap-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
