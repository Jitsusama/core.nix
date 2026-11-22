# Keeps agents' work and the builds on the efficient cores, where the
# hardware names them (cores.nix). Under every core's load the
# processor's one power budget held each core near 2.3 GHz on 19
# November, so one of Joel's threads ran no faster on a performance core
# than on an efficient one, and a quarter of the processors' quota on jobs
# took it from about 2140 ms to 1580. Held to the efficient cores, that
# work leaves the performance cores, and the budget they'd spend, to what
# he uses, while it keeps every efficient core; whether that buys his
# thread what the quota did is measured on the laptop once it holds. It
# holds around the clock, so agents lose the performance cores while he's
# away too.
#
# Only what agents start is held: pi's commands, which land in
# jobs.service and are sorted into jobs-work.slice, the units they start
# with systemd-run, which go in jobs-work.slice too (systemd-run.sh), and
# the Nix daemon's builds. Joel's own terminal work in jobs-awaited.slice is what he waits
# on, and timed work's results would change with its cores, so both stay
# free, as background.slice does, since the daemons his launcher and tools
# ask run there. What either asks the Nix daemon to build here is held
# with the builds, though, a VM test built here among them, since the
# daemon's slice can't tell whose build it is; most go to the builders. The call brake's quota is processor time, so it still
# holds a quarter of the processors' worth, within these cores.
{ config, lib, ... }:
let
  efficient = config.jitsusama.slices.efficientCores;
in
{
  config = lib.mkIf (efficient != null) {
    systemd.user.services.jobs.serviceConfig.AllowedCPUs = efficient;
    systemd.user.slices.jobs-work.sliceConfig.AllowedCPUs = efficient;
    systemd.slices.build.sliceConfig.AllowedCPUs = efficient;

    # systemd's own unit hands each account's manager the cpu, memory and
    # pids controllers, and io reaches it besides. Its units take a list
    # of cores only with cpuset too, so all five are named. A lingering
    # account's manager, as Joel's is, keeps running through a logout, so
    # it takes cpuset when a switch re-executes it, as the laptop's did on
    # 19 November, or else at the next boot. With cores held, a process holding
    # capabilities its mover lacks, a setuid program's, can't be moved onto
    # other cores, so it stays where it was, and the sorter says so once
    # for each group while it runs.
    systemd.services."user@".serviceConfig.Delegate = "cpu cpuset io memory pids";
  };
}
