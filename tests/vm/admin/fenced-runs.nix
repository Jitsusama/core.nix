# An agent's `mycelium run` starts through the launcher its fence declares
# (modules/mycelium), and this runs commands through it as the account's
# declared.json says, as the node would: in the run's folder, with the
# account's runtime folder, bus, agents and sockets all live beside it.
# Nothing the fence keeps out is reachable from inside, and the folder
# still is. Only a running system has a user bus, a runtime folder and the
# kernel's Landlock, which is why this is a VM test. The tests are
# fenced-runs-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
let
  # Landlock alone, as a stand-in for the fence's launcher until it ships:
  # it leaves named sockets reachable, which the tests show.
  standIn = pkgs.writeShellScript "fence-stand-in" ''
    exec ${pkgs.landrun}/bin/landrun \
      --rox /nix/store --rox /run/current-system --rox /etc/profiles \
      --ro /etc --ro /proc --rw "$PWD" --rw /dev/null \
      --unrestricted-network --ignore-missing --env PATH --env HOME -- "$@"
  '';
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "fenced-runs";

  nodes.machine = {
    imports = [ self.nixosModules.home-manager ];
    # The fence asks for Landlock's scoping of abstract sockets and signals,
    # which the default kernel's Landlock lacks, as the machines' doesn't.
    boot.kernelPackages = pkgs.linuxPackages_latest;
    users.users.someone = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    environment.systemPackages = [
      pkgs.jq
      pkgs.openssh
      pkgs.socat
    ];
    home-manager.users.someone = {
      imports = [ self.homeModules.mycelium ];
      home.stateVersion = "26.05";
      jitsusama.mycelium = {
        declares.fence = {
          fence = {
            builders = [ ];
            forges = [ ];
            secrets = [ ];
          };
          strict = {
            builders = [ ];
            forges = [ ];
            secrets = [ ];
          };
        };
        fenceRuns = [ "${standIn}" ];
      };
    };
  };

  tests = ./fenced-runs-vm-test.py;
}
