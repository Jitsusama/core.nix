# UDP's receiving side, for a machine whose VPN runs over QUIC.
#
# A socket that never asks for a receive buffer of its own gets
# net.core.rmem_default, 208 KiB, which holds about 90 full datagrams: a few
# milliseconds of a download at 370 Mbit/s. A VPN client that reads its
# QUIC socket in bursts, without raising that buffer, drops whatever arrives
# while it's busy, and every connection inside the tunnel slows as though the
# line had lost it. On one laptop the tunnel's socket dropped 14,545
# datagrams in 19 minutes against 1.08 million the tunnel received, 1.3%, and
# 4.5% in its busiest minutes. At 4 MiB the buffer holds about 60 ms of the
# same download, so a busy moment waits rather than drops.
#
# TCP sizes its own buffers from net.ipv4.tcp_rmem, and a program asking for
# a buffer gets what it asks, so this reaches only the UDP and local sockets
# that ask for nothing. A buffer is a limit, not memory set aside: it's spent
# only while datagrams wait to be read.
_: {
  boot.kernel.sysctl."net.core.rmem_default" = 4194304;
}
