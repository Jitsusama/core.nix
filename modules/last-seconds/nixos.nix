# Keeps what the machine did in each boot's last ten minutes on the disk,
# a second at a time, for after a reset or a loss of power.
#
# When the XPS 14 reset itself on 17 and 20 November, the journal's last
# minutes went with it, so nothing said what the processor drew, how hot it
# ran or what waited in the seconds before. last-seconds-keep writes each
# second's reading to a ring in /var/lib/last-seconds/<boot id>/ and flushes
# that record alone; last-seconds reads the rings back for the wheel group
# (src/lib.rs says why, and src/ring.rs what it costs the disk).
#
# The recorder runs as an account of its own with nothing beyond reading
# the machine and writing its folder. The energy counters are root's alone,
# since readings milliseconds apart could tell what a processor computed,
# so a rule gives the package's and the platform's to its group alone; a
# second's average says nothing of the kind. The rule takes hold as the
# zones appear, at the next boot; until then the records leave energy out.
#
# It starts as soon as its folder is there, before the machine's services.
# It's ordered before basic.target, which every service starts after and
# which stops at a shutdown, so it stops only once the services have: its
# last record, saying it was stopped, is the shutdown's own. A boot whose
# newest record doesn't say so ended with a reset or a cut. It never gives
# up restarting, so a boot it stopped writing in reads as such, not as a
# reset at the moment it stopped, and backs off to a minute between tries,
# so a recorder that fails at every start can't fill the journal. A sleep's
# hook tells it the machine is going to sleep, so a boot that never woke
# says so (sleeps.sh).
#
# A switch that adds it makes its folder in systemd-tmpfiles-resetup, which
# switch-to-configuration restarts and waits on before it starts new units;
# the ordering after it holds where both are in one job.
{ config, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
  inherit (pkgs) coreutils;
  sleeps = pkgs.writeShellApplication {
    name = "last-seconds-sleeps";
    # systemd-sleep runs it with a PATH that has no coreutils; it reads the
    # ring with the reader, as the recorder's account, to see its record
    # land.
    runtimeInputs = [
      config.systemd.package
      coreutils
      package
      pkgs.jq
      pkgs.util-linux
    ];
    text = builtins.readFile ./sleeps.sh;
  };
  folder = "/var/lib/last-seconds";
  account = "last-seconds";
in
{
  users.users.${account} = {
    isSystemUser = true;
    group = account;
    description = "Keeps each boot's last seconds";
  };
  users.groups.${account} = { };

  # The package's and the platform's zones by name, as the recorder finds
  # them, whatever number they load as; not the cores' within them.
  services.udev.extraRules = ''
    ACTION=="add", SUBSYSTEM=="powercap", KERNEL=="intel-rapl:*", \
      ATTR{name}=="package-0|psys", \
      RUN+="${coreutils}/bin/chgrp ${account} $sys$devpath/energy_uj", \
      RUN+="${coreutils}/bin/chmod 0440 $sys$devpath/energy_uj"
  '';

  # setgid, so each boot's folder and ring is wheel's to read.
  systemd.tmpfiles.settings.last-seconds.${folder}.d = {
    mode = "2750";
    user = account;
    group = "wheel";
    age = "mM:90d";
  };

  systemd.services.last-seconds = {
    description = "Keeps what the machine does each second, for after a reset";
    wantedBy = [ "sysinit.target" ];
    after = [
      "local-fs.target"
      "systemd-tmpfiles-setup.service"
      "systemd-tmpfiles-resetup.service"
      # The energy counters' driver, which boot.kernelModules loads.
      "systemd-modules-load.service"
    ];
    before = [
      "sysinit.target"
      "basic.target"
      "shutdown.target"
    ];
    conflicts = [ "shutdown.target" ];
    unitConfig = {
      DefaultDependencies = false;
      StartLimitIntervalSec = 0;
    };
    serviceConfig = {
      ExecStart = "${package}/bin/last-seconds-keep";
      Restart = "always";
      RestartSec = 1;
      RestartSteps = 6;
      RestartMaxDelaySec = 60;
      User = account;
      Group = account;
      UMask = "0027";
      ReadWritePaths = [ folder ];
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
      PrivateDevices = true;
      PrivateNetwork = true;
      IPAddressDeny = "any";
      RestrictAddressFamilies = "none";
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      ProtectClock = true;
      ProtectHostname = true;
      # It reads /proc's pressure, memory and load, so /proc stays whole
      # but for other processes.
      ProtectProc = "invisible";
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged"
        "~@resources"
      ];
    };
  };

  environment.etc."systemd/system-sleep/last-seconds-sleeps".source =
    "${sleeps}/bin/last-seconds-sleeps";

  environment.systemPackages = [ package ];
}
