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

  # A core is root's, with an access list that lets the crashed process's
  # account read it, and it holds whatever that process held: a plain pi's
  # keys and tokens, a password manager's vault. (A pi host started under
  # its unit's core limit of 0 keeps none; one started before a switch
  # keeps its old limit until its next start, and the six-hour removal
  # below takes the host's own core, though not its commands'.) Any
  # program running as the same account can abort another and read its
  # core seconds later, so the folder is root's alone and the access lists
  # lead nowhere. coredumpctl's list and each crash's stack trace come from
  # the journal and still reach an account in wheel, though coredumpctl
  # info then fails to reach the core; Joel reads a core through run0.
  # systemd-coredump names the folder as its state directory, which
  # systemd sets back to 0755 each time a crash starts it, so its mode is
  # root's there. NixOS's tmpfiles rules come first, so this one replaces
  # systemd's own line for the folder from boot on, and keeps its two weeks.
  systemd.services."systemd-coredump@".serviceConfig.StateDirectoryMode = "0700";
  systemd.tmpfiles.rules = [ "d /var/lib/systemd/coredump 0700 root root 2w" ];

  # A pi's core holds its keys and tokens: a plain pi names its process
  # pi, and a pi host's node, as any other node program is named, holding
  # whatever it was given. Six hours is long enough to read one for where
  # it faulted, and the stack trace systemd-coredump wrote to the journal
  # outlives it, so the core goes then, rather than staying the two weeks
  # or 4 GB the rest may. Checked each quarter hour, so none stays much
  # past it, and each one removed is named in the service's journal.
  # Root's job, since the cores are.
  systemd.services.coredump-forget-node = {
    description = "Remove pi's and node's cores six hours after they were kept";
    serviceConfig.Type = "oneshot";
    script = ''
      find /var/lib/systemd/coredump -maxdepth 1 -type f \
        \( -name 'core.pi.*' -o -name 'core.node.*' \) -mmin +360 -print -delete
    '';
  };
  systemd.timers.coredump-forget-node = {
    wantedBy = [ "timers.target" ];
    timerConfig.OnCalendar = "*:0/15";
  };
}
