# The Nix daemon, which runs every build on the machine. Builds yield the CPU
# and the disk to whatever Joel is doing, so a build on every core never slows
# the desktop: the scheduler never prefers a batch process over an interactive
# one, and idle I/O waits until nothing else wants the disk.
{
  nix.daemonCPUSchedPolicy = "batch";
  nix.daemonIOSchedClass = "idle";
}
