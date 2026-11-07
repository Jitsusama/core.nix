# The delivery cycle watched from outside every session: cadence, a program
# of core's own (src/main.rs), run every five minutes by a timer, writes the
# last hour's card to ~/.local/state/cadence/card.txt and, given the mesh
# addresses of the session keeping the cycle fast and of the steward above
# it, asks that session when a target in docs/delivery.md slips or its card
# is overdue, and asks the steward when the session doesn't answer or is
# gone. A session can die, park or lose its model; a timer keeps going.
#
# It reads core's merges from a bare clone of its own in the same folder,
# fetched each tick, the switches from update-to-main's follower's journal,
# the runs from nix-offload's record, and the net's round from its state,
# so it needs no builder and holds nothing between ticks.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.cadence;
  package = pkgs.callPackage ./package.nix { };
  asking = lib.optionals (cfg.watcher != null && cfg.steward != null) [
    "--watcher"
    cfg.watcher
    "--steward"
    cfg.steward
  ];
in
{
  options.jitsusama.cadence = {
    watcher = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The mesh address of the session keeping the delivery cycle fast, asked when a target slips.";
    };
    steward = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The mesh address asked when the watcher doesn't answer or is gone.";
    };
  };

  config = {
    home.packages = [ package ];

    systemd.user.services.cadence = {
      Unit.Description = "Watches the delivery cycle and writes its card";
      Service = {
        Type = "oneshot";
        ExecStart = lib.escapeShellArgs (
          [
            (lib.getExe package)
            "tick"
          ]
          ++ asking
        );
        # git and journalctl, as the account has them.
        Environment = [
          "PATH=${
            lib.makeBinPath [
              pkgs.git
              pkgs.systemd
            ]
          }"
        ];
        Slice = "background.slice";
      };
    };
    systemd.user.timers.cadence = {
      Unit.Description = "Watches the delivery cycle every five minutes";
      Timer = {
        OnCalendar = "*:0/5";
        AccuracySec = "30s";
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
