# How the machine sends over TCP. A home connection's upload is often the
# narrowest link a laptop's traffic crosses, and Cubic, the kernel's default,
# sends until that link's buffer overflows, so whatever else crosses it, a
# call's audio and video among them, waits behind a full queue. BBR paces
# what it sends to the rate it measures the link at, whatever the link, so
# the queue stays short wherever the laptop is. On one 9 Mbit/s home upload,
# pinging past it while other sessions sent as they always do, the slowest
# twentieth of replies took 218 and 187 ms with Cubic for every new
# connection and 55 and 107 ms with BBR, the typical reply 21 ms either way.
# The cost: one BBR upload alone moved about a sixth less than Cubic did.
# Calls carry their own congestion control over UDP, so this paces the work
# beside them: pi's requests, git, Nix and the rest.
_: {
  boot.kernelModules = [ "tcp_bbr" ];
  boot.kernel.sysctl."net.ipv4.tcp_congestion_control" = "bbr";
}
