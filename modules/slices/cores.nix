# Which cores the work nobody waits on runs on. Under every core's load
# the processor's one power budget held each near 2.3 GHz on 19 November,
# so one of Joel's threads ran no faster on a performance core than on an
# efficient one, and a quarter of the processors' quota on jobs took it
# from about 2140 ms to 1580. Held to the efficient cores, jobs and
# background in each account, and the builds beside them, leave the
# performance cores, and the budget they'd spend, to what he uses, while
# the work keeps every efficient core.
#
# The hardware names its efficient cores, so this is a file of its own,
# which the slices and a machine model's hardware both import.
{ config, lib, ... }:
let
  efficient = config.jitsusama.slices.efficientCores;
  held = {
    AllowedCPUs = efficient;
  };
in
{
  options.jitsusama.slices.efficientCores = lib.mkOption {
    description = ''
      The processor's efficient cores, as the kernel lists them in
      /sys/devices/cpu_atom/cpus, such as "4-15". Naming them holds jobs
      and background in each account, and the builds, to them. An
      account's slices take it at its next login, the builds at the
      switch. A processor without efficient cores names none, and
      nothing is held.
    '';
    default = null;
    example = "4-15";
    type = lib.types.nullOr (lib.types.strMatching "[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*");
  };

  config = lib.mkIf (efficient != null) {
    systemd.user.slices.jobs.sliceConfig = held;
    systemd.user.slices.background.sliceConfig = held;
    systemd.slices.build.sliceConfig = held;

    # systemd's own unit hands each account's manager the cpu, memory and
    # pids controllers, and io reaches it besides. Its slices take a list
    # of cores only with cpuset too, so all five are named.
    systemd.services."user@".serviceConfig.Delegate = "cpu cpuset io memory pids";
  };
}
