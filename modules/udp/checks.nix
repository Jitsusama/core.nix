# The checks of udp, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  onLinux,
  ...
}:
onLinux {
  # A machine with a screen gives a UDP socket that asks for no buffer of its
  # own 4 MiB to receive into, so a VPN's burst waits rather than drops.
  a-burst-waits-rather-than-drops =
    holds "a-burst-waits-rather-than-drops"
      "A machine with a screen gives a socket that asks for no receive buffer the kernel's 208 KiB, so a VPN over QUIC drops a burst."
      ((graphical.config.boot.kernel.sysctl."net.core.rmem_default" or null) == 4194304);
}
