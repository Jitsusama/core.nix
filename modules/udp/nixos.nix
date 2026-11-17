# UDP's receiving side, for a machine whose VPN runs over QUIC.
#
# A socket that never asks for a receive buffer of its own gets
# net.core.rmem_default, 208 KiB, which holds about 90 full datagrams at the
# 2.3 KB the kernel charges each: under 3 ms of a download at 370 Mbit/s. A
# VPN client that reads its QUIC socket in bursts, without raising that
# buffer, drops whatever arrives while it's busy, and every connection inside
# the tunnel slows as though the line had lost it. On one laptop the
# tunnel's socket dropped 14,545 datagrams in 19 minutes against 1.08
# million the tunnel received, 1.3%, and 4.5% in its busiest minutes.
#
# At 2 MiB the buffer holds about 900 datagrams, fewer where a driver charges
# more for each, so a 25 ms pause of the same download waits rather than
# drops. It's no larger because a reader slower than the line keeps a queue
# as long as the buffer, which everything inside the tunnel waits behind: 25
# ms at 370 Mbit/s, longer the slower the line. net.core.rmem_max is set to
# 4 MiB, as the kernel's own default is today and older ones weren't, so a
# program that asks for a buffer can always get more than one that doesn't.
#
# TCP sizes its own buffers from net.ipv4.tcp_rmem, and a program that asks
# gets what it asks for, up to rmem_max, so this reaches only the sockets
# that ask for nothing, such as UDP's, netlink's and raw ones. A buffer is a
# limit, not memory set aside, spent only while what's received waits to be
# read. A socket takes its buffer when it opens, so one already open keeps
# 208 KiB until it's opened again, and taking this out leaves 2 MiB until
# the next boot.
_: {
  boot.kernel.sysctl = {
    "net.core.rmem_default" = 2097152;
    "net.core.rmem_max" = 4194304;
  };
}
