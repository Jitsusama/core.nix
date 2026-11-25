# night-shift runs an account's heavy upkeep at night, once a night, only
# while the machine has memory to spare, and stops it the moment what Joel
# uses waits for memory (src/main.rs says how). A module declares a job:
#
#   jitsusama.night-shift.jobs.repack = {
#     description = "Repacks the repository's newest packs";
#     command = [ "git" "-C" "/home/someone/repo" "maintenance" "run" "--task=incremental-repack" ];
#     memoryHigh = "12G";
#   };
#
# and it is tried every quarter hour from 01:30 to 05:00, starting only
# while the machine's full memory pressure over the last minute is under
# 3%, never beside another night job, and never outside its hours, which a
# start by a switch can be. It is stopped if the apps' or the
# desktop's slice passes 1% over ten seconds, agents' sessions' 10%, or the
# machine's 25%. A job's log is its unit's journal; each sample it took is
# in ~/.local/state/night-shift/NAME.samples, and NAME.done is written when
# it succeeds. The job's limits hold only the job: night-shift watches from
# a cgroup beside it, so a job held at its limit never stalls its watcher.
# The unit is delegated memory and io for that.
#
# A heavy job by day, a test suite or a machine's evaluation, runs as
# `night-shift turn NAME -- COMMAND`: it waits for the turn a night job
# holds, or another heavy job's, rather than starting beside it, and a
# night job tried meanwhile leaves the turn to a later try. One Joel waits
# on, such as a switch's build, adds `--ahead` and goes next; a night job
# running then pauses for it, its cgroup frozen, and goes on once it
# ends, before any job that came to wait meanwhile. A pause holds only
# the job's own processes, not a build it handed nix-daemon, and a stop
# while paused lets it go on for its grace to hear it. A job that can run
# long names `runsAtMost`, the time it may run, its pauses aside. So
# night-shift is on every account's path.
#
# A job can ease a slice's bar while Joel is away from the machine, with
# `stopsWhileAway`, and the machine's with `stopMachineWhileAway`: a
# browser left open stalls app.slice a little now and then with nobody at
# it, which stopped a repack 19 times on 23 November's night. Away is
# `away.after` with no key or touch and no call on this machine, as
# swayidle-away's mark, rhythm's minutes and the calls module say; the
# moment he's back, the job's own bars hold again (src/away.rs).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  jobs = config.jitsusama.night-shift.jobs;
  away = config.jitsusama.night-shift.away;
  # A path as systemd's command line reads it, its specifiers kept so %t
  # and %S name the account's own folders; its type holds no space or
  # quote, so it needs no quoting.
  specifiedArg = lib.replaceStrings [ "$" ] [ "$$" ];
  specifiedPath = lib.types.strMatching "[^[:space:]\"'\\;]+";
  eases = job: job.stopsWhileAway != { } || job.stopMachineWhileAway != null;
  awayArgs =
    job:
    lib.optionals (eases job) (
      [
        "--away-mark"
        (specifiedArg away.mark)
        "--away-seen"
        (specifiedArg away.seen)
        "--away-after"
        (number away.after)
      ]
      ++ lib.optionals (away.call != [ ]) [
        "--away-call"
        (execArg (lib.concatStringsSep " " away.call))
      ]
      ++ lib.concatLists (
        lib.mapAttrsToList (slice: bar: [
          "--away-stop"
          (execArg "${slice}=${number bar}")
        ]) job.stopsWhileAway
      )
      ++ lib.optionals (job.stopMachineWhileAway != null) [
        "--away-stop-machine"
        (number job.stopMachineWhileAway)
      ]
    );

  # One argument as systemd's command line reads it: its own specifiers
  # and variables escaped, then quoted only when systemd would split it,
  # unescape it or read it as the end of a command.
  execArg =
    arg:
    let
      escaped = lib.replaceStrings [ "%" "$" ] [ "%%" "$$" ] arg;
    in
    if builtins.match "[^[:space:]\"'\\;]+" escaped != null then
      escaped
    else
      lib.escapeShellArg escaped;

  number = toString;

  # The tries a job has unless it names its own, and the hours they keep
  # within.
  nightTries = [
    "*-*-* 01:30/15"
    "*-*-* 02..04:00/15"
    "*-*-* 05:00"
  ];
  nightHours = "01:30-05:15";

  # A job naming its own tries gets no hours it didn't name: core can't
  # see when those tries fall, so hours of its own would quietly stop it.
  hoursOf =
    job:
    if job.hours != null then
      job.hours
    else if job.tries == nightTries then
      nightHours
    else
      null;

  clockTime = "([01][0-9]|2[0-3]):[0-5][0-9]";

  environment = job: lib.mapAttrsToList (variable: value: "${variable}=${value}") job.environment;

  execStart =
    name: job:
    lib.concatMapStringsSep " " execArg (
      [
        (lib.getExe package)
        "run"
        name
      ]
      ++ lib.optionals (hoursOf job != null) [
        "--hours"
        (hoursOf job)
      ]
      ++ [
        "--start-under"
        (number job.startUnder)
      ]
      ++ lib.concatLists (
        lib.mapAttrsToList (slice: bar: [
          "--stop"
          "${slice}=${number bar}"
        ]) job.stops
      )
      ++ [
        "--stop-machine"
        (number job.stopMachine)
        "--rests"
        (number job.restsHours)
      ]
      ++ lib.optionals (job.waitsAtMost != null) [
        "--wait-at-most"
        (number job.waitsAtMost)
      ]
      ++ lib.optionals (job.runsAtMost != null) [
        "--runs-at-most"
        (number job.runsAtMost)
      ]
      ++ lib.optionals (job.memoryHigh != null) [
        "--memory-high"
        job.memoryHigh
      ]
      ++ lib.optionals (job.memoryMax != null) [
        "--memory-max"
        job.memoryMax
      ]
      ++ lib.optionals (job.swapMax != null) [
        "--swap-max"
        job.swapMax
      ]
      ++ lib.concatLists (
        lib.mapAttrsToList (disk: rate: [
          "--reads-at-most"
          "${disk}=${rate}"
        ]) job.readsAtMost
      )
    )
    + lib.optionalString (eases job) " ${lib.concatStringsSep " " (awayArgs job)}"
    + " "
    + lib.concatMapStringsSep " " execArg ([ "--" ] ++ job.command);

  jobOptions = {
    options = {
      description = lib.mkOption {
        description = "What the job does, as its unit says.";
        type = lib.types.str;
      };
      command = lib.mkOption {
        description = "The job's program and its arguments, run as they're given, with no shell.";
        type = lib.types.nonEmptyListOf lib.types.str;
      };
      tries = lib.mkOption {
        description = ''
          When a start is tried, as systemd's calendar reads it. A try after
          the job succeeded that night does nothing.
        '';
        type = lib.types.nonEmptyListOf lib.types.str;
        default = nightTries;
      };
      hours = lib.mkOption {
        description = ''
          When a start may run the job, by the local clock, as FROM-UNTIL,
          running past midnight when UNTIL comes first; a start outside
          them runs nothing and isn't a failure. A switch can start the
          timer, and the timer the job, at any hour, so keep every try
          within them. Null gives a job on the default tries
          ${nightHours}, and a job naming its own tries no hours at all,
          as before hours were kept; the nearest a job on the default
          tries comes to none is 00:00-23:59.
        '';
        type = lib.types.nullOr (
          lib.types.addCheck (lib.types.strMatching "${clockTime}-${clockTime}") (
            hours:
            let
              ends = lib.splitString "-" hours;
            in
            # Hours ending as they begin would hold no minute, so the job
            # would never run.
            lib.elemAt ends 0 != lib.elemAt ends 1
          )
          // {
            description = "two clock times, HH:MM-HH:MM, that differ";
          }
        );
        default = null;
        example = "22:00-02:00";
      };
      startUnder = lib.mkOption {
        description = "The machine's full memory pressure over the last minute, in percent, it starts only under.";
        type = lib.types.number;
        default = 3;
      };
      stops = lib.mkOption {
        description = ''
          The slices of the account that stop the job, each at the full
          memory pressure over ten seconds, in percent, it may not pass.
        '';
        type = lib.types.attrsOf lib.types.number;
        default = {
          "app.slice" = 1;
          "session.slice" = 1;
          "agents.slice" = 10;
        };
      };
      stopMachine = lib.mkOption {
        description = ''
          The machine's full memory pressure over ten seconds, in percent,
          that stops the job: a backstop, since a job stalling at its own
          limit counts in it.
        '';
        type = lib.types.number;
        default = 25;
      };
      stopsWhileAway = lib.mkOption {
        description = ''
          Eased bars for slices in `stops`, each holding in place of its own
          while Joel is away from the machine, as `away` tells; the moment
          he's back, its own bar holds again.
        '';
        type = lib.types.attrsOf lib.types.number;
        default = { };
        example = {
          "app.slice" = 10;
          "session.slice" = 10;
        };
      };
      stopMachineWhileAway = lib.mkOption {
        description = ''
          An eased bar for the machine, in place of `stopMachine` while Joel
          is away; null keeps `stopMachine` whether he's here or not.
        '';
        type = lib.types.nullOr lib.types.number;
        default = null;
      };
      restsHours = lib.mkOption {
        description = "How long after a success a try does nothing, in hours.";
        type = lib.types.number;
        default = 12;
      };
      waitsAtMost = lib.mkOption {
        description = ''
          How long a try waits for the turn another heavy job holds, in
          seconds, rather than leaving at once; it starts once the turn is
          free and the machine quiet. A try that waits this long leaves
          with 75, which its unit counts a success, and the next try comes
          as the timer says. The wait counts toward `longest`, so keep it
          well under that: at or past it, systemd stops the try as a
          failed unit before it gives up.
        '';
        type = lib.types.nullOr lib.types.ints.positive;
        default = null;
      };
      runsAtMost = lib.mkOption {
        description = ''
          How long the job may run, in seconds, not counting the time it
          spends paused for a job Joel waits on. Past it, night-shift stops
          the job and the try ends as a success with no `.done`, so a later
          try runs it again. Leave it null to have no window but `longest`.
        '';
        type = lib.types.nullOr lib.types.ints.positive;
        default = null;
      };
      longest = lib.mkOption {
        description = ''
          How long one run may take before systemd stops it as a failed
          unit. It counts the wait for the turn and every pause, which
          systemd can't leave out, so a job naming `runsAtMost` sets this
          well above it and its wait.
        '';
        type = lib.types.str;
        default = "1h";
      };
      memoryHigh = lib.mkOption {
        description = "The memory past which the job is made to give some back, as systemd's MemoryHigh, on the job's own cgroup.";
        type = lib.types.nullOr lib.types.str;
        default = null;
      };
      memoryMax = lib.mkOption {
        description = "The most memory the job may hold, as systemd's MemoryMax, on the job's own cgroup.";
        type = lib.types.nullOr lib.types.str;
        default = null;
      };
      swapMax = lib.mkOption {
        description = "The most swap the job may hold, as systemd's MemorySwapMax, on the job's own cgroup.";
        type = lib.types.nullOr lib.types.str;
        default = null;
      };
      readsAtMost = lib.mkOption {
        description = "How fast the job may read each disk, as systemd's IOReadBandwidthMax, on the job's own cgroup.";
        type = lib.types.attrsOf lib.types.str;
        default = { };
        example = {
          "/dev/nvme0n1" = "50M";
        };
      };
      environment = lib.mkOption {
        description = "Variables the job runs with.";
        type = lib.types.attrsOf lib.types.str;
        default = { };
      };
    };
  };
