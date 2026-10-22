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
# session.slice peaks at about 400 MiB, so a 1 GiB floor keeps it in memory
# however hard a build pushes. The floor is a hard one, MemoryMin: the kernel
# takes a soft one, MemoryLow, back whenever it finds nothing else to take, and
# when an evaluation filled a 61 GiB laptop it did so 215,001 times, leaving
# two thirds of niri in swap. niri then read keys up to 281 ms late, and past
# the 200 ms repeat delay a program takes a key that was let go as held. In a
# 4 GiB virtual machine with zram and a swapfile behind it, a program reading
# keys in session.slice, with 200 MiB it touches a little of on each one, read
# them at most 728 ms late under a 5 GB hog and 5.3 s under a 6 GB one with
# the soft floor, and at most 7.3 ms under either with the hard one. The
# kernel protects a slice only as far as each slice above it is protected, so
# every one on the way down claims the same floor, and none of it goes
# anywhere else.
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
{ pkgs, ... }:
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
  disk = "/home 1ms";
in
{
  systemd.user.slices.session.sliceConfig = {
    CPUWeight = 1000;
    MemoryMin = "1G";
    IODeviceLatencyTargetSec = disk;
  };

  systemd.user.slices.app.sliceConfig.IODeviceLatencyTargetSec = disk;

  # Beside app.slice and session.slice, so it yields to both.
  systemd.user.slices.jobs.sliceConfig.CPUWeight = "idle";

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

  systemd.user.services.jobs-sorter = {
    description = "Moves a terminal's busy work into jobs";
    wantedBy = [ "default.target" ];
    requires = [ "jobs.service" ];
    after = [ "jobs.service" ];
    serviceConfig = {
      Slice = "session.slice";
      ExecStart = "${sorter}/bin/jobs-sorter --home kitty- --keep pi";
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
  systemd.user.slices.user.sliceConfig.CPUWeight = "idle";
  systemd.user.slices.machine.sliceConfig.CPUWeight = "idle";

  # Where pi, and anything else that wants to, finds jobs: a path under the
  # account's own user@<uid>.service.
  environment.sessionVariables.JOBS_CGROUP = "jobs.slice/jobs.service";

  systemd.slices.user.sliceConfig.MemoryMin = "1G";

  # Every account's user-<uid>.slice, which logind makes as it logs in.
  systemd.slices."user-" = {
    overrideStrategy = "asDropin";
    sliceConfig.MemoryMin = "1G";
  };

  systemd.services."user@".serviceConfig = limits // {
    MemoryMin = "1G";
  };

  # What the account's own units start with, said outright rather than
  # left to whatever the manager inherited.
  systemd.user.settings.Manager = {
    DefaultLimitRTPRIO = limits.LimitRTPRIO;
    DefaultLimitNICE = limits.LimitNICE;
    DefaultLimitRTTIME = limits.LimitRTTIME;
  };
}
