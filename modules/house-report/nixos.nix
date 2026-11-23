# Runs house-report on a Linux machine that shares a line with a shaped
# one, as a system service with an account of its own and nothing it may
# write (system.nix says what it's told).
{ config, lib, ... }:
{
  imports = [ ./system.nix ];

  systemd.services.house-report = {
    description = "Tells the shaped machine on this line what this one sends up";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    serviceConfig = {
      ExecStart = lib.escapeShellArgs config.jitsusama.house-report.arguments;
      DynamicUser = true;
      Restart = "always";
      RestartSec = 5;
      # Asked wrongly (the terminal kit's usage exit), it would exit the
      # same way each start, so it stays stopped and says why once.
      RestartPreventExitStatus = "64";
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
      NoNewPrivileges = true;
      RestrictAddressFamilies = [
        "AF_INET"
        "AF_INET6"
        "AF_NETLINK"
      ];
    };
  };
}
