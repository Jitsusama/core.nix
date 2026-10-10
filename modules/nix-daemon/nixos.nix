# The Nix daemon, which runs every build on the machine. Builds yield the CPU
# to whatever Joel is doing, so a build on every core never slows the desktop:
# the scheduler never prefers a batch process over an interactive one.
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

  systemd.services.nix-daemon.serviceConfig = {
    ManagedOOMMemoryPressure = "kill";
    ManagedOOMMemoryPressureLimit = "80%";
  };
}
