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
# between them on the XPS 14, and 1.63 GB on 29 October, past the 1.5 GiB
# floor it had: 678 MB of it the shell, 126 MB niri. So the floor is 2.5 GiB,
# and inside it the compositor and the shell hold floors of their own,
# 256 MiB and 1 GiB, set in their modules beside their units, so what's
# kept for them is theirs rather than the portals'. They come first there
# too, at ten times the CPU weight of session.slice's other units: they're
# how Joel drives the machine. Neither runs in real time. On 29 October at
# a load of 28, the shell's main thread waited 11 us to run and niri's
# 16 us, while the shell's own work took 43% of a core: real time would run
# that ahead of the compositor and save almost nothing.
#
# The floor is a hard one, MemoryMin: the kernel
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
  # Inside the account, though, the same targets came to hold Joel's own
  # work back. On 10 November, with some ninety agents' sessions paging in
  # beside his programs at the same 1 ms, a 4 KiB read from app.slice waited
  # in the kernel's latency throttle, not on the drive, which answered in
  # 0.2 ms. Over seven interleaved pairs, it took a median 2.5 to 9.8 ms
  # with the account's targets cleared, against 7.9 to 141 ms with them, and
  # a synced save 11 to 47 ms against 60 to 209. So only user.slice keeps
  # one, ranking the account against the system's builds; inside it, the
  # disk serves whoever asks, and the processor's and memory's settings
  # rank Joel's work.
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

  # The programs Joel works in, and agents' sessions beside them, keep their
  # memory while a build fills the rest. With a build taking all but 2 GiB
  # of a 62 GiB laptop, a program in app.slice walking 2 GiB of its own
  # took 45 ms a pass at the median and up to 338 ms, against 24 ms and
  # 26 ms with memory free. A soft floor of 8 GiB, MemoryLow, brought that
  # to 32 ms and 143 ms: the kernel still took it back. A hard one,
  # MemoryMin, to 26 ms and 68 ms, and nothing was ended. The floor is a
  # share of memory, so it scales with the machine.
  # At 13%, 8.1 GiB, it held too little of what Joel works in: on 29 October,
  # with about 17 agents' sessions open, app.slice held 17.6 GiB, 8.4 of it in
  # swap. Under builds it read 1 GiB of that back from swap every two minutes
  # and waited on memory 2.6% of the time, over the 1% past which the machine
  # queue holds builds, so the builds waited too. 20%, 12.5 GiB there, holds
  # what it kept in memory and what it read back. On 3 November, with 38
  # sessions open, they held 12.0 GiB of app.slice's 13.1 and Joel's
  # programs 1.1, Chrome with 1.55 GiB in swap: the sessions filled the
  # floor and the apps were what swapped. So the sessions have a slice and a
  # floor of their own, agents.slice's 15%, 9.3 GiB, and app.slice keeps 10%,
  # 6.2 GiB, for Joel's programs alone. A floor its programs don't fill
  # costs work only what they leave unused.
  # The sessions' floor is a soft one, MemoryLow. From 4 to 6 November the
  # kernel ended queued work more than 40 times with most of swap free,
  # finding nothing it could take: the hard floors kept 20 GiB, 10 of it for
  # the sessions, most of them idle and waiting on their next message. A
  # session that waits can wait on swap; work that's ended starts again. So the kernel keeps the
  # sessions' 15% while it finds anything else, and takes it back before it
  # ends anything.
  # Every slice above claims both kinds of floor, since the kernel protects a
  # slice only as far as each slice above it is protected. Hard, 15% covers
  # app.slice's 6.2 GiB and session.slice's 2.5 on a 62 GiB machine; soft,
  # agents.slice's 15%. What the floors below leave unused the kernel shares
  # among the account's other slices. Work in jobs, background and containers
  # has the rest of memory, and when that runs out, it's what swaps.
  apps = "10%";
  agents = "15%";
  agentsHeld = "512M";

  # Past a ceiling, the sessions reclaim their own memory rather than the
  # machine's. With 83 of them open on 10 November they held 27 GiB, 40
  # more in swap, and their swapping in kept free memory at the minimum:
  # kswapd gave up a third of the time, and every program that needed a
  # page reclaimed one itself, the shell and Joel's apps among them. In
  # three interleaved rounds of 100 s each, a ceiling of 40% against none
  # took kswapd's give-ups from 31% of the time to 1.3%, allocation stalls
  # from 528 a second to 187, app.slice's memory pressure from 0.39% to
  # 0.11% and session.slice's from 0.18% to 0.04%. The sessions pay: they
  # waited on memory 6.4% of the time rather than 3.6%. At 35% they waited
  # 11.9%, and Joel's slices gained nothing more. MemoryHigh slows the
  # sessions past it and never ends one.
  # At 40%, though, the sessions sat at the ceiling on 11 November with
  # 24 GiB free, holding 90 MiB of file cache, so even pi's own code came
  # back from disk: one idle pi waited 52 s on a page read in. Raised to
  # 51%, 32 GiB here, the sessions' disk swap-ins fell from 2,671 a second
  # to 1,650 and their memory waits from 1.1% of the time to 0.3%, while
  # app.slice's went from 0.034% to 0.023% and session.slice's from
  # 0.015% to 0.003%. The hard floors below, not this ceiling, keep Joel's
  # apps' room.
  agentsHigh = "51%";
  account = "15%";

  # The system's own services a call or the desktop waits on keep a soft
  # floor each, about twice what each held: the journal every program
  # writes to, logins and seats, devices and input, the message bus, the
  # names and links of the network, Bluetooth for a headset and rtkit for
  # sound's real time. On 9 November at 13:0x system.slice held 575 MiB
  # with 671 MiB more in swap, the message bus with 3 of its 5 MiB there,
  # so a call's program could wait on swap to reach it. The kernel keeps
  # a floor only as far as every slice above it claims one, and system.slice
  # claimed none, so the oomd's own 64 MiB kept nothing either. The slice
  # claims exactly the sum: a larger one would be shared out by use, and
  # update-to-main's evaluation, which reached 20 GB there that day, would
  # take most of it. Each counts only while its service is on, so a machine
  # without one gets no empty unit.
  systemFloors = lib.filterAttrs (_: floor: floor.on) {
    systemd-journald = {
      mib = 64;
      on = true;
    };
    systemd-logind = {
      mib = 16;
      on = true;
    };
    systemd-udevd = {
      mib = 128;
      on = true;
    };
    dbus-broker = {
      mib = 32;
      on = config.services.dbus.implementation == "broker";
    };
    systemd-oomd = {
      mib = 64;
      on = config.systemd.oomd.enable;
    };
    systemd-resolved = {
      mib = 16;
      on = config.services.resolved.enable;
    };
    systemd-networkd = {
      mib = 16;
      on = config.systemd.network.enable;
    };
    wpa_supplicant = {
      mib = 16;
      on = config.networking.wireless.enable;
    };
    bluetooth = {
      mib = 16;
      on = config.hardware.bluetooth.enable;
    };
    keyd = {
      mib = 64;
      on = config.services.keyd.enable;
    };
    rtkit-daemon = {
      mib = 8;
      on = config.security.rtkit.enable;
    };
  };
  systemFloor = lib.foldl' (sum: floor: sum + floor.mib) 0 (lib.attrValues systemFloors);
