# A crash's core is kept where only root reads it, while its owner, in
# wheel as Joel's account is, still reads its stack trace in the journal,
# and a node's is removed six hours on
# (modules/coredump). An account aborts its own process, as any program
# running as it could abort a pi in a terminal, and then can't open or dump
# the core that holds what the process held. A program named node that only
# waits stands in for that pi, since the removal goes by the core's name; a copy of
# sleep won't do, as coreutils names its process after the program it acts
# as. Only a running
# system has systemd-coredump, the journal and tmpfiles' modes, which is
# why this is a VM test. The tests are cores-kept-from-their-owner-vm-test.py,
# which pytest runs inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "cores-kept-from-their-owner";

  nodes.machine = {
    imports = [ self.nixosModules.coredump ];
    environment.systemPackages = [
      (pkgs.writeCBin "node" ''
        #include <unistd.h>
        int main(void) { sleep(600); return 0; }
      '')
    ];
    users.users.someone = {
      isNormalUser = true;
      uid = 1000;
      extraGroups = [ "wheel" ];
    };
  };

  tests = ./cores-kept-from-their-owner-vm-test.py;
}
