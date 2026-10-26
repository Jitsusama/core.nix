# The Nix daemon, which runs every build on the machine. Builds yield the CPU
# to whatever Joel is doing, so a build on every core never slows the desktop:
# each build runs at SCHED_IDLE, which the scheduler takes off a core the
# moment any normal task there wakes, whatever cgroups either is in, and a
# core running only idle work counts as free when one looks for a core.
# Batch, which they ran at before, only never preempts as it wakes; a batch
# build kept its turn, and niri waited for it. With a sixteen-thread Nix build
# at batch beside the agents' work, niri waited for a core over 3 ms 47 and
# 76 times in 30 s and missed 4 and 7 of 3,600 refreshes; at idle it waited
# 10 and 11 times and missed 4 and 1. The build did 32% less work, giving way
# to the agents' cargo builds as well as to Joel. An idle slice did as well
# for niri but starved VM checks, whose threads vm-first puts back to the
# normal policy below; at SCHED_IDLE they keep build.slice's weight.
#
# A policy only orders processes when they meet on a core, though, and the
# daemon's group, system.slice, weighs the same as Joel's user.slice, so a
# build took half the CPU from his programs whatever its policy. In a slice of its own at
# a tenth of their weight it takes what they leave. On the XPS 14, with a
# kernel build in an app's scope, a sixteen-thread Nix build and a virtual
# machine booting at once, a wakeup in another app waited 280 to 377 us at
# the 99th percentile over two runs, against 802 to 1,534 us in system.slice;
# the kernel build took 202 s, against 233 s; and the Nix build kept 58% of
# the throughput it had. An idle slice did no better for the apps, and since
# it yields to everything in user.slice, a build in a terminal would stop
# Nix's builds altogether. The slice's name has no dash, since systemd reads
# one as nesting: nix-build.slice would sit in a nix.slice at the default
# weight, and its own weight would rank it only against its siblings.
#
# Builds also ask for idle I/O, which a disk scheduler that honours priority
# would serve only when nothing else wants the disk. NVMe drives run without
# one, as the kernel sets them up and as measured fastest on the XPS 14, so
# there it's a statement of intent rather than a limit.
#
# A build that keeps the machine short of memory is stopped by systemd-oomd
# after 30 s, at the same pressure as the user slices, rather than left to the
# kernel's OOM killer, which picks by size and could pick the browser instead.
# oomd stops whichever of the daemon's cgroups reclaims most, which with a
# cgroup for each build is most often one build. vm-first marks the daemon's
# own cgroup for oomd to pass over while a build can go instead, since the
# daemon's going takes every connection with it; when it does go, it starts
# again with the next connection.
#
# A build that brings free space under 24 GiB has the daemon collect garbage
# until 64 GiB is free, so Nix never fills the disk by itself, whatever else
# looks after it. The daemon checks every 5 s while it builds, and collects
# only garbage, nothing a profile or a root still needs. Nix's default of 0
# never collects: the XPS 14 made 238 GiB of garbage in its first three days,
# most of it disk images of up to 9 GiB each. 24 GiB leaves room for two of
# those to finish while the collection runs, and 40 GiB more keeps it from
# running again with the next build.
#
# Each build runs in a cgroup of its own, which Nix makes under the daemon's
# as nix-build-uid-<its user>, having moved the daemon itself into a
# nix-daemon cgroup beside them. So a build's processes end with it, none
# left running, each build's CPU, memory and disk use can be read from its
# cgroup, and the CPU is shared by build rather than by thread, so a build
# on sixteen threads no longer crowds out one on a single thread. They all
# stay in build.slice, under its weight and its limits. Nix 3.21 makes the
# cgroups without handing them any controllers, and systemd handed the
# daemon none for the CPU until asked, so vm-first hands on what systemd
# gives, once the daemon has moved aside, since systemd can't start a new
# daemon in a cgroup that hands controllers on; Nix 2.35 does that itself.
# The daemon stops only its own process when it restarts, so a switch that
# turns this on lets the builds already running finish where they are.
# Before each start, systemd takes the hand-off back so the new daemon can
# start, which would fail, and the restart with it, while a build had handed
# controllers further down itself. None can: Nix gives a build its cgroup
# and the right to move its processes, but not its cgroup.subtree_control,
# so a systemd-nspawn container in a uid-range build makes cgroups with no
# controllers, and the daemon restarts beside it. A Nix that hands that on
# too fails vm-checks-go-first before a switch can find it out. As
# each build ends, vm-first says in its journal what the build wrote and
# read, and for which command and quest, so the builds that wear the disk
# most can be found: journalctl -u vm-first.
#
# A check's virtual machine is a build too, and on a busy machine it shared
# each CPU evenly with the compilers beside it, as batch, which never
# preempts as it wakes, and at idle it would wait behind every program
# and the agents' work besides: its guest's timers fired late and the check's
# timeouts failed, so other sessions held their builds while one ran. So
# vm-first puts every qemu thread back to the normal policy at nice -10
# within a second of it starting, and weighs the build holding it at 1000
# against every other build's 100, as nice -10 is to 0. On one CPU, a
# spinner at nice -10 took 81% of it against one at 0. Everything outside
# the daemon, what Joel uses, keeps its lead.
#
# What needs that is a check with deadlines, not qemu itself: a check run in
# containers or in a nested compositor keeps time the same way. So a build
# running NixOS's test driver, systemd-nspawn, niri or sway is put ahead as
# a VM's is, every thread in it and in its containers' cgroups, and weighed
# at 1000. Only the daemon's builds are looked in, never Joel's own niri.
#
# A check can run its machines as systemd-nspawn containers rather than
# virtual machines, which start in a second or two rather than booting a
# kernel. A container needs a range of user ids of its own, which a build
# asking for the uid-range feature gets once the daemon allocates build users
# itself rather than taking nixbld's.
{ pkgs, ... }:
let
  gib = 1024 * 1024 * 1024;
  vmFirst = pkgs.callPackage ./vm-first/package.nix { };
