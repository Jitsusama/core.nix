# Boots a virtual machine with the XPS 14's hold keeping the cores from
# their deepest rest (hardware/dell-xps-14-da14260/deep-rest.nix), and
# reads the kernel's processor latency bound as it boots, stopped, started
# and killed, and the journal's line on rest. A VM's processors have no C6S
# or C10 to keep out of, so what's checked is the bound the kernel's
# choice of rest keeps to, which it holds the same on any machine. A guest
# under KVM has no cpuidle driver of its own, so the paravirtual one,
# haltpoll, is made to take it, and the residency line is read in its
# confinement over real rest states, as on the laptop. The tests are
# deep-rest-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "deep-rest";

  nodes.machine = {
    imports = [ ../../../hardware/dell-xps-14-da14260/deep-rest.nix ];
    # haltpoll takes a guest only on a host hinting it runs in real time,
    # unless it's forced.
    boot.kernelParams = [ "cpuidle_haltpoll.force=1" ];
    boot.kernelModules = [ "cpuidle-haltpoll" ];
  };

  tests = ./deep-rest-vm-test.py;
}
