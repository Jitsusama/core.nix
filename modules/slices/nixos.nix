# Who gets the CPU and memory first when everything wants them. systemd runs
# each account's compositor, sound, portals and message bus in its
# session.slice, and every program Joel starts, a terminal and the build in it
# included, in app.slice, a scope each. A build on every core then shares the
# CPU with Chrome or Slack scope by scope, and session.slice's tenfold weight
# gets the compositor and sound onto a CPU first: under sixteen spinning
# threads, a wakeup in session.slice waited at most 751 us at this weight
# against 1,962 us at the default.
#
# What Joel works in comes before the work it starts. The shell, pi, an
# editor and a terminal's copy all sit in the window's own scope, kitty's
# kitty-<pid>-<n>.scope, at an app's weight, so they answer as quickly as
# Chrome does. The work they start goes to jobs.slice, which is idle: it
# gets every CPU nobody else wants, so a build loses nothing while Joel
# reads, and gives way the moment anything he uses wakes. pi puts each
# process it starts there itself. A terminal's work gets there by how it
# behaves: jobs-sorter moves a process group out of a window's scope once
# it has used over half a core for two seconds running, never the shell
# and never pi, so a pager, an editor or ssh stays where it is. With
# sixteen spinners in an app's scope, a wakeup in a window's idle scope
# had waited up to 17.6 ms, a 10 ms stream of sound had broken up 6 times
# in 20 s, and a paste from a terminal's copy had taken up to 20 ms; with
# the spinners idle, an app's wakeup waited at most 1.3 ms, and the sound
# never broke. The cost: work that must wait for a busy app gets nothing
# until it rests, and for its first two seconds a build in a terminal
# shares the CPU as an app would.
#
# session.slice holds the compositor and the shell, which took 0.98 GiB
# between them on the XPS 14, so a 1.5 GiB floor keeps it in memory however
# hard a build pushes. The floor is a hard one, MemoryMin: the kernel
# takes a soft one, MemoryLow, back whenever it finds nothing else to take, and
# when an evaluation filled a 61 GiB laptop it did so 215,001 times, leaving
# two thirds of niri in swap. niri then read keys up to 281 ms late, and past
# the 200 ms repeat delay a program takes a key that was let go as held. In a
# 4 GiB virtual machine with zram and a swapfile behind it, a program reading
# keys in session.slice, with 200 MiB it touches a little of on each one, read
# them at most 728 ms late under a 5 GB hog and 5.3 s under a 6 GB one with
# the soft floor, and at most 7.3 ms under either with the hard one. The
# kernel protects a slice only as far as each slice above it is protected, so
# every one on the way down claims enough for what the floors below it hold,
# app.slice's as well (see apps below).
#
# A call's sound must never wait for work at all, and only real time does
# that: an RT thread runs ahead of every cgroup's weight. Chrome and every
# Electron app ask for SCHED_RR 8 on their audio threads, and fall back to
# nice -10, but only when the account may: with no real-time or nice limit,
# both fail and a call's audio runs as any build does. So the account may go
# to real-time priority 10 and nice -10. Ten stays under PipeWire's 11, the
# least it takes from the limit before asking rtkit, so its own threads keep
# rtkit's 20, and keyd its 49. A real-time thread that runs 200 ms without
# resting is stopped, so one that spins can't take the machine.
#
# Agents' work runs without real time. What pi starts inherits the account's
# limit, and a headless compositor an agent starts for a test takes SCHED_RR
# with it, ahead of everything Joel uses. So jobs-sorter puts every
# real-time thread under jobs.service, jobs-work.slice and jobs-timed.slice
# back on the ordinary policy, and drops its process's real-time limit to
# none, so what it starts afterwards can't take it either. Joel's own
# terminal work in jobs-awaited.slice keeps it: a player he runs there is
# something he listens to.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  sorter = pkgs.callPackage ./jobs-sorter/package.nix { };
  limits = {
    LimitRTPRIO = "10";
    LimitNICE = "-10";
    LimitRTTIME = "200ms:400ms";
  };

  # A build's writes mustn't hold up a save. The disk caches what it's given,
  # so a synced write, as an editor's save or a browser's database makes,
  # waits for everything ahead of it. Beside 2.1 to 2.6 GiB/s of writes from
  # jobs, a synced 4 KiB write from an app took 9 to 18 ms at the median and
  # up to 282 ms, against 1.9 ms with the disk quiet. A latency target says
  # how long an app's disk work may take: when a tenth of it takes longer, the
  # kernel lets less of jobs' work onto the disk, and lets it back as the
  # app's work meets the target again. Slices with no target give way, jobs,
  # background and the account's containers among them; app.slice and
  # session.slice, at the same target, never hold each other back. At 1 ms the
  # same write took 3 ms at the median and at most 27 ms. The build pays:
  # about half its writing speed while an app saves ten times a second, and
  # about a tenth while the apps Joel has open only idle. The disk is
  # whichever holds /home: systemd follows it through dm-crypt to the drive.
  #
  # A target holds back every slice beside the one it's on, whoever waits.
  # user.slice's holds build.slice back for everything the account does on
  # the disk, jobs' own writes included: with no app using the disk, Nix's
  # builds waited on it 88% of the time while it was under half busy. So a
  # disk the machine has measured can be shared by weight instead, through
  # the kernel's io.cost: each slice using it gets it in proportion to its
  # weight, and what an idle slice leaves goes to the rest. io.cost needs a
  # model of what the disk can do, and the one it guesses for the XPS 14's
  # was 489 MB/s and 8,500 operations a second, far from what the disk
  # measured: 5.8 GB/s of reads, but 403 MB/s of writes once its cache was
  # full. A machine names its measured disks, the one behind /home among
  # them, in jitsusama.slices.disks, and then sets no targets. The
  # weights follow the CPU's: the compositor's ten times an app's, the work
  # Joel starts a hundredth of one, timed and awaited work ahead of agents'
  # within it, and the account ten times the system's other slices. Without
  # io.cost they do nothing, so a machine that names no disk keeps targets.
  #
  # Name a disk only once io.cost has been shown to beat the targets on it,
  # with the same synced write beside the same writers. On the XPS 14's,
  # which has no DRAM of its own, it didn't. Beside two buffered writers in
  # jobs, an app's worst synced save in four runs took 55 to 170 ms under the
  # targets, and 76 to 492 ms under every io.cost setting tried: its
  # automatic one, a 5 or 10 ms latency goal, and a goal held to the model.
  # io.cost let the writers run at two to two and a half times the model's
  # write rate. The disk's cache taking them quickly, with a sync then
  # waiting behind them, is the suspected cause, not yet measured. So that
  # disk keeps its targets.
  disks = config.jitsusama.slices.disks;
  latency = lib.optionalAttrs (disks == { }) { IODeviceLatencyTargetSec = "/home 1ms"; };
  settings = attrs: lib.concatStringsSep " " (lib.mapAttrsToList (k: v: "${k}=${toString v}") attrs);
  diskOrder = pkgs.writeShellApplication {
    name = "disk-order";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.findutils
      pkgs.gnugrep
    ];
    text = builtins.readFile ./disk-order.sh;
  };

  # The programs Joel works in keep their memory while a build fills the
  # rest. With a build taking all but 2 GiB of a 62 GiB laptop, a program in
  # app.slice walking 2 GiB of its own took 45 ms a pass at the median and up
  # to 338 ms, against 24 ms and 26 ms with memory free. A soft floor of
  # 8 GiB, MemoryLow, brought that to 32 ms and 143 ms: the kernel still took
  # it back. A hard one, MemoryMin, to 26 ms and 68 ms, and nothing was
  # ended. The floor is a share of memory, so it scales with the machine.
  # At 13%, 8.1 GiB, it held too little of what Joel works in: on 29 October,
  # with about 17 agents' sessions open, app.slice held 17.6 GiB, 8.4 of it in
  # swap. Under builds it read 1 GiB of that back from swap every two minutes
  # and waited on memory 2.6% of the time, over the 1% past which the machine
  # queue holds builds, so the builds waited too. 20%, 12.5 GiB there, holds
  # what it kept in memory and what it read back. A floor its programs
  # don't fill costs work only what they leave unused. Every slice above
  # app.slice claims a little more, to cover session.slice's 1.5 GiB beside
  # it. What the floors below leave unused the kernel shares among the
  # account's other slices. Work in jobs, background and containers has the
  # rest of memory, and when that runs out, it's what swaps.
  apps = "20%";
  account = "23%";
