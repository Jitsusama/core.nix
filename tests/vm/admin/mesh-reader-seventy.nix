# Runs the mesh reader as its module renders its socket and service, for
# an account whose seventy pi processes follow one folder: the first
# process to ask starts it, each change is read once however many follow,
# and each is told it within 100 ms. Only the account's own systemd starts
# a service from its socket, and only the kernel's inotify shows who opened
# a file, which is why this is a VM test; the unit's values are
# pi-mesh-reader-answers-only-the-account's, and what one client is told is
# the crate's own tests'. The clients are mesh-reader-clients.py, and the
# tests mesh-reader-seventy-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "mesh-reader-serves-seventy";

  nodes.machine = {
    imports = [ self.nixosModules.home-manager ];
    users.users.alice = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    environment.systemPackages = [
      pkgs.inotify-tools
      pkgs.python3
    ];
    environment.etc."mesh-reader-clients.py".source = ./mesh-reader-clients.py;
    home-manager.users.alice = {
      imports = [ ../../../modules/pi/mesh-reader.nix ];
      home.stateVersion = "26.05";
    };
  };

  tests = ./mesh-reader-seventy-vm-test.py;
}
