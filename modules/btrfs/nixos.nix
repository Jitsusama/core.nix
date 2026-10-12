# Keeps room on every Btrfs filesystem for its own bookkeeping. Btrfs hands
# out space to data and to metadata in chunks, and a chunk given to data stays
# data's after the files in it are deleted. Once data's chunks have taken all
# the unallocated space, metadata can't grow, and a write, or even a delete,
# fails with ENOSPC while df still shows hundreds of gigabytes free. On a disk
# that cargo targets and Nix builds fill and empty every day, that is where a
# filesystem ends up.
#
# Dynamic reclaim has the kernel move what's left in nearly empty chunks
# together and give the emptied chunks back, but only while unallocated space
# is under ten chunks, 10 GiB on any disk over 10 GiB, so it costs nothing until
# it's needed, and more the closer the disk gets to running out. Periodic
# reclaim has the kernel look for those chunks each time a chunk's worth of
# space is freed, rather than waiting for an allocation to fail.
# fs/btrfs/space-info.c gives the reasoning, and both are off by default. The
# system chunks are left alone, as the kernel leaves them: they hold only the
# chunk tree and never crowd anything out.
#
# The files exist once a filesystem is mounted, and tmpfiles runs after every
# filesystem the machine mounts at boot, and again whenever a switch changes
# its rules. One mounted later, such as a stick, keeps the kernel's defaults.
let
  reclaim = kind: setting: "w /sys/fs/btrfs/*/allocation/${kind}/${setting} - - - - 1";
in
{
  systemd.tmpfiles.rules = [
    (reclaim "data" "dynamic_reclaim")
    (reclaim "data" "periodic_reclaim")
    (reclaim "metadata" "dynamic_reclaim")
    (reclaim "metadata" "periodic_reclaim")
  ];
}
