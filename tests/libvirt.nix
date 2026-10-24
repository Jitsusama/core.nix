# Stops libvirt's daemon while its udev thread waits for the CPU, as it does
# on a machine busy shutting down. At shutdown libvirt tells that thread to
# end and then joins it, holding the lock the thread needs to see it was told,
# so a thread that hasn't run by then never can: both wait until the daemon
# gives up 30 seconds later and exits as failed. The daemon shares one CPU
# with a busy loop here, and its udev thread runs only when nothing else will,
# so a stop often finds the thread still waiting, and ten leave the old code
# next to no way through. The script is libvirt-vm-test.py.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "libvirt";

  nodes.machine = {
    imports = [ self.nixosModules.libvirt ];
    virtualisation.memorySize = 2048;
    # The busy loop takes one, and the test's own shell the other.
    virtualisation.cores = 2;
  };

  # What VM tests' scripts share, and pytest, which runs this one's tests (tests/lib).
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  testScript = ''import vm_pytest; vm_pytest.run(globals(), "${./libvirt-vm-test.py}")'';
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}
