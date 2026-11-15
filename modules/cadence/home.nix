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
# the runs from nix-offload's record, the net's round from its state, and
# from each realm's mycelium node, the other machines whose newest update
# failed, so it needs no builder and holds nothing between ticks.
#
# Given `revive.restart`, each tick first starts again every orchestrator
# and every orchestrator's session whose pi died, in a terminal on the
# account's own PATH, so a crash overnight costs minutes, not the night.
# Given an orchestrator, it also asks each orchestrator about any session of its
# that has sat idle two hours, once for each idle spell, so a session that
# finished its piece doesn't wait the night for its next. And it tells the
# orchestrator, and the unit's owner, of any of the account's units
# restarting in a loop (three restarts in fifteen minutes), failed, or a
# daemon whose socket reads a source signed out, unreachable or refused, or
# doesn't answer, once for each spell, with since when and its last error:
# on 13 November slack-unreads restarted 44 times and Google read signed out
# for two hours before anyone knew. A unit's owner is the one `owners` names
# for `unit:<unit>`, else for `unstamped:<unit>`, else the watcher.
#
# Each evening at 18:00, `cadence day` reads the day just ended (crashes,
# failed units, out-of-memory kills, pressure, switches, each
# repository's figures that modules/code-health wrote, and the machine's
# that modules/health wrote: boots, errors by kind, memory, agents that
# died and the worst moments) against the seven
# before it, keeps it in day/ beside them, and sends the orchestrator a
# note of what got worse and better, so a regression is named the day it
# appears.
# Each finding stays in findings.jsonl until its figure is back two days,
# goes once to the owner `owners` names for it, and to the orchestrator
# once more if it's still open past its date.
#
# Each Friday at 18:00, `cadence review` sends the week's noise (what
# nobody acted on or came again, by sender) and blind spots (where the
# axes and Joel's pains disagree) to the orchestrator and to the owner
# `owners` names for `review`.
#
# Each hour at ten past, `cadence hour` measures what shipped, what broke,
# what slipped and how clean the code is against their targets, names the
# one biggest drag and files it with its owner in findings.jsonl, says
# whether last hour's moved, and sends the card to the orchestrator.
#
# cadence-spec (./spec) serves the axes to the launcher's Spec place over
# $XDG_RUNTIME_DIR/daemons/cadence-spec.sock in the shell's daemon interface,
# asking `cadence axes --served` for the rows at most every five minutes
# and only while Spec is watched, so the launcher rereads no file (Joel,
# 2026-11-10: "the daemons need to be the data layer"). Spec.qml is that
# place, inside the launcher's Agents after Budgets, saying how many axes
# are out.
{ quickshell }:
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.cadence;
  package = pkgs.callPackage ./package.nix { };
  spec = pkgs.callPackage ./spec/package.nix { };
  # Who Friday's review goes to: the orchestrator, and the owner of `review`.
  reviewed = lib.filter (address: address != null) [
    cfg.orchestrator
    (cfg.owners.review or null)
  ];
  # Who owns what, by the prefix of its key, for the tick's units and the
  # day's findings alike.
  owning = lib.optionals (cfg.orchestrator != null) (
    lib.concatLists (
      lib.mapAttrsToList (prefix: address: [
        "--owner"
        "${prefix}=${address}"
      ]) cfg.owners
    )
  );
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
  # The realms this machine's mycelium nodes keep, where it has any, whose
  # other machines' failed updates each tick says: on 15 November the work
  # Mac's had failed for hours before Joel saw it idle.
  realms = lib.optionals (options.jitsusama ? mycelium) (
    lib.optional (config.jitsusama.mycelium.realm != null) config.jitsusama.mycelium.realm
    ++ lib.attrNames config.jitsusama.mycelium.alsoJoins
  );
  reading = lib.concatMap (realm: [
    "--realm"
    realm
  ]) realms;
  # The account's own programs, mycelium among them, for reviving and for
  # asking a realm's node.
  onProfile = reviving || realms != [ ];
