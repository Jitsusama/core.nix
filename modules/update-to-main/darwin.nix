# update-to-main on a Mac: root's launchd daemon, on a socket only root and
# the update-ask group write, that updates the Mac to its repository's main
# when asked (system.nix says what it's told). A machine imports this and
# says its repository, its system and whose signatures count.
#
# The daemon's definition names only the running system's path and a fixed
# config path, so it's the same in every system and activate never reloads
# it mid-switch; it starts itself again once a switch is done.
#
# launchd loads it before that path names a system holding it: at boot, and
# in the switch that first brings it. launchd never tries a missing program
# again, so the daemon waits for its program rather than be one.
#
# It never switches while a call is on, as calls counts one through
# CoreAudio. A follower, when one is named, is that account's launchd agent
# asking whenever main moves (src/follow.rs); a switch it asked for waits,
# once built, until nobody has touched a key or the pointer for two
# minutes, and for 45 minutes at most, as NixOS's does. When the Mac can't
# update, the follower says so in a notice (./tells-on-a-mac.sh), since
# nobody reads its log.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.update-to-main;
  program = "/run/current-system/sw/bin/update-to-main";
  # The machine's own Nix: nix-darwin's when it manages Nix, and otherwise
  # the installer's, as Determinate's is, never another beside it.
  nixBin =
    if config.nix.enable then
      "${lib.getBin config.nix.package}/bin"
    else
      "/nix/var/nix/profiles/default/bin";
in
{
  imports = [ ./system.nix ];

  config = {
    assertions = [
      {
        assertion = cfg.follower == null || cfg.follower == config.system.primaryUser;
        message = "jitsusama.update-to-main.follower has to be the Mac's primary user, the one nix-darwin's agents are for.";
      }
    ];

    jitsusama.update-to-main.refuseWhile = lib.mkBefore [
      {
        command = [ (lib.getExe (pkgs.callPackage ../calls/package.nix { })) ];
        why = "a call is on";
      }
      # HIDIdleTime is how long since any key or the pointer was touched, in
      # nanoseconds; with none read it can't tell.
      {
        command = [
          "${pkgs.writeShellScript "update-to-main-someone-is-here" ''
            idle=$(/usr/sbin/ioreg -c IOHIDSystem -d 4 -r -k HIDIdleTime \
              | /usr/bin/awk '/HIDIdleTime/ { print int($NF / 1000000000); exit }')
            test -n "$idle" || exit 2
            test "$idle" -lt 120
          ''}"
        ];
        why = "someone is at the machine";
        followerWaits = true;
        followerWaitsAtMost = 45 * 60;
      }
    ];

    jitsusama.update-to-main.followerTells = lib.mkDefault [
      "${pkgs.writeShellScript "update-to-main-tells" (builtins.readFile ./tells-on-a-mac.sh)}"
    ];

    jitsusama.update-to-main.system = {
      kind = "darwin";
      state = "/var/root/.local/state/update-to-main";
      socket = "/var/run/update-to-main.sock";
      ended = "/var/run/update-to-main.last.json";
      running = "/var/run/update-to-main.running.json";
      profile = "/nix/var/nix/profiles/system";
      # A Mac boots no NixOS system, says its power through pmset, and
      # keeps no CPU pressure, so nothing it builds elsewhere builds here.
      booted = "/var/empty";
      power_supplies = "/var/empty";
      cpu_pressure = "/var/empty";
      tools = {
        nix = "${nixBin}/nix";
        nix_env = "${nixBin}/nix-env";
        pmset = "/usr/bin/pmset";
      };
      # A Mac's keychain answers only in its account's login session, so a
      # private input is fetched there (src/session.rs). Each job's folder is
      # made in a folder of root's the account may pass through, since a
      # root daemon's own temporary folder is root's alone.
      session = {
        launchctl = "/bin/launchctl";
        jobs = "/var/run/update-to-main";
      };
    };

    # A group nix-darwin keeps, its id one macOS leaves free.
    users.knownGroups = [ "update-ask" ];
    users.groups.update-ask = {
      gid = 452;
      members = cfg.askers;
      description = "Accounts that may ask update-to-main to update the Mac";
    };

    launchd.daemons.update-to-main.serviceConfig = {
      Label = "org.jitsusama.update-to-main";
      ProgramArguments = [
        "/bin/sh"
        "-c"
        "until [ -x ${program} ]; do sleep 1; done; exec ${program} serve --config /etc/update-to-main/config.json"
      ];
      RunAtLoad = true;
      KeepAlive = true;
      ThrottleInterval = 10;
      # A switch it runs is its child, and launchd leaves it be if the
      # daemon itself stops.
      AbandonProcessGroup = true;
      StandardOutPath = "/var/log/update-to-main.log";
      StandardErrorPath = "/var/log/update-to-main.log";
    };

    # follow ends once the system's path holds another update-to-main, and
    # launchd starts again only what ends unsuccessfully, so it ends 75
    # for the newest to start.
    launchd.user.agents.update-to-main-follows = lib.mkIf (cfg.follower != null) {
      serviceConfig = {
        Label = "org.jitsusama.update-to-main-follows";
        ProgramArguments = [
          "/bin/sh"
          "-c"
          "until [ -x ${program} ]; do sleep 1; done; ${program} follow; exit 75"
        ];
        RunAtLoad = true;
        KeepAlive.SuccessfulExit = false;
        ThrottleInterval = 30;
        StandardOutPath = "/Users/${cfg.follower}/Library/Logs/update-to-main-follows.log";
        StandardErrorPath = "/Users/${cfg.follower}/Library/Logs/update-to-main-follows.log";
      };
    };
  };
}
