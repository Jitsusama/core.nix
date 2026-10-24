# The checks of tcp, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  onLinux,
  ...
}:
onLinux {
  # A machine with a screen, whose uplink the shaper holds, lets no socket
  # keep more than 128 KiB unsent, and sends at full speed after idle.
  # It asks for ECN, so CAKE marks rather than drops what it opens.
  writes-go-out-sooner =
    let
      sysctl = graphical.config.boot.kernel.sysctl;
    in
    holds "writes-go-out-sooner"
      "A machine with a screen lets a socket's unsent data grow to megabytes, slows a connection after idle, or never asks for ECN."
      (
        (sysctl."net.ipv4.tcp_notsent_lowat" or null) == 131072
        && (sysctl."net.ipv4.tcp_slow_start_after_idle" or null) == 0
        && (sysctl."net.ipv4.tcp_ecn" or null) == 1
        && (sysctl."net.ipv4.tcp_ecn_fallback" or null) == 1
      );
}
