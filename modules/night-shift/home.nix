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
# 3%, and never beside another night job. It is stopped if the apps' or the
# desktop's slice passes 1% over ten seconds, agents' sessions' 10%, or the
# machine's 25%. A job's log is its unit's journal; each sample it took is
# in ~/.local/state/night-shift/NAME.samples, and NAME.done is written when
# it succeeds. The job's limits hold only the job: night-shift watches from
# a cgroup beside it, so a job held at its limit never stalls its watcher.
# The unit is delegated memory and io for that.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  jobs = config.jitsusama.night-shift.jobs;

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

  execStart =
    name: job:
    lib.concatMapStringsSep " " execArg (
      [
        (lib.getExe package)
        "run"
        name
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
      ++ [ "--" ]
      ++ job.command
    );

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
        default = [
          "*-*-* 01:30/15"
          "*-*-* 02..04:00/15"
          "*-*-* 05:00"
        ];
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
      restsHours = lib.mkOption {
        description = "How long after a success a try does nothing, in hours.";
        type = lib.types.number;
        default = 12;
      };
      longest = lib.mkOption {
        description = "How long one run may take before systemd stops it.";
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

  config = lib.mkIf (jobs != { } && pkgs.stdenv.hostPlatform.isLinux) {
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
          # night-shift splits its cgroup, so the job's limits hold the job
          # and never the watcher beside it.
          Delegate = "memory io";
          Environment = lib.mapAttrsToList (variable: value: "${variable}=${value}") job.environment;
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
  };
}
