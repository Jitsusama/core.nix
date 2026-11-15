# The checks of net, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # net's tests run as it builds: a real modem's pages and eero's golden
  # record collect into the golden record, the modem first with every
  # channel and its log placed by its own clock, the gateway eero as one
  # device with both its roles and its cable to the modem, a real Sony
  # receiver's answers as eero's device for it with its volume and what
  # it's doing, and a source that doesn't answer said so while the rest
  # are kept.
  net-reads-the-home-network = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in net, its tests included.
  net-passes-clippy = passesClippy "net-passes-clippy" (pkgs.callPackage ./package.nix { });
}
