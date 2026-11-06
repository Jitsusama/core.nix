# Keeps the clock with chrony, on any network. chrony asks every server it's
# given at once and keeps the clock by those that agree, so a network's own
# server that never answers, as a hotel's didn't, is only out of reach while
# the machine's own servers set the clock. It's told the servers a network
# names as networkd learns them (network-servers.sh), beside the machine's
# own, the pool of public servers.
#
# It's chrony rather than systemd-timesyncd because Time shows every server
# the clock is kept by, each server's figures and why it's chosen or set
# aside, which timesyncd, asking one server at a time, never says (P9R1KI,
# 2026-10-30). Those are read from chrony's command port by the shell's
# timekeeping reader, never by running chronyc.
{ pkgs, ... }:
let
  folder = "/run/chrony-network";
  told = pkgs.writeShellApplication {
    name = "chrony-network-servers";
    runtimeInputs = [
      pkgs.chrony
      pkgs.coreutils
      pkgs.diffutils
      pkgs.gnused
    ];
    text = builtins.readFile ./network-servers.sh;
  };
in
{
  services.chrony = {
    enable = true;
    # The module's own choice stands: chrony keeps the hardware clock's
    # drift in its file and trims the hardware clock when it's far out
    # (enableRTCTrimming), rather than having the kernel write it every
    # eleven minutes (rtcsync), which can't be had beside it. It's what
    # lets Time say how far the hardware clock is out and how fast it
    # drifts.
    #
    # Beside the commands chrony answers anyone on the machine, the ones
    # Time reads each server's own figures and why it's chosen by. Its
    # command port answers this machine alone.
    extraConfig = ''
      sourcedir ${folder}
      opencommands activity manual rtcdata smoothing sourcename sources sourcestats tracking ntpdata selectdata
    '';
  };

  systemd.tmpfiles.rules = [ "d ${folder} 0755 root root -" ];

  # Each time networkd writes a link's state, as it does when the link
  # comes up or a lease names its servers, and once as the machine starts.
  systemd.paths.chrony-network-servers = {
    description = "Hears the time servers the networks name";
    wantedBy = [ "multi-user.target" ];
    pathConfig.PathChanged = "/run/systemd/netif/links";
  };
  systemd.services.chrony-network-servers = {
    description = "Tells chrony the time servers the networks name";
    wantedBy = [ "multi-user.target" ];
    after = [ "chronyd.service" ];
    # networkd writes link states in bursts as a machine starts, six in
    # under three seconds on a laptop, past systemd's five starts in ten;
    # stopped there, it never heard the networks' servers until a restart.
    # Each run is a read and at most a reload, so it has no limit.
    startLimitIntervalSec = 0;
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${told}/bin/chrony-network-servers";
      # It writes its own folder, and chronyc's socket beside chrony's.
      ProtectSystem = "strict";
      ReadWritePaths = [
        folder
        "/run/chrony"
      ];
      ProtectHome = true;
      # chronyc reaches chrony by its socket alone, in chrony's own folder,
      # which only root's overriding of permissions enters.
      PrivateNetwork = true;
      NoNewPrivileges = true;
      CapabilityBoundingSet = [
        "CAP_DAC_OVERRIDE"
        "CAP_CHOWN"
        "CAP_FOWNER"
      ];
      RestrictAddressFamilies = [ "AF_UNIX" ];
    };
  };
}
