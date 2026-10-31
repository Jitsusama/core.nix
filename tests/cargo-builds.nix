# Boots a machine whose /var/cache is Btrfs, as a workstation's system disk
# is, and checks cargo's own filesystem from there: the image made without
# copy-on-write, attached with direct IO and mounted ext4 with a folder for
# each account; kept across a detach and attach, and across a switch's
# reload with a file open in it; and made again when it's past repairing.
# The size asked for isn't the default, so the test proves it's read. The
# test is cargo-builds-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "cargo-builds";

  nodes.machine = {
    imports = [ self.nixosModules.cargo ];
    users.users.joel.isNormalUser = true;
    jitsusama.cargo.builds.size = "600M";
    virtualisation.emptyDiskImages = [ 2048 ];
    virtualisation.fileSystems."/var/cache" = {
      device = "/dev/vdb";
      fsType = "btrfs";
      autoFormat = true;
    };
    environment.systemPackages = [ pkgs.e2fsprogs ];
  };

  tests = ./cargo-builds-vm-test.py;
}
