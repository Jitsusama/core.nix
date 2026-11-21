# Keeps the cores from their deepest rest, C6S and C10, from boot, and
# says in the journal how long they rest in each state, so it shows
# whether the kernel kept to the hold.
#
# Twice on 20 November the firmware reset the laptop after a core's fatal
# internal-timer machine check. Intel's errata (Core Ultra Series 3
# Specification Update, PTL039) name such a hang when several P-cores
# leave a low-power state at once, and another owner of this model found
# that keeping the cores out of C6S and C10 held through tests that reset
# it before (docs/hardware.md). While the hold runs, it asks the kernel to
# keep every core able to come out of rest within 20 us. rest-states.json,
# beside this, is the laptop's cpuidle table as it read on 20 November,
# the same on each of its 16 cores: C1E's exit latency is 10 us and C6S's
# 300, so the cores rest no deeper than C1E, and
# the-deep-rest-hold-stops-short-of-c6 fails if the bound or that copy
# moves past it. The kernel's own table can change with the kernel, so
# the journal's line gives each state's exit latency as the kernel has it
# now. A suspend still reaches the chip's own deepest rest. What the hold
# costs hasn't been measured here: with no core in C6, the package can't
# reach its own deep states either, so a laptop left alone draws more.
#
# The hold starts with the machine unless
# jitsusama.dellXps14.keepCoresFromDeepRest is false, which takes effect
# from the next boot. `systemctl stop cores-kept-from-deep-rest` ends it at
# once, until the next boot or switch, since a switch starts what
# multi-user.target wants again, and `start` brings it back.
# deep-rest-hold.sh, beside this, holds the bound; a start returns only
# once the kernel's bound reads it back, and fails otherwise.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # How soon every core must be able to come out of rest, in microseconds.
  bound = 20;
  # Neither unit takes any input or needs anything of the system but what
  # each names, so each is shut out of the rest.
  confined = {
    CapabilityBoundingSet = "";
    NoNewPrivileges = true;
    ProtectSystem = "strict";
    ProtectHome = true;
    PrivateTmp = true;
    PrivateNetwork = true;
    ProtectKernelModules = true;
    ProtectKernelTunables = true;
    ProtectKernelLogs = true;
    ProtectControlGroups = true;
    ProtectClock = true;
    ProtectHostname = true;
    ProtectProc = "invisible";
    ProcSubset = "pid";
    RestrictNamespaces = true;
    RestrictRealtime = true;
    RestrictSUIDSGID = true;
    LockPersonality = true;
    MemoryDenyWriteExecute = true;
    SystemCallArchitectures = "native";
    SystemCallFilter = "@system-service";
  };
in
{
  options.jitsusama.dellXps14.keepCoresFromDeepRest = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = ''
      Whether the cores are kept from their deepest rest from boot, a test
      of whether that stops the resets. False leaves the hold to
      `systemctl start cores-kept-from-deep-rest`, from the next boot; a
      hold already running ends with `systemctl stop cores-kept-from-deep-rest`.
    '';
  };

  config = {
    systemd.services.cores-kept-from-deep-rest = {
      description = "Keeps the cores from their deepest rest";
      wantedBy = lib.mkIf config.jitsusama.dellXps14.keepCoresFromDeepRest [ "multi-user.target" ];
      environment.BOUND_US = toString bound;
      path = [
        pkgs.coreutils
        config.systemd.package
      ];
      serviceConfig = confined // {
        Type = "notify";
        # The script's own systemd-notify, a child, says it's ready.
        NotifyAccess = "all";
        ExecStart = "${lib.getExe pkgs.bash} ${./deep-rest-hold.sh}";
        # A killed sleep, by any signal, would drop the hold without a
        # word, so it comes back; a bound that doesn't take exits 1 and
        # stays failed, and a stop is never restarted.
        Restart = "always";
        RestartPreventExitStatus = 1;
        # Root owns the file, so it opens it without any capability.
        DeviceAllow = "/dev/cpu_dma_latency rw";
        DevicePolicy = "closed";
        # Only the socket systemd-notify speaks to systemd through.
        RestrictAddressFamilies = "AF_UNIX";
      };
    };

    systemd.services.cores-rest-residency = {
      description = "Says how long the cores have rested in each state";
      serviceConfig = confined // {
        Type = "oneshot";
        ExecStart = "${lib.getExe pkgs.bash} ${./rest-residency.sh}";
        DynamicUser = true;
        DevicePolicy = "closed";
        RestrictAddressFamilies = "none";
      };
    };

    # Every five minutes is a line in the journal small enough to keep for
    # weeks, and fine enough to see a hold start or stop.
    systemd.timers.cores-rest-residency = {
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnBootSec = "5min";
        OnUnitActiveSec = "5min";
      };
    };
  };
}