in
{
  options.jitsusama.slices.disks = lib.mkOption {
    description = ''
      The disks shared by the slices' weights, by their names under
      /sys/class/block, each with the model io.cost measured on it. Naming
      any drops user.slice's latency target, so the disk behind /home belongs here,
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
    # A unit an agent starts with systemd-run stays in the agent's slice,
    # rather than landing among Joel's apps, as systemd-run.sh says.
    environment.systemPackages = [
      (pkgs.callPackage ./systemd-run.nix { real = "${config.systemd.package}/bin/systemd-run"; })
    ];

    systemd.user.slices.session.sliceConfig = {
      CPUWeight = 1000;
      IOWeight = 1000;
      MemoryMin = "2560M";
    };

    # Ten times the weight of what sits beside it in the account, agents'
    # sessions included. At the same weight his programs waited on the
    # processor all evening while agents worked: on 11 November, switching
    # between 100 and 1000 every five minutes for 40 minutes, app.slice's
    # CPU wait fell from 3.6 ms a second to 1.1 at a load under 20, and
    # from 8.9 to 4.3 above it. A busy app takes only what it uses, so the
    # sessions lose little: his programs used 0.04 of a core of their 1.6.
    systemd.user.slices.app.sliceConfig = {
      CPUWeight = 1000;
      MemoryMin = apps;
    };

    # Agents' sessions, pi in a window of kitty's each, beside the apps
    # rather than among them, at a tenth of their weight. A task in app.slice is preempted
    # whenever a sibling there wakes on its processor: on 3 November the
    # sessions and mycelium woke 2,627 times a second, Joel's programs under
    # 100, and a task spinning 300 ms in app.slice waited a median 27 ms and
    # at worst 57, against 5.6 ms and 16 in an empty slice of the same
    # weight. Raising app.slice's weight to 500 changed nothing then, as it
    # can't reach what competes inside it. Beside it, the sessions meet
    # Joel's programs as one group, which his weight now puts first, and
    # the sorter weighs the window he is at first among them. Whoever starts
    # their terminal names this slice, and kitty puts each window's scope
    # in its own. Its floor is soft but for one part, hard: the floor the
    # kitty drawing every session's window keeps for itself (Quickshell's
    # sessions-kitty), which a child's hard floor has only as far as its
    # slice has one. Soft alone, the kernel took it back 285,154 times in
    # 40 minutes of the account swapping on 2026-11-10, and Joel's typing
    # stalled each time a session opened. Inside the account's 15% it sits
    # beside session.slice's 2.5 GiB and app.slice's 6.2.
    systemd.user.slices.agents.sliceConfig = {
      MemoryLow = agents;
      MemoryMin = agentsHeld;
      MemoryHigh = agentsHigh;
    };

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

    # Work other sessions wait on, beside jobs rather than in it, so it runs
    # ahead of jobs, which is idle on the processor. On 5 November, while the
    # account's slices held disk targets, pushes holding one of a
    # repository's few walk slots sat about an hour in jobs waiting in the
    # kernel's latency throttle, while twelve walks queued behind them
    # waited 80 hours that day. On the processor it still yields to what
    # Joel uses, at a tenth of an app's weight, and past its ceiling it
    # swaps its own memory, as jobs does. Whoever starts such work puts it
    # here, with `systemd-run --user --scope --slice=`.
    systemd.user.slices.shared.sliceConfig = {
      CPUWeight = 10;
      IOWeight = 10;
      MemoryHigh = "15%";
    };

    # Each measured disk is shared by weight from the moment it appears, and
    # stopping its unit hands it back to first come, first served. Boot wants
    # the unit, for a disk the initrd already found: a device plugged before
    # the switch to the real root never changes state again, so its own wants
    # are never pulled in. Boot doesn't wait for the disk, though: without
    # it the unit is skipped, and a disk that comes later wants it itself.
    # Its stop reaches the unit without the unit requiring it, since a
    # requirement would have boot wait out the device's timeout.
    systemd.slices.system.sliceConfig.MemoryLow = "${toString systemFloor}M";

    # Every move of a process between cgroups, which the sorter makes for
    # each piece of work, takes the hierarchy's write lock, and without
    # favordynmods that lock first waits out an RCU grace period, longer the
    # more CPUs are busy, while the process asking waits. On 11 November, in
    # two VMs alike but for it, under a load of forks, a move's median went
    # from 106 µs to 49 and its p99 from 596 to 240, while a fork's stayed
    # at 439 and went from 1,096 to 1,112. A boot takes it from the kernel's
    # command line.
    boot.kernelParams = [ "cgroup_favordynmods=true" ];

    systemd.services = {
      # Every account's manager, user@<uid>.service, beside its slice.
      "user@".serviceConfig = limits // {
        MemoryMin = account;
        MemoryLow = agents;
      };
      # A machine already up takes favordynmods at the switch that brings it,
      # from a remount, which keeps the hierarchy's other flags; it can't be
      # turned off again until the next boot.
      cgroups-favor-moves = {
        description = "Has cgroups favour moving processes over forking them";
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = lib.escapeShellArgs [
            (lib.getExe' pkgs.util-linux "mount")
            "-o"
            "remount,favordynmods"
            "/sys/fs/cgroup"
          ];
        };
      };
      # The sorter asks for a session window's swapped memory to be read
      # back the moment the focus comes to it, rather than a page at a time
      # as Joel types (jobs-sorter/src/swap_in.rs): from disk, 505 MiB came
      # back in 0.56 s that way against 8.3 s, on 11 November. Asking for
      # another process's memory takes CAP_SYS_NICE, which no user service
      # holds, so the unit starts as root, becomes the process's own account
      # with that capability alone, and reads nothing back unless niri says
      # the process's window has the focus. Administrators may start it
      # (swap-in.rules), at most once a window every 30 s.
      "memory-swap-in@" = {
        description = "Reads back the swapped memory of focused window process %i";
        path = [ pkgs.util-linux ];
        serviceConfig = {
          Type = "oneshot";
          ExecStart = "${lib.getExe sorter} swap-in %i";
          CapabilityBoundingSet = [
            "CAP_SETUID"
            "CAP_SETGID"
            "CAP_SETPCAP"
            "CAP_SYS_NICE"
          ];
          NoNewPrivileges = true;
          # Its capability can't turn to taking real time.
          RestrictRealtime = true;
          ProtectSystem = "strict";
          # Root's own stamps of when each window was last read back, out of
          # its account's reach. Every instance shares the one folder, so
          # one instance ending mustn't take it from the others.
          RuntimeDirectory = "memory-swap-in";
          RuntimeDirectoryMode = "0700";
          RuntimeDirectoryPreserve = true;
          PrivateTmp = true;
          PrivateDevices = true;
          # No network but Unix sockets, and those of the host's network
          # namespace: the kitty drawing many windows answers which has
          # the focus only on its abstract socket (src/kitty.rs), which a
          # private network would hide.
          IPAddressDeny = "any";
          RestrictAddressFamilies = "AF_UNIX";
          ProtectKernelTunables = true;
          ProtectKernelModules = true;
          ProtectControlGroups = true;
          LockPersonality = true;
          MemoryDenyWriteExecute = true;
          RestrictNamespaces = true;
          SystemCallFilter = [
            "@system-service"
            "pidfd_open"
            "process_madvise"
          ];
        };
      };
    }
    // lib.mapAttrs (_: floor: { serviceConfig.MemoryLow = "${toString floor.mib}M"; }) systemFloors
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
        ExecStart =
          lib.escapeShellArgs [
            "${sorter}/bin/jobs-sorter"
            "--under"
            "app.slice"
            "--under"
            "agents.slice"
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
            # A window whose every pi runs a turn nobody typed, with no key
            # for a minute and nothing in its editor, as the harness marks
            # them, yields to the apps until a key comes; a session started
            # outside a terminal is weighed the same way.
            "--background"
            "%t/pi/background"
            "--weigh"
            "mycelium-pi-"
            # A pi host's unit holds many sessions behind one node process,
            # each shown by a terminal running only mycelium's attach, so the
            # unit ranks as they would in windows of their own: low while
            # every one is marked, held while Joel is at a window showing
            # one, and keeping an orchestrator's floor for each it holds.
            "--hosts"
            "pi-host-"
            # When the kernel's reclaim thread gives up, agents' work gives a
            # gibibyte back, which brings it back; otherwise every program
            # reclaims for itself until the OOM killer ends one.
            "--revive"
            "jobs.slice"
            # A Nix evaluation or build agents start runs at once while a
            # quarter of memory stays available once it has grown by a
            # sixteenth, counting those let in the last two minutes as
            # growing. Short of that it runs ranked behind all other work,
            # its memory capped to keep a sixteenth free, and only short of
            # that too does it wait, frozen as it starts, five minutes at
            # most. Through a call, which pw-dump hears as a program
            # recording a microphone, three eighths stay, and nothing held
            # or ranked comes into the call.
            "--door"
            "jobs.slice"
            "--calls"
            (lib.getExe' pkgs.pipewire "pw-dump")
            # A window holding a session others report to, as the harness's
            # mesh says, keeps a memory floor and is the last systemd-oomd
            # ends, and every other pi gives way first to the OOM killer.
            "--orchestrators"
            "%h/.local/state/pi/agentic-harness.pi/mesh-workflow/presence"
          ]
          # The session window Joel is at, as niri says, and those he was at
          # in the last ten minutes, three in all, rank as his own programs:
          # an app's weight, a floor inside agents.slice's own and oomd's
          # avoid. On 10 November the pi he typed into, at an agent's rank
          # beside eighty, had 726 MB in swap and his keys felt sluggish.
          + lib.optionalString config.programs.niri.enable (
            " "
            + lib.escapeShellArgs [
              "--niri"
              (lib.getExe config.programs.niri.package)
            ]
          );
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
    security.polkit.extraConfig = lib.concatMapStringsSep "\n" builtins.readFile [
      ./work-slices.rules
      ./swap-in.rules
    ];
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
      SHARED_SLICE = "shared.slice";
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
      MemoryLow = agents;
      IOWeight = 1000;
    }
    // latency;

    # Every account's user-<uid>.slice, which logind makes as it logs in.
    systemd.slices."user-" = {
      overrideStrategy = "asDropin";
      sliceConfig = {
        MemoryMin = account;
        MemoryLow = agents;
      };
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
