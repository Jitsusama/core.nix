# Takes a virtual machine's second disk away under disk-loss-panic and
# watches the kernel panic. First a warning from the disk's path while the
# disk still answers, which must change nothing; then the disk's PCI device
# removed, as a disk that drops off its bus is, and the same warning, which
# must panic the kernel with the program's line before the kernel's. The
# warning is written to the kernel's log from the test, since nothing else
# would say it on cue. The script is disk-loss-panic-vm-test.py.
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "disk-loss-panic";

  nodes.machine = {
    imports = [ self.nixosModules.disk-loss-panic ];
    virtualisation.emptyDiskImages = [ 16 ];
    jitsusama.diskLossPanic.disk = "/dev/vdb";
  };

  tests = ./disk-loss-panic-vm-test.py;
}
