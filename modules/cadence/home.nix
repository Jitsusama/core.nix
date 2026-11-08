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
#
# Given `revive.restart`, each tick first starts again every orchestrator
# and every orchestrator's session whose pi died, in a terminal on the
# account's own PATH, so a crash overnight costs minutes, not the night.
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
  reviving = cfg.revive.restart != null && cfg.steward != null;
  revive = lib.optionals reviving (
    [
      "--restart"
      cfg.revive.restart
    ]
    ++ lib.concatMap (a: [
      "--orchestrator"
      a
    ]) cfg.revive.orchestrators
    ++ lib.concatMap (a: [
      "--skip"
      a
    ]) cfg.revive.skip
  );
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
    revive = {
      restart = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "pi --model provider/model";
        description = ''
          The words a login shell runs to start a died session, before
          `--session <id>`; null revives nothing. It needs `steward`, who
          is told of each start.
        '';
      };
      orchestrators = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "Mesh addresses revived as orchestrators, beside any that others name as theirs.";
      };
      skip = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "Mesh addresses never revived.";
      };
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
          ++ revive
        );
        # git and journalctl, as the account has them, and to revive, the
        # terminal, shell and holder it starts a session with.
        Environment = [
          "PATH=${
            lib.makeBinPath [
              pkgs.git
              pkgs.systemd
            ]
          }${lib.optionalString reviving ":${config.home.profileDirectory}/bin:/run/current-system/sw/bin"}"
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
