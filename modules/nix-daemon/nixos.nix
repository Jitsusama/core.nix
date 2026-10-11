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
{
  nix.daemonCPUSchedPolicy = "batch";
  nix.daemonIOSchedClass = "idle";

  systemd.slices.build.sliceConfig.CPUWeight = 10;

  systemd.services.nix-daemon.serviceConfig = {
    Slice = "build.slice";
    ManagedOOMMemoryPressure = "kill";
    ManagedOOMMemoryPressureLimit = "80%";
  };
}
