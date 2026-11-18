# systemd-coredump keeps the core of every program that crashes, for
# coredumpctl to debug, and by default lets them take a tenth of the
# filesystem: 95 GiB on a 953 GiB disk. They pile up quickly on a desktop
# that crashes now and then: two days on the XPS 14 left 19 cores in 3.6 GiB,
# one of them a 1 GiB Nix evaluator. Four gigabytes keeps the last few days'
# worth, which is as far back as anyone debugs a crash, and systemd removes
# the oldest first to stay under it. KeepFree stays at its default of 15% of
# the filesystem, so a core is never what fills the disk.
{
  systemd.coredump.settings.Coredump.MaxUse = "4G";

  # A node's core holds what its process held: a pi host's holds every
  # hosted conversation's keys and tokens, and a plain pi's its own. Six
  # hours is long enough to read one for where it faulted, and the stack
  # trace systemd-coredump wrote to the journal outlives it, so the core goes
  # then, rather than staying the two weeks or 4 GB the rest may. Checked each
  # quarter hour, so none stays much past it. Root's job, since the cores are.
  systemd.services.coredump-forget-node = {
    description = "Remove node's cores six hours after they were kept";
    serviceConfig.Type = "oneshot";
    script = ''
      find /var/lib/systemd/coredump -maxdepth 1 -type f -name 'core.node.*' -mmin +360 -delete
    '';
  };
  systemd.timers.coredump-forget-node = {
    wantedBy = [ "timers.target" ];
    timerConfig.OnCalendar = "*:0/15";
  };
}
