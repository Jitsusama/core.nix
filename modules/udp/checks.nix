# The checks of udp, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  onLinux,
  ...
}:
onLinux {
  # A machine with a screen is where a person's VPN runs, so a socket there
  # that asks for no buffer of its own gets room for a burst, and one that
  # asks can get more.
  a-socket-asking-for-nothing-holds-a-burst =
    let
      sysctl = graphical.config.boot.kernel.sysctl;
    in
    holds "a-socket-asking-for-nothing-holds-a-burst"
      "A machine with a screen doesn't give a socket that asks for no receive buffer 2 MiB under a 4 MiB most, so a VPN over QUIC may drop a burst."
      (
        (sysctl."net.core.rmem_default" or null) == 2097152
        && (sysctl."net.core.rmem_max" or null) == 4194304
      );
}
