# The service manager itself. systemd reads at most five changes to the mount
# table a second, and starts no mount while it's over that, until the second
# is up. The kernel's own filesystems use the five as the machine starts, so
# /boot, /home and the swap waited a second for them, and the mounts after
# those waited another, and the greeter came two seconds late. Fifty a second
# leaves the dozen or so changes a boot makes well inside the limit, and still
# caps how often a storm of mounts has systemd read the table again.
{
  systemd.managerEnvironment.SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST = "50";
}