in
{
  options.jitsusama.night-shift.jobs = lib.mkOption {
    description = "The account's heavy upkeep, each job run once a night while the machine has memory to spare.";
    type = lib.types.attrsOf (lib.types.submodule jobOptions);
    default = { };
  };

  # How a job easing its bars tells Joel is away, systemd's specifiers
  # expanding in the paths.
  options.jitsusama.night-shift.away = {
    mark = lib.mkOption {
      description = "The file there while someone touched a key or the pointer lately, as swayidle-away keeps it.";
      type = specifiedPath;
      default = "%t/someone-at-the-machine";
    };
    seen = lib.mkOption {
      description = "rhythm's minutes at the machine, whose last says when Joel was last there as a job starts.";
      type = specifiedPath;
      default = "%S/rhythm/seen";
    };
    call = lib.mkOption {
      description = ''
        A program and its arguments exiting 0 while a call is on this
        machine and 1 while none is, run at most every 30 seconds once
        nothing else says he's here; any other answer counts as a call.
        Empty asks about no call.
      '';
      type = lib.types.listOf (lib.types.strMatching "[^[:space:]]+");
      default = [
        (lib.getExe (pkgs.callPackage ../calls/package.nix { }))
        "--here"
      ];
    };
    after = lib.mkOption {
      description = "How long with no key, touch or call, in seconds, before Joel counts as away.";
      type = lib.types.ints.positive;
      default = 1800;
    };
  };

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    lib.mkMerge [
      {
        # Every heavy job by day takes its turn through `night-shift turn`,
        # night job or none, so it is on every account's path.
        home.packages = [ package ];
        # An eased bar replaces one of the job's own, so each names a slice
        # the job stops at; night-shift refuses any other as it starts.
        assertions = lib.mapAttrsToList (name: job: {
          assertion = lib.all (slice: job.stops ? ${slice}) (lib.attrNames job.stopsWhileAway);
          message = "night-shift job ${name} eases a slice's bar it has no stop for: give each slice in stopsWhileAway a bar in stops";
        }) jobs;
      }
      (lib.mkIf (jobs != { }) {
        systemd.user.services = lib.mapAttrs' (
          name: job:
          lib.nameValuePair "night-shift-${name}" {
            Unit = {
              Description = job.description;
              # A run can take most of an hour; a switch leaves it going.
              X-RestartIfChanged = false;
            };
            Service = {
              Type = "exec";
              ExecStart = execStart name job;
              # Whatever the account runs comes first: the slice gives it the
              # disk's idle priority, and the job the least of the processor.
              Slice = "background.slice";
              CPUWeight = 1;
              IOWeight = 1;
              IOAccounting = true;
              RuntimeMaxSec = job.longest;
              # A try that waited as long as it was let for the turn didn't
              # fail; the next try comes as the timer says.
              SuccessExitStatus = lib.mkIf (job.waitsAtMost != null) 75;
              # night-shift splits its cgroup, so the job's limits hold the job
              # and never the watcher beside it.
              Delegate = "memory io";
              Environment = environment job;
            };
          }
        ) jobs;

        systemd.user.timers = lib.mapAttrs' (
          name: job:
          lib.nameValuePair "night-shift-${name}" {
            Unit.Description = "Tries ${name} at night";
            Timer = {
              OnCalendar = job.tries;
              AccuracySec = "1min";
              # A night slept through is left, never caught up on by day.
              Persistent = false;
            };
            Install.WantedBy = [ "timers.target" ];
          }
        ) jobs;
      })
    ]
  );
}
