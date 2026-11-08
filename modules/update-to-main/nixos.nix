# update-to-main on NixOS: root's service, on a socket only root and the
# update-ask group write, that updates the machine to its repository's main
# when asked (system.nix says what it's told). A machine imports this and
# says its repository, its system and whose signatures count.
#
# The service starts from the running system's path with the config at a
# fixed one, and a switch never restarts it: it's what's switching, and it
# starts itself again once the switch is done. It never starts or switches
# while a call is on, as calls counts one, since a switch restarts the
# sound server under it.
#
# A follower, when one is named, is that account's service asking whenever
# main moves, so a merge needn't wait for someone to ask (src/follow.rs).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.update-to-main;
in
{
  imports = [ ./system.nix ];

  options.jitsusama.update-to-main.follower = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "someone";
    description = ''
      The account, one of the askers, whose service asks for an update
      whenever main or a followed input moves, and at least every ten
      minutes; none to ask only when someone does.
    '';
  };

  config = {
    assertions = [
      {
        assertion = cfg.follower == null || lib.elem cfg.follower cfg.askers;
        message = "jitsusama.update-to-main.follower has to be one of its askers.";
      }
    ];

    jitsusama.update-to-main.system = {
      kind = "nixos";
      # Where an asker grants a switch a held daemon's restart, in a
      # folder only root and the askers write.
      hold_given = "/run/update-to-main-hold/given";
      state = "/var/lib/update-to-main";
      socket = "/run/update-to-main/ask.sock";
      ended = "/run/update-to-main/last.json";
      running = "/run/update-to-main/running.json";
      profile = "/nix/var/nix/profiles/system";
      booted = "/run/booted-system";
      power_supplies = "/sys/class/power_supply";
      cpu_pressure = "/proc/pressure/cpu";
      tools = {
        nix = lib.getExe' config.nix.package "nix";
        nix_env = lib.getExe' config.nix.package "nix-env";
        pmset = null;
      };
    };

    jitsusama.update-to-main.refuseWhile = lib.mkBefore [
      {
        command = [ (lib.getExe (pkgs.callPackage ../calls/package.nix { })) ];
        why = "a call is on";
      }
    ];

    users.groups.update-ask.members = cfg.askers;
    systemd.tmpfiles.rules = [ "d /run/update-to-main-hold 0770 root update-ask -" ];

    systemd.user.services.update-to-main-follows = lib.mkIf (cfg.follower != null) {
      description = "Asks for an update whenever main moves";
      wantedBy = [ "default.target" ];
      unitConfig.ConditionUser = cfg.follower;
      serviceConfig = {
        # It ends once the system's path holds another update-to-main, so
        # coming back starts the newest.
        ExecStart = "/run/current-system/sw/bin/update-to-main follow";
        Restart = "always";
        RestartSec = 30;
      };
    };

    systemd.services.update-to-main = {
      description = "Updates the machine to its repository's main when asked";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      restartIfChanged = false;
      stopIfChanged = false;
      serviceConfig = {
        ExecStart = "/run/current-system/sw/bin/update-to-main serve --config /etc/update-to-main/config.json";
        Restart = "always";
        RestartSec = 5;
        StateDirectory = "update-to-main";
        StateDirectoryMode = "0700";
        RuntimeDirectory = "update-to-main";
        RuntimeDirectoryMode = "0755";
        # A switch it runs may restart much of the machine, but never what
        # it started: the switch is its child.
        KillMode = "process";
      };
    };
  };
}
