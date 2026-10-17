# Takes the compressed swap down while something holds its device open, as
# udev does to probe it when swapoff closes it, and lets go only once the
# reset has found it busy. The kernel won't reset a device anything holds
# open, so a reset that tries once fails, and the device stays set up.
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

  testScript = ''
    setup = "systemd-zram-setup@zram0.service"
    machine.wait_for_unit("dev-zram0.swap")
    machine.succeed("systemd-run --unit=hold -p Type=exec hold-zram")
    machine.wait_for_file("/run/holding-zram")

    with subtest("the swap comes down once the device is let go"):
        machine.succeed(f"systemctl stop --no-block {setup}")
        machine.wait_until_succeeds(
            f"systemctl show -P ActiveState {setup} | grep -q -x -E 'inactive|failed'", timeout=30
        )
        t.assertEqual(machine.succeed(f"systemctl show -P Result {setup}").strip(), "success")
        t.assertEqual(machine.succeed("cat /sys/block/zram0/disksize").strip(), "0")
  '';
}
