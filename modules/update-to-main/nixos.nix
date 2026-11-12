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

  config = {
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
      # A switch cut off by shutdown leaves the machine between two systems:
      # on 8 November an ask was still running when Joel restarted, three
      # minutes after a switch.
      {
        command = [
          "${pkgs.writeShellScript "update-to-main-shutting-down" ''
            ${lib.getExe' config.systemd.package "systemctl"} list-jobs --no-legend \
              | ${lib.getExe pkgs.gnugrep} -qE \
                ' (shutdown|reboot|poweroff|halt|kexec|soft-reboot)\.target '
          ''}"
        ];
        why = "the machine is shutting down";
      }
      # A switch restarts the desktop's shell, and on 9 November a launcher
      # stopped taking keys after five switches the follower asked for in
      # half an hour. So one it asked for waits, once built, until nobody
      # has touched a key or the pointer for two minutes, which swayidle
      # marks (../swayidle/home.nix). That evening no such gap came for two
      # hours while 26 merges waited, so after 45 minutes it waits only
      # for the launcher to close, which the shell writes in its
      # quickshell-launcher file, and for no call. A machine without the
      # marks never waits.
      {
        command = [
          "${pkgs.writeShellScript "update-to-main-someone-is-here" ''
            for mark in /run/user/*/someone-at-the-machine; do
              test -e "$mark" && exit 0
            done
            exit 1
          ''}"
        ];
        why = "someone is at the machine";
        followerWaits = true;
        followerWaitsAtMost = 45 * 60;
      }
      {
        command = [
          "${pkgs.writeShellScript "update-to-main-the-launcher-is-open" ''
            for mark in /run/user/*/quickshell-launcher; do
              ${lib.getExe pkgs.gnugrep} -qx open "$mark" 2>/dev/null && exit 0
            done
            exit 1
          ''}"
        ];
        why = "the launcher is open";
        followerWaits = true;
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
