# The rules rules-here's check gives rules.nix in place of tests/rules/checks.nix:
# one that's a script over the tree, waiting on nothing, and one whose check
# builds, which rules-here names as not run.
{ pkgs, self, ... }:
{
  every-assert-says-what-was-wrong = self.lib.everyAssertSaysWhatWasWrong pkgs {
    src = ./.;
    waiting = { };
  };
  comments-name-what-exists = self.lib.commentsNameWhatExists pkgs { src = ./.; };
}
