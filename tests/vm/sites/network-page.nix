# Runs the local sites' server and net's daemon as their home modules render
# them, for an account that lingers, and reads the network page the way a
# browser on the machine would: the page as the server's unit mounts it, and
# its feed reaching net's daemon over the socket systemd holds for it. Away
# from any home network, as a VM is, net still serves how each of its
# sources stands, so the feed has something to say before eero has; once the
# record eero writes is in the runtime folder, net reads the house from it.
# That the page draws what the feed says is network-site-reads-live's,
# beside stand-in daemons in the build; only a running account's manager,
# its runtime folder and the units side by side show that they find each
# other, which is why this is a VM test. The tests are
# network-page-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "the-network-page-reads-its-daemon";

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
    # What eero's daemon writes, as its contract's golden copy holds it, for
    # the test to put where that daemon would; eero's own daemon needs its
    # cloud, which a VM can't reach.
    environment.etc."eero-heard.json".source = ../../../modules/eero/home-network.json;
    home-manager.users.alice = {
      imports = [
        self.homeModules.local-sites
        self.homeModules.net
      ];
      home.stateVersion = "26.05";
    };
  };

  tests = ./network-page-vm-test.py;
}
