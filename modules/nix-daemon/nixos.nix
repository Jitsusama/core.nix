# The Nix daemon, which runs every build on the machine. Builds yield the CPU
# to whatever Joel is doing, so a build on every core never slows the desktop:
# the scheduler never prefers a batch process over an interactive one.
#
# Batch only orders processes that share a group, though, and the daemon's
# group, system.slice, weighs the same as Joel's user.slice, so a build took
# half the CPU from his programs whatever its policy. In a slice of its own at
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
# oomd stops the daemon's whole group, every build with it; the daemon starts
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
# A check's virtual machine is a build too, and on a busy machine it shared
# each CPU evenly with the compilers beside it, as batch, which never
# preempts as it wakes: its guest's timers fired late and the check's
# timeouts failed, so other sessions held their builds while one ran. Inside
# the daemon's one cgroup the CPU goes by nice, so vm-first puts every qemu
# thread there back to the normal policy at nice -10 within a second of it
# starting. On one CPU, a spinner at nice -10 took 81% of it against one at
# 0. Everything outside the daemon, what Joel uses, keeps its lead. Nix's own
# use-cgroups would give each build a cgroup, but never hands them the CPU
# controller, and turning it on restarts the daemon under every build.
#
# A check can run its machines as systemd-nspawn containers rather than
# virtual machines, which start in a second or two rather than booting a
# kernel. A container needs a range of user ids of its own, which a build
# asking for the uid-range feature gets once the daemon allocates build users
# itself rather than taking nixbld's. Only those builds get a cgroup of their
# own, which Nix makes under the daemon's, so they stay in build.slice and
# under its limits; every other build runs as before. vm-first looks only in
# the daemon's own cgroup, where every check's virtual machine still runs,
# since none asks for a range of ids; one beside a container would be missed.
{ pkgs, ... }:
let
  gib = 1024 * 1024 * 1024;
  vmFirst = pkgs.callPackage ./vm-first/package.nix { };
in
{
  nix.daemonCPUSchedPolicy = "batch";
  nix.daemonIOSchedClass = "idle";

  nix.settings.auto-allocate-uids = true;
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
    ManagedOOMMemoryPressure = "kill";
    ManagedOOMMemoryPressureLimit = "80%";
  };

  systemd.services.vm-first = {
    description = "Puts the VMs of Nix's checks ahead of its other builds";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${vmFirst}/bin/vm-first";
      Restart = "on-failure";
      RestartSec = 5;
      DynamicUser = true;
      # Only to change another user's threads' policy and nice.
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
