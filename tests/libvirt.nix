# Stops libvirt's daemon while its udev thread waits for the CPU, as it does
# on a machine busy shutting down. At shutdown libvirt tells that thread to
# end and then joins it, holding the lock the thread needs to see it was told,
# so a thread that hasn't run by then never can: both wait until the daemon
# gives up 30 seconds later and exits as failed. The daemon shares one CPU
# with a busy loop here, and its udev thread runs only when nothing else will,
# so a stop often finds the thread still waiting, and ten leave the old code
# next to no way through. The script is libvirt-vm-test.py.
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "libvirt";

  nodes.machine = {
    imports = [ self.nixosModules.libvirt ];
    virtualisation.memorySize = 2048;
    # The busy loop takes one, and the test's own shell the other.
    virtualisation.cores = 2;
  };

  tests = ./libvirt-vm-test.py;
}
