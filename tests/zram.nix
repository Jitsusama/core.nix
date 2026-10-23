# Takes the compressed swap down while something holds its device open, as
# udev does to probe it when swapoff closes it, and lets go only once the
# reset has found it busy. The kernel won't reset a device anything holds
# open, so a reset that tries once fails, and the device stays set up. The
# script is zram-vm-test.py.
{ self, pkgs }:
let
  # Holds the device open until the reset says it waits, then lets go at
  # once, as udev's probe would, well inside the time the reset allows.
  hold = pkgs.writers.writePython3Bin "hold-zram" { } ''
    import os
    import subprocess

    device = os.open("/dev/zram0", os.O_RDONLY)
    journal = subprocess.Popen(
        ["journalctl", "-k", "-f", "-n", "1", "-o", "cat"],
        stdout=subprocess.PIPE,
        text=True,
    )
    # The last line already logged, so the journal is followed from here.
    journal.stdout.readline()
    open("/run/holding-zram", "w").close()
    for line in journal.stdout:
        if "zram0 is busy" in line:
            break
    os.close(device)
    journal.kill()
  '';
in
pkgs.testers.runNixOSTest {
  name = "zram";

  nodes.machine = {
    imports = [ self.nixosModules.memory ];
    environment.systemPackages = [ hold ];
  };

  testScript = builtins.readFile ./zram-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
