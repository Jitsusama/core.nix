# The checks of rules-here, which checks.nix gathers with every other
# folder's.
{
  lib,
  pkgs,
  self,
  ...
}:
{
  # rules-here runs a rule that's a script over the tree as its check would,
  # passing a tree that keeps it and failing one with a violation planted,
  # naming the rule and what it found, and names and counts a rule whose
  # check builds as not run (rules-here-test.sh, over stand-in-rules.nix's
  # plan as rules.nix makes it).
  rules-here-fails-a-broken-rule =
    let
      plan = pkgs.writeText "plan" (
        import ./rules.nix {
          root = ../..;
          rules = ./stand-in-rules.nix;
          inherit pkgs self;
        }
      );
    in
    pkgs.runCommandLocal "rules-here-fails-a-broken-rule" { } ''
      ${lib.getExe pkgs.bash} ${./rules-here-test.sh} ${./rules-here.sh} ${plan}
      touch $out
    '';
}