in
{
  nix.daemonCPUSchedPolicy = "idle";
  nix.daemonIOSchedClass = "idle";

  nix.settings.auto-allocate-uids = true;
  nix.settings.use-cgroups = true;
  nix.settings.experimental-features = [
    "auto-allocate-uids"
    "cgroups"
  ];
  nix.settings.extra-system-features = [ "uid-range" ];

  nix.settings.min-free = 24 * gib;
  nix.settings.max-free = 64 * gib;

  systemd.slices.build.sliceConfig.CPUWeight = 10;

  systemd.services.nix-daemon.serviceConfig = {
    Slice = "build.slice";
    # Nix's own unit says `Delegate=`, which delegates but asks for no
    # controllers, so the daemon got only those systemd already ran for its
    # slice, and no unit there asks for the CPU's. Named here, after it, they
    # are asked for. (DelegateControllers is the property systemd reports,
    # not a key it reads.)
    Delegate = "cpu io memory pids";
    ManagedOOMMemoryPressure = "kill";
    ManagedOOMMemoryPressureLimit = "80%";
    # A VM check's guest syncs the folders it shares with the host, and
    # virtiofsd forwards each as syncfs() of the host's whole filesystem,
    # read-only shares too. On btrfs each one commits a transaction, which
    # every program's saves then wait behind: 133 of 160 commits in ten
    # minutes of VM checks came from there. Answered ENOSYS, the guest's
    # FUSE stops asking, and a check's files are thrown away anyway. A
    # build's `sync -f` fails with "Function not implemented"; sync, fsync
    # and fdatasync still work, and Nix itself calls no syncfs(). A test
    # run straight from its driver runs outside the daemon, so core's VM
    # tests also start virtiofsd through without-syncfs (checks.nix).
    SystemCallFilter = [ "~syncfs:ENOSYS" ];
  };

  systemd.services.vm-first = {
    description = "Puts the VMs of Nix's checks ahead of its other builds";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${vmFirst}/bin/vm-first";
      Restart = "on-failure";
      RestartSec = 5;
      # As root, which owns the daemon's cgroup and its builds' weights. It
      # holds no capability but the one below, and ProtectSystem leaves it
      # nothing to write outside /sys, /proc and /dev.
      # The capability, only to change another user's threads' policy and
      # nice.
      AmbientCapabilities = [ "CAP_SYS_NICE" ];
      CapabilityBoundingSet = [ "CAP_SYS_NICE" ];
      NoNewPrivileges = true;
      PrivateNetwork = true;
      ProtectHome = true;
      ProtectSystem = "strict";
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
    };
  };
}
