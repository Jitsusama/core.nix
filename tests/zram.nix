# Takes the compressed swap down while something holds its device open, as
# udev does to probe it when swapoff closes it, and lets go only once the
# reset has found it busy. The kernel won't reset a device anything holds
# open, so a reset that tries once fails, and the device stays set up. The
# test is zram-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  # Holds the device open until the reset says it waits, then lets go at
  # once, as udev's probe would. ruff and mypy read it through `nix fmt`, so
  # the writers' flake8 is off.
  hold = pkgs.writers.writePython3Bin "hold-zram" { doCheck = false; } (
    builtins.readFile ./zram-hold.py
  );
in
pkgs.testers.runNixOSTest {
  name = "zram";

  nodes.machine = {
    imports = [ self.nixosModules.memory ];
    environment.systemPackages = [ hold ];
  };

  testScript = ''import vm_pytest; vm_pytest.run(globals(), "${./zram-vm-test.py}")'';
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}
