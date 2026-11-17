# UDP's receiving side, for a machine whose VPN runs over QUIC.
#
# A socket that never asks for a receive buffer of its own gets
# net.core.rmem_default, 208 KiB, which holds about 90 full datagrams: under
# 3 ms of a download at 370 Mbit/s. A VPN client that reads its QUIC socket
# in bursts, without raising that buffer, drops whatever arrives while it's
# busy, and every connection inside the tunnel slows as though the line had
# lost it. On one laptop the tunnel's socket dropped 14,545 datagrams in 19
# minutes against 1.08 million the tunnel received, 1.3%, and 4.5% in its
# busiest minutes.
#
# At 2 MiB the buffer holds about 900 datagrams, a 25 ms pause of the same
# download, so a pause waits rather than drops. It's no larger because a
# reader slower than the line keeps a queue as long as the buffer, which
# everything inside the tunnel waits behind; 2 MiB holds that to about 25 ms.
# It stays under net.core.rmem_max, 4 MiB in the kernel's own default, so a
# program that asks for a buffer can still ask for more.
#
# TCP sizes its own buffers from net.ipv4.tcp_rmem, and a program that asks
# gets what it asks for, so this reaches only the sockets that ask for
# nothing: UDP's, netlink's and raw ones. A buffer is a limit, not memory set
# aside, spent only while what's received waits to be read. A socket takes
# its buffer when it opens, so one already open keeps 208 KiB until it's
# opened again, and taking this out leaves 2 MiB until the next boot.
_: {
  boot.kernel.sysctl."net.core.rmem_default" = 2097152;
}
