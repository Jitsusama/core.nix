# A switch starts again what it stopped even while home-manager's setup of
# an account hangs (modules/home-manager/nixos.nix). A switch stops a
# changed system service, restarts home-manager's unit, and only then
# starts what it stopped; on 14 November one account's setup hung on its
# user units and kept a keyboard's layout and the VPN down for minutes.
# Here the second system changes a stand-in for such a service and gives
# the account a user unit whose start never ends, so home-manager's setup
# hangs as that one did. Only systemd, a real switch and home-manager show
# this, which is why this is a VM test. The tests are
# switch-outruns-home-manager-vm-test.py, which pytest runs inside the
# driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "a-switch-outruns-home-manager";

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.home-manager ];
      users.users.alice = {
        isNormalUser = true;
        uid = 1000;
        linger = true;
      };
      home-manager.users.alice.home.stateVersion = "26.05";
      # Stands in for keyd or WARP: a service a switch stops when it changes,
      # to start it again at the end.
      systemd.services.stand-in = {
        wantedBy = [ "multi-user.target" ];
        environment.SYSTEM = "first";
        serviceConfig.ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
      };
      # A test machine's disk was never made to boot from, so GRUB can't be
      # installed on it; a switch with no boot loader still switches.
      boot.loader.grub.enable = false;
      specialisation.hangs.configuration = {
        systemd.services.stand-in.environment.SYSTEM = lib.mkForce "hangs";
        # A user unit whose start never ends, which home-manager waits on as
        # it sets the account up.
        home-manager.users.alice.systemd.user.services.never-starts = {
          Unit.Description = "A start that never ends";
          Service = {
            Type = "oneshot";
            ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
          };
          Install.WantedBy = [ "default.target" ];
        };
      };
    };

  tests = ./switch-outruns-home-manager-vm-test.py;
}