in
{
  # A machine still setting the orchestrator by its old name, a word the
  # glossary replaces, keeps working, told to rename it.
  imports = [
    quickshell
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
        watcher. A broken unit the tick finds goes to the owner of
        `unit:<unit>`, else of `unstamped:<unit>`, as `unit:google.service`.
        The key `review` names who gets Friday's `cadence review` beside the
        orchestrator.
      '';
    };
    checkouts = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      example = {
        mycelium = "/home/me/src/mycelium";
      };
      description = ''
        The checkouts whose mains say which of the misses' checks merged,
        by the name a check calls each by, beside the core and harness
        mirrors cadence keeps itself: a miss naming `mycelium #177` closes
        once that pull request merged on the checkout's `origin/main`, or
        its `main`. Nothing is fetched; the main is read as it stands.
      '';
    };
    path = lib.mkOption {
      type = lib.types.listOf (lib.types.either lib.types.package lib.types.str);
      default = [ ];
      description = ''
        Programs the hour finds first on its PATH, ahead of the git core
        gives it, for a checkout only a machine's own git reads, such as
        one whose repository needs an extension stock git refuses: packages,
        or the folders of profiles kept outside the store, each of whose
        bin is searched.
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
          ++ owning
          ++ revive
          ++ reading
        );
        # git and journalctl, as the account has them; to revive, the
        # terminal, shell and holder it starts a session with; and the
        # node it asks about each realm's machines.
        Environment = [
          "PATH=${
            lib.makeBinPath [
              pkgs.git
              pkgs.systemd
            ]
          }${lib.optionalString onProfile ":${config.home.profileDirectory}/bin:/run/current-system/sw/bin"}"
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
            ++ owning
          )
        );
        # journalctl and coredumpctl, as the account has them.
        Environment = [ "PATH=${lib.makeBinPath [ pkgs.systemd ]}" ];
        Slice = "background.slice";
      };
    };
    systemd.user.services.cadence-review = {
      Unit.Description = "Reviews the week's noise and blind spots";
      Service = {
        Type = "oneshot";
        ExecStart = lib.escapeShellArgs (
          [
            (lib.getExe package)
            "review"
          ]
          ++ lib.concatMap (address: [
            "--to"
            address
          ]) reviewed
        );
        # git, systemctl and niri, for the axes it reads beside the week:
        # niri's windows are scale's figure, and the user manager hands
        # the service his session's niri socket. Without niri the review
        # of 13 November called scale unmeasured. niri is the machine's,
        # as its own module builds it, where it has one.
        Environment = [
          "PATH=${
            lib.makeBinPath [
              pkgs.git
              pkgs.systemd
            ]
          }:/run/current-system/sw/bin"
        ];
        Slice = "background.slice";
      };
    };
    # A daemon of its own, named cadence-spec, since every daemon's units
    # and program say its socket's name (docs/daemons.md) and cadence is
    # the tick's.
    systemd.user.services.cadence-spec = {
      Unit = {
        Description = "Serves the axes to the launcher's Spec";
        X-SwitchMethod = "restart";
        # Its socket is systemd's, handed to it as it starts.
        Requires = [ "cadence-spec.socket" ];
        After = [ "cadence-spec.socket" ];
      };
      Service = {
        ExecStart = lib.concatStringsSep " " [
          "@${lib.getExe spec}"
          "cadence-spec"
          "%t/daemons/cadence-spec.sock"
          (lib.getExe package)
          "${(import ../quickshell/launcher/words.nix).file}"
        ];
        # git, systemctl and niri for the axes cadence reads, as the review
        # has.
        Environment = [
          "PATH=${
            lib.makeBinPath [
              pkgs.git
              pkgs.systemd
            ]
          }:/run/current-system/sw/bin"
        ];
        Restart = "always";
        RestartSec = 5;
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };
    # Spec in the launcher, inside Agents after Budgets, with no row on
    # home (VKCF3Z): the axes the daemon serves, its head saying how many
    # are out.
    jitsusama.quickshell.launcher.providers.spec = {
      order = 54;
      source = ./Spec.qml;
    };

    # Its socket, held by systemd whether or not the daemon runs, so the
    # shell asking while it starts waits on it rather than finding none.
    systemd.user.sockets.cadence-spec = {
      Unit.Description = "Where the launcher asks for the axes";
      Socket = {
        ListenStream = "%t/daemons/cadence-spec.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.cadence-hour = {
      Unit.Description = "Measures the hour's shipping, quality and clean zones";
      Service = {
        Type = "oneshot";
        ExecStart = lib.escapeShellArgs (
          [
            (lib.getExe package)
            "hour"
          ]
          ++ lib.optionals (cfg.orchestrator != null) [
            "--to"
            cfg.orchestrator
          ]
          ++ lib.concatLists (
            lib.mapAttrsToList (name: path: [
              "--checkout"
              "${name}=${path}"
            ]) cfg.checkouts
          )
          ++ reading
        );
        # git for the three mains, and journalctl for update-to-main's
        # looks at them, as the axes read them. A machine's own git comes
        # first, for a checkout stock git refuses to open. The account's
        # mycelium, last, says how crowded each realm's machines are.
        Environment = [
          "PATH=${
            lib.makeBinPath (
              cfg.path
              ++ [
                pkgs.git
                pkgs.systemd
              ]
            )
          }${lib.optionalString (realms != [ ]) ":${config.home.profileDirectory}/bin"}"
        ];
        Slice = "background.slice";
      };
    };
    systemd.user.timers.cadence-hour = {
      Unit.Description = "Measures the hour at ten past each hour";
      Timer = {
        OnCalendar = "*-*-* *:10:00";
        AccuracySec = "30s";
      };
      Install.WantedBy = [ "timers.target" ];
    };
    systemd.user.timers.cadence-review = {
      Unit.Description = "Reviews the week each Friday evening";
      Timer = {
        OnCalendar = "Fri *-*-* 18:00:00";
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
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
