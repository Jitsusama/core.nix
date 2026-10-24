# The checks of bat, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  workstation,
  ...
}:
{
  # An account's switch builds no bat cache of its own, since it has no
  # themes or syntaxes for one to hold, and clears any left from before.
  bat-builds-no-cache =
    let
      step = (account workstation).home.activation.batCache.data or "";
    in
    holds "bat-builds-no-cache"
      "A switch rebuilds bat's cache, which holds nothing bat hasn't already, or leaves an old one."
      (!lib.hasInfix "cache --build" step && lib.hasInfix "rm -rf" step);
}
