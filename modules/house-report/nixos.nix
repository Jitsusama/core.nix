# Runs house-report on a Linux machine that shares a line with a shaped
# one, as a system service with an account of its own and nothing it may
# write (system.nix says what it's told).
{ config, lib, ... }:
let
  inherit (config.jitsusama.house-report) keyFile;
in
{
  imports = [ ./system.nix ];

  # Its account is made for each start and may read nothing of root's, so
  # systemd hands it the key as a credential, readable by it alone. The
  # credential is a copy made as the service starts, so a key put there or
  # swapped restarts it (`house-report-key`), and one missing at start
  # fails the start, said by systemd, until it's there five seconds on.
  jitsusama.house-report.keyPath = lib.mkIf (keyFile != null) "%d/house.key";

  systemd.paths.house-report-key = lib.mkIf (keyFile != null) {
    description = "Watches the house's key, so house-report signs with the one there";
    wantedBy = [ "multi-user.target" ];
    pathConfig.PathChanged = keyFile;
  };

  # A key written in several steps fires the path unit several times, and
  # each start counts toward systemd's start limit, past which neither
  # unit starts again and the reporter stays down; RestartSec already
  # spaces the reporter's own starts, so neither has a limit (house
  # signing round 4's F4, F10, F15 and F25).
  systemd.services.house-report-key = lib.mkIf (keyFile != null) {
    description = "Restarts house-report on the house's key as it stands";
    startLimitIntervalSec = 0;
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${config.systemd.package}/bin/systemctl try-restart house-report.service";
    };
  };

  systemd.services.house-report = {
    description = "Tells the shaped machine on this line what this one sends up";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    startLimitIntervalSec = 0;
    serviceConfig = {
      ExecStart = lib.escapeShellArgs config.jitsusama.house-report.arguments;
      LoadCredential = lib.mkIf (keyFile != null) "house.key:${keyFile}";
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
