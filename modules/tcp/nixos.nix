# TCP's sending side, for a machine whose uplink the shaper holds in CAKE.
#
# A program that writes as fast as its socket takes, as an HTTP/2 client or
# an upload does, fills a send buffer the kernel grows to megabytes, and a
# small write it makes next, a request or a keystroke over the same
# connection, waits behind all of it. With no more than 128 KiB unsent, a
# socket tells the program it can write only once that much is left, so the
# rest waits in the program, where a later write can go ahead of it. In two
# machines over a 20 Mbit/s uplink at 40 ms, an urgent record written behind
# an upload was echoed in 83 to 87 ms rather than 209 to 403, with the upload
# still at 19 Mbit/s.
#
# After a second idle a connection starts again from a few packets, so an
# agent's request after a pause takes several round trips to reach the line's
# rate. Kept at full speed, a 1 MB send after 3 s idle took 482 ms rather
# than 560 to 574, against 460 that the line allows. The burst that leaves
# lands in CAKE's own queue on this machine, a queue for each flow, not the
# router's, so nothing else waits behind it.
#
# CAKE signals a queue too long by dropping a packet, unless the flow asked
# for ECN, when it marks one instead and the sender slows with nothing lost.
# The kernel only answers a peer that asks, so a connection this machine
# opens never asked, and a dropped last packet of a short request costs a
# loss probe or a 200 ms timeout. Asking, with the fallback that drops ECN
# for a peer whose path mangles it, CAKE marked where it had dropped, and a
# peer that refuses ECN still connected. On a laptop's Wi-Fi, an hour each,
# per thousand segments, retransmits fell from 3.08 to 1.70, timeouts from
# 0.347 to 0.126 and loss probes from 0.559 to 0.178, and the shaper's CAKE
# went from 645 drops and no marks to 8 drops and 572 marks. That hour
# carried 27% fewer segments, so lighter load may account for some of it.
_: {
  boot.kernel.sysctl = {
    "net.ipv4.tcp_notsent_lowat" = 131072;
    "net.ipv4.tcp_slow_start_after_idle" = 0;
    "net.ipv4.tcp_ecn" = 1;
    "net.ipv4.tcp_ecn_fallback" = 1;
  };
}
