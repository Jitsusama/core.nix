# The delivery cycle watched from outside every session: cadence, a program
# of core's own (src/main.rs), run every five minutes by a timer, writes the
# last hour's card to ~/.local/state/cadence/card.txt and, given the mesh
# addresses of the session keeping the cycle fast and of the orchestrator above
# it, asks that session when a target in docs/delivery.md slips or its card
# is overdue, and asks the orchestrator when the session doesn't answer or is
# gone. A session can die, park or lose its model; a timer keeps going.
#
# It reads core's merges from a bare clone of its own in the same folder,
# fetched each tick, the switches from update-to-main's journal, whoever asked,
# the runs from nix-offload's record, and the net's round from its state,
# so it needs no builder and holds nothing between ticks.
#
# Given `revive.restart`, each tick first starts again every orchestrator
# and every orchestrator's session whose pi died, in a terminal on the
# account's own PATH, so a crash overnight costs minutes, not the night.
# Given an orchestrator, it also asks each orchestrator about any session of its
# that has sat idle two hours, once for each idle spell, so a session that
# finished its piece doesn't wait the night for its next.
#
# Each evening at 18:00, `cadence day` reads the day just ended (crashes,
# failed units, out-of-memory kills, pressure, switches, and each
# repository's figures that modules/code-health wrote) against the seven
# before it, keeps it in day/ beside them, and sends the orchestrator a
# note of what got worse and better, so a regression is named the day it
# appears.
# Each finding stays in findings.jsonl until its figure is back two days,
# goes once to the owner `owners` names for it, and to the orchestrator
# once more if it's still open past its date.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.cadence;
  package = pkgs.callPackage ./package.nix { };
  asking = lib.optionals (cfg.watcher != null && cfg.orchestrator != null) [
    "--watcher"
    cfg.watcher
    "--orchestrator"
    cfg.orchestrator
  ];
  reviving = cfg.revive.restart != null && cfg.orchestrator != null;
  revive = lib.optionals reviving (
    [
      "--restart"
      cfg.revive.restart
    ]
    ++ lib.concatMap (a: [
      "--revive"
      a
    ]) cfg.revive.orchestrators
    ++ lib.concatMap (a: [
      "--skip"
      a
    ]) cfg.revive.skip
  );
in
{
  # A machine still setting the orchestrator by its old name, a word the
  # glossary replaces, keeps working, told to rename it.
  imports = [
    (lib.mkRenamedOptionModule
      [ "jitsusama" "cadence" "steward" ]
      [ "jitsusama" "cadence" "orchestrator" ]
    )
  ];

  options.jitsusama.cadence = {
    watcher = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The mesh address of the session keeping the delivery cycle fast, asked when a target slips.";
    };
    orchestrator = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "The mesh address asked when the watcher doesn't answer or is gone.";
    };
    owners = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      example = {
        crashes = "ADDRESS";
        "switch." = "ADDRESS";
      };
      description = ''
        The mesh address each finding of `cadence day` goes to, by the
        prefix of its key, the longest matching: `crashes:biome`,
        `failed:quickshell.service`, `pressure.cpu.jobs`, `oom.machine`,
        `switch.minutes.longest`. A finding no prefix matches goes to the
        watcher.
      '';
    };
    revive = {
      restart = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "pi --model provider/model";
        description = ''
          The words a login shell runs to start a died session, before
          `--session <id>`; null revives nothing. It needs `orchestrator`, who
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
    systemd.user.services.cadence-day = {
      Unit.Description = "Reads the day just ended against the days before it";
      Service = {
        Type = "oneshot";
        ExecStart = lib.escapeShellArgs (
          [
            (lib.getExe package)
            "day"
          ]
          ++ lib.optionals (cfg.orchestrator != null) (
            [
              "--orchestrator"
              cfg.orchestrator
            ]
            ++ lib.optionals (cfg.watcher != null) [
              "--watcher"
              cfg.watcher
            ]
            ++ lib.concatLists (
              lib.mapAttrsToList (prefix: address: [
                "--owner"
                "${prefix}=${address}"
              ]) cfg.owners
            )
          )
        );
        # journalctl and coredumpctl, as the account has them.
        Environment = [ "PATH=${lib.makeBinPath [ pkgs.systemd ]}" ];
        Slice = "background.slice";
      };
    };
    systemd.user.timers.cadence-day = {
      Unit.Description = "Reads the day each evening";
      Timer = {
        OnCalendar = "*-*-* 18:00:00";
        # A laptop asleep at six reads its day when it wakes.
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
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