in
{
  options.jitsusama.slices.disks = lib.mkOption {
    description = ''
      The disks shared by the slices' weights, by their names under
      /sys/class/block, each with the model io.cost measured on it. Naming
      any drops the latency targets, so the disk behind /home belongs here,
      and only once io.cost has measured better than the targets on it.
      The name is the kernel's, which follows the order disks are found in:
      on a machine with two of a kind, the model can land on the other one.
    '';
    default = { };
    example = {
      nvme0n1.model = {
        rbps = 3800000000;
        rseqiops = 340000;
        rrandiops = 340000;
        wbps = 3000000000;
        wseqiops = 300000;
        wrandiops = 300000;
      };
    };
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          model = lib.genAttrs [ "rbps" "rseqiops" "rrandiops" "wbps" "wseqiops" "wrandiops" ] (
            name:
            lib.mkOption {
              type = lib.types.ints.positive;
              description = "io.cost.model's ${name}, as measured on the disk.";
            }
          );
          qos = lib.mkOption {
            type = lib.types.attrsOf (lib.types.either lib.types.ints.unsigned lib.types.str);
            default = { };
            example = {
              rpct = 95;
              rlat = 5000;
              wpct = 95;
              wlat = 5000;
            };
            description = "io.cost.qos's settings beside enable, the kernel's own where unset.";
          };
        };
      }
    );
  };

  config = {
    systemd.user.slices.session.sliceConfig = {
      CPUWeight = 1000;
      IOWeight = 1000;
      MemoryMin = "1536M";
    }
    // latency;

    systemd.user.slices.app.sliceConfig = {
      MemoryMin = apps;
    }
    // latency;

    # Beside app.slice and session.slice, so it yields to both, and held to
    # half of memory. The floors keep only what apps already hold: work in
    # jobs still took the rest, pushing out to swap what an app reached for
    # next. Past the ceiling, jobs swaps its own memory instead. In 238
    # ten-second windows without it, apps waited on memory 0.67% of the time,
    # over 1% in 28 windows. In 298 with it, while three evaluations held jobs
    # at the ceiling with up to 44 GiB swapped, they waited 0.024%, over 1% in
    # one. Jobs pays: it waited 12.7% of the time rather than 8.3%, its worst
    # minute at 55%, under the 80% for 30 s at which oomd ends it. MemoryHigh
    # slows work past the ceiling and never ends it.
    systemd.user.slices.jobs.sliceConfig = {
      CPUWeight = "idle";
      MemoryHigh = "50%";
      IOWeight = 1;
    };
    systemd.user.slices.background.sliceConfig.IOWeight = 1;

    # Each measured disk is shared by weight from the moment it appears, and
    # stopping its unit hands it back to first come, first served. Boot wants
    # the unit, for a disk the initrd already found: a device plugged before
    # the switch to the real root never changes state again, so its own wants
    # are never pulled in. Boot doesn't wait for the disk, though: without
    # it the unit is skipped, and a disk that comes later wants it itself.
    # Its stop reaches the unit without the unit requiring it, since a
    # requirement would have boot wait out the device's timeout.
    systemd.services = {
      # Every account's manager, user@<uid>.service, beside its slice.
      "user@".serviceConfig = limits // {
        MemoryMin = account;
      };
    }
    // lib.mapAttrs' (
      disk: measured:
      lib.nameValuePair "disk-order-${disk}" {
        description = "Shares ${disk} by the slices' weights";
        wantedBy = [
          "multi-user.target"
          "dev-${disk}.device"
        ];
        after = [ "dev-${disk}.device" ];
        unitConfig = {
          ConditionPathExists = "/sys/class/block/${disk}";
          StopPropagatedFrom = "dev-${disk}.device";
        };
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = lib.escapeShellArgs [
            (lib.getExe diskOrder)
            "on"
            disk
            (settings measured.model)
            (settings measured.qos)
          ];
          ExecStop = lib.escapeShellArgs [
            (lib.getExe diskOrder)
            "off"
            disk
          ];
        };
      }
    ) disks;

    # The cgroup work is moved into. A cgroup holds only while something is
    # in it, so this one holds a process of its own, and its delegation says
    # the processes others write into it are theirs, not the unit's.
    systemd.user.services.jobs = {
      description = "The place work Joel starts runs, behind what he uses";
      wantedBy = [ "default.target" ];
      serviceConfig = {
        Slice = "jobs.slice";
        Delegate = true;
        ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
        # systemd-oomd ends the whole cgroup when work in it runs memory out,
        # holder and all, and pi's writes would find nowhere to go until the
        # next login. Coming back at once restarts the sorter with it.
        Restart = "always";
        RestartSec = 1;
      };
    };

    # Work whose result depends on its timing, as a VM test's timeouts and a
    # benchmark's numbers do, takes jobs' share ahead of bulk work and still
    # gives way to what Joel uses. So it runs inside jobs, idle as that is,
    # rather than beside the apps: there, a VM's or a container's threads at
    # any weight but idle made apps wait, and call audio break up (see
    # machines below). On one CPU shared with a spinner in jobs, a spinner here
    # took 92 to 94% of it in three runs, against an even split in jobs. pi's
    # queue moves timed work here, from the waiting shell; TIMED_CGROUP names
    # the place, held as jobs is.
    systemd.user.slices.jobs-timed.sliceConfig = {
      CPUWeight = 10000;
      IOWeight = 10000;
    };

    systemd.user.services.timed = {
      description = "The place timed work runs, ahead of other work";
      wantedBy = [ "default.target" ];
      serviceConfig = {
        Slice = "jobs-timed.slice";
        Delegate = true;
        ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
        Restart = "always";
        RestartSec = 1;
      };
    };

    # Work Joel starts in a terminal is work he's waiting on, so it goes ahead
    # of what agents start, and still gives way to what he uses. pi puts its
    # own work in jobs.service; the sorter moves a terminal's busy work here
    # instead, inside jobs, idle as that is. On one CPU shared with a spinner
    # in jobs, a spinner here took 91% of it, as its weight says. Timed work
    # stays ahead of both, since its results fail when it waits.
    systemd.user.slices.jobs-awaited.sliceConfig = {
      CPUWeight = 1000;
      IOWeight = 1000;
    };

    systemd.user.services.awaited = {
      description = "The place Joel's own work runs, ahead of agents' work";
      wantedBy = [ "default.target" ];
      serviceConfig = {
        Slice = "jobs-awaited.slice";
        Delegate = true;
        ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
        Restart = "always";
        RestartSec = 1;
      };
    };

    # Each command written into a holder's cgroup, pi's into jobs.service, a
    # terminal's into awaited.service and the queue's timed work into
    # timed.service, the sorter moves on into a scope of its own in the same
    # slice. systemd-oomd ends the leaf cgroup holding the most swap when
    # swap runs out (modules/memory), and one cgroup for every session's
    # commands was that leaf: at 03:5x one night, 110 processes and 21 GiB
    # of swap, so all their builds would have ended for the one that filled
    # it. The holders stay, as somewhere the next write lands. A scope takes
    # the weight of the slice it's in, so pi's scopes share jobs-work.slice,
    # at jobs.service's weight, rather than each taking that weight beside
    # jobs-awaited: ten agents' commands would then weigh as much as Joel's.
    systemd.user.slices.jobs-work.sliceConfig = {
      CPUWeight = 100;
      IOWeight = 100;
    };

    systemd.user.services.jobs-sorter = {
      description = "Sorts work into jobs, each piece in a scope of its own";
      wantedBy = [ "default.target" ];
      requires = [ "awaited.service" ];
      after = [ "awaited.service" ];
      # busctl starts each scope, and systemctl names each holder.
      path = [ config.systemd.package ];
      serviceConfig = {
        Slice = "session.slice";
        ExecStart = lib.escapeShellArgs [
          "${sorter}/bin/jobs-sorter"
          "--home"
          "kitty-"
          "--keep"
          "pi"
          "--jobs"
          "jobs.slice/jobs-awaited.slice/awaited.service"
          "--split"
          "jobs.slice/jobs.service=jobs-work.slice"
          "--split"
          "jobs.slice/jobs-awaited.slice/awaited.service"
          "--split"
          "jobs.slice/jobs-timed.slice/timed.service"
          "--no-real-time"
          "jobs.slice/jobs.service"
          "--no-real-time"
          "jobs.slice/jobs-work.slice"
          "--no-real-time"
          "jobs.slice/jobs-timed.slice"
          # An agent's command for a quest goes in that quest's slice under
          # jobs-work.slice, at the weight pi's sessions give the quest by
          # its priority, so a quest Joel drives gets more of what agents'
          # work gets, and never more than that. The disk's share acts only
          # under io.cost or BFQ, not on io.latency targets alone.
          "--by-quest"
          "jobs-work.slice"
          "--quests"
          "%t/jobs/quests.json"
        ];
        Restart = "on-failure";
        RestartSec = 5;
      };
    };

    # Machines are work too, however long they run, so a virtual machine or a
    # container yields to what Joel uses as a build does. libvirt's system
    # machines and systemd-nspawn's run in machine.slice, beside build.slice,
    # and take its weight. Inside the account, a weight buys nothing: with a
    # container spinning sixteen threads in the account's own user.slice, an
    # app's wakeup waited up to 15 ms and a 10 ms stream of sound broke up 46
    # times in 20 s at weight 10, and 4.9 ms and 58 times at 100. Idle, as
    # jobs is, the wakeup waited at most 2.0 ms and the sound never broke. So
    # there machines are idle: rootless Podman's containers in user.slice,
    # and libvirt's session machines in the account's machine.slice
    # (modules/libvirt). A guest then waits only while what Joel uses takes
    # every core, and catches up after.
    systemd.slices.machine.sliceConfig.CPUWeight = 10;

    # The machine's administrators, agents included since they work as
    # Joel's account, may change how build.slice and machine.slice share the
    # machine without a password (work-slices.rules), so a runtime A/B of
    # their weight or cores waits for no one: on 25 October, two of them
    # each waited for Joel to type a line. The account's own slices are its
    # user manager's, which never asks. It grants little: the most a
    # property there can do is slow or starve the system's builds and
    # machines, never Joel's own programs. polkit can't tell a runtime
    # change from a lasting one, so a change made without --runtime stays
    # in /etc/systemd/system.control until it's reverted with
    # `systemctl revert build.slice`; this configuration still sets the
    # slices as they boot.
    security.polkit.enable = true;
    security.polkit.extraConfig = builtins.readFile ./work-slices.rules;
    systemd.user.slices.user.sliceConfig = {
      CPUWeight = "idle";
      IOWeight = 1;
    };
    systemd.user.slices.machine.sliceConfig = {
      CPUWeight = "idle";
      IOWeight = 1;
    };

    # Where pi, and anything else that wants to, finds jobs: a path under the
    # account's own user@<uid>.service.
    environment.sessionVariables = {
      JOBS_CGROUP = "jobs.slice/jobs.service";
      TIMED_CGROUP = "jobs.slice/jobs-timed.slice/timed.service";
    };

    # The Nix daemon's builds run outside the account, in build.slice beside
    # user.slice, where the account's own targets don't reach. Beside 1.5 GiB/s
    # of writes from there, an app's synced 4 KiB write took 26 ms at the
    # median and up to 2 s. With user.slice at the same target it took 3.9 ms
    # and at most 205 ms, and the build wrote 974 MiB/s while the app saved
    # ten times a second. Its siblings, build.slice, machine.slice and
    # system.slice, give way: to its target, or on a measured disk to its
    # weight, ten times theirs.
    systemd.slices.user.sliceConfig = {
      MemoryMin = account;
      IOWeight = 1000;
    }
    // latency;

    # Every account's user-<uid>.slice, which logind makes as it logs in.
    systemd.slices."user-" = {
      overrideStrategy = "asDropin";
      sliceConfig.MemoryMin = account;
    };

    # What the account's own units start with, said outright rather than
    # left to whatever the manager inherited.
    systemd.user.settings.Manager = {
      DefaultLimitRTPRIO = limits.LimitRTPRIO;
      DefaultLimitNICE = limits.LimitNICE;
      DefaultLimitRTTIME = limits.LimitRTTIME;
    };
  };
}
