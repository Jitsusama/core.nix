# Runs the local sites' server and the system daemon as their home modules
# render them, for an account that lingers, and reads the system page the
# way a browser on the machine would: the page as the server's unit mounts
# it, and its feed reaching the daemon over the socket systemd holds for it,
# both named by the units alone. That the page draws what the feed says is
# system-site-reads-live's, beside a stand-in daemon in the build; only a
# running account's manager, its runtime folder and the two units side by
# side show that the server and the daemon find each other, which is why
# this is a VM test. The tests are system-page-vm-test.py, which pytest runs
# inside the driver.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "the-system-page-reads-its-daemon";

  nodes.machine = {
    imports = [ self.nixosModules.home-manager ];
    users.users.alice = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    # A page's fetch and its feed's WebSocket, as a browser makes them.
    environment.systemPackages = [
      pkgs.curl
      pkgs.websocat
    ];
    home-manager.users.alice = {
      imports = [
        self.homeModules.local-sites
        self.homeModules.system
      ];
      home.stateVersion = "26.05";
    };
  };

  tests = ./system-page-vm-test.py;
}
