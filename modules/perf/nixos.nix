# perf, and the kernel settings that let Joel profile his own programs
# without root, as samply and cargo flamegraph expect.
{ pkgs, ... }:
{
  environment.systemPackages = [ pkgs.perf ];

  boot.kernel.sysctl = {
    # Sample your own processes in the kernel as well as in user space. The
    # kernel's default, 2, stops at user space.
    "kernel.perf_event_paranoid" = 1;

    # The NMI watchdog holds one of the CPU's few performance counters for
    # itself, so turning it off gives that counter back to perf.
    "kernel.nmi_watchdog" = 0;
  };
}
