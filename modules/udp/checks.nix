# The checks of udp, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  onLinux,
  ...
}:
onLinux {
  # A machine with a screen gives a socket that asks for no buffer of its own
  # 2 MiB to receive into, as the role is where a VPN over QUIC runs.
  a-quiet-socket-receives-into-2-mib =
    holds "a-quiet-socket-receives-into-2-mib"
      "A machine with a screen doesn't give a socket that asks for no receive buffer 2 MiB, so a VPN over QUIC may drop a burst."
      ((graphical.config.boot.kernel.sysctl."net.core.rmem_default" or null) == 2097152);
}
