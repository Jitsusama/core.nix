# Runs network-revive in a virtual machine, on an account whose services need
# the network, the way a laptop's do. The machine's only network is a dummy
# link networkd keeps, down at first, as a network that comes late is. Each
# stand-in service starts only once that link reaches beyond the machine, as
# a program that looks a name up as it starts does, and is left in a state
# the network let it down into: one failed for good, one waiting an hour to
# restart, one stopped on purpose, one whose check never passes, and a
# one-shot job with no check, as a backup is. Once the link comes up, each
# that should starts, at once and with one line saying so; the one stopped
# stays stopped, and the one never ready stays down, with a line saying
# why. One that runs reloads when the servers resolved asks change: once
# for a burst however long, and not at all for a burst that ends where it
# began. It keeps following networkd through a restart, and when it starts
# itself, it starts what it finds down. A start asked just as systemd
# executes itself again, as a switch has it do, is asked again once it's
# back. The script is network-revive-vm-test.py.
{ self, pkgs }:
let
  inherit (pkgs) lib;

  # Starts only while the uplink reaches 10.0.0.53, then runs until
  # stopped, or checks whether it could (network-revive-stand-in.sh).
  standIn = pkgs.writeShellScript "stand-in" ''
    export PATH=${
      lib.makeBinPath [
        pkgs.iproute2
        pkgs.coreutils
        pkgs.systemd
      ]
    }
    exec ${lib.getExe pkgs.bash} ${./network-revive-stand-in.sh} "$@"
  '';

  # Writes the servers resolved asks on the link at each reload, so the
  # test can tell one reload from another.
  reloaded = pkgs.writeShellScript "reloaded" ''
    ${pkgs.systemd}/bin/resolvectl dns uplink | ${pkgs.gnused}/bin/sed "s/^.*: //" >> "$HOME/reloads"
  '';

  # Holds the account's systemd in the middle of executing itself again,
  # once, when the stand-in asks for it. A user manager runs its generators
  # at the start of starting up, before it closes its old bus connection.
  holdReexec = pkgs.writeShellScript "hold-reexec" ''
    if [ -e /home/joel/hold ]; then
      ${pkgs.coreutils}/bin/rm /home/joel/hold
      ${pkgs.coreutils}/bin/sleep 3
    fi
  '';

  service = settings: {
    Service = {
      ExecStart = "${standIn}";
    }
    // settings;
  };
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "network-revive";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.network-revive
    ];

    virtualisation.vlans = [ ];
    systemd.user.generators.hold-reexec = "${holdReexec}";
    systemd.network = {
      # Nothing waits for a network the test brings up itself.
      wait-online.enable = false;
      netdevs."10-uplink".netdevConfig = {
        Name = "uplink";
        Kind = "dummy";
      };
      networks."10-uplink" = {
        matchConfig.Name = "uplink";
        address = [ "10.0.0.2/24" ];
        dns = [ "10.0.0.53" ];
        linkConfig.ActivationPolicy = "manual";
      };
    };

    users.users.joel = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    home-manager.users.joel = {
      home.stateVersion = "26.05";
      # What it finds each time it looks, so the test can tell when it has.
      jitsusama.network-revive.debug = true;
      jitsusama.network-revive.units = {
        "needs-network.service".check = [ "--check" ];
        "waits-long.service".check = [ "--check" ];
        "stopped.service".check = [ "--check" ];
        "never-ready.service".check = [ "--never" ];
        "follows-names.service" = {
          check = [ "--check" ];
          reload = true;
        };
        "backs-up.service" = { };
        "meets-reexec.service".check = [ "--reexec" ];
      };
      systemd.user.services = {
        # As a proxy's: it tries five times a second apart, then fails.
        needs-network = service {
          Restart = "always";
          RestartSec = 1;
        };
        waits-long = service {
          Restart = "always";
          RestartSec = "1h";
        };
        stopped = service { };
        never-ready = service {
          Restart = "always";
          RestartSec = 1;
        };
        follows-names = service { ExecReload = "${reloaded}"; };
        backs-up.Service = {
          Type = "oneshot";
          ExecStart = "${standIn} --once";
        };
        meets-reexec = service { };
      };
    };
  };

  tests = ./network-revive-vm-test.py;
}
