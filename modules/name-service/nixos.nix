# The name service: nsncd, which answers each program's host lookups on
# NixOS's behalf, patched twice.
#
# It asks for every address family on its own. The nscd protocol carries no
# family, so nsncd made one lookup for any family and left the program to
# keep the family it asked for. systemd-resolved answers such a lookup of a
# .local name from whichever of mDNS's IPv4 and IPv6 scopes replies first,
# with that family's addresses alone, so a machine whose neighbours answer
# over IPv6 first found no IPv4 address for them: ssh -4 and ssh's
# AddressFamily inet said the name didn't resolve, though resolvectl found
# it (getai-asks-each-family.patch).
#
# And it answers a .local name in time. glibc's client gives nscd 5 s, then
# stops asking it for any host for its next 100 lookups. The modules in its
# own process reach resolved only as a DNS server, slowly when
# /etc/resolv.conf names resolved's stub and not at all when it names another
# resolver, as a VPN client's does. resolved takes about 7 s to give up on an
# absent name, so one lookup of a neighbour that was off cut a long-lived
# program off from every .local name. nsncd now answers a .local name as not
# found after 4 s (local-names-answer-in-time.patch).
{
  nixpkgs.overlays = [
    (_: prev: {
      nsncd = prev.nsncd.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [
          ./getai-asks-each-family.patch
          ./local-names-answer-in-time.patch
        ];
      });
    })
  ];
}
