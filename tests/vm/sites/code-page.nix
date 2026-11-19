# Runs the local sites' server and the repos daemon as their home modules
# render them, for an account that lingers with one repository on its disk,
# and reads the code page the way a browser on the machine would: the page
# as the server's unit mounts it, its feed reaching the daemon over the
# socket systemd holds for it, the daemon reading again as git writes, and
# the homes the daemon writes as it reads answered beside the page. That the
# page draws what the feed says is code-site-reads-live's, beside stand-in
# daemons in the build, and how git's writes are read is serve.rs's own;
# only a running account's manager, its runtime folder and the units side
# by side show that they find each other, which is why this is a
# VM test. The tests are
# code-page-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "the-code-page-reads-its-daemon";

  nodes.machine = {
    imports = [ self.nixosModules.home-manager ];
    users.users.alice = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    # A page's fetch and its feed's WebSocket, as a browser makes them, and
    # git to make and write the repository the daemon reads.
    environment.systemPackages = [
      pkgs.curl
      pkgs.git
      pkgs.websocat
    ];
    home-manager.users.alice = {
      imports = [
        self.homeModules.local-sites
        self.homeModules.repos
      ];
      # Not repos's own root, so the daemon finds the repository only if the
      # option reaches it.
      jitsusama.repos.roots = [ "/home/alice/code" ];
      home.stateVersion = "26.05";
    };
  };

  tests = ./code-page-vm-test.py;
}
