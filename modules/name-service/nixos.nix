# The name service: nsncd, which answers each program's host lookups on
# NixOS's behalf, asks for every address family on its own. The nscd
# protocol carries no family, so nsncd made one lookup for any family and
# left the program to keep the family it asked for. systemd-resolved answers
# such a lookup of a .local name from whichever of mDNS's IPv4 and IPv6
# scopes replies first, with that family's addresses alone, so a machine
# whose neighbours answer over IPv6 first found no IPv4 address for them:
# ssh -4 and ssh's AddressFamily inet said the name didn't resolve, though
# resolvectl found it. The patch is in getai-asks-each-family.patch.
{
  nixpkgs.overlays = [
    (_: prev: {
      nsncd = prev.nsncd.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ ./getai-asks-each-family.patch ];
      });
    })
  ];
}
