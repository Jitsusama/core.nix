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
}
