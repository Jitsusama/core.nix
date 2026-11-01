# The claude-code provider, pi's authors' own, which drives Claude Code as a
# Claude plan's terms allow rather than impersonating it. Its billing guard
# stops pi at the first request Anthropic would charge to usage credits
# rather than the plan's limits, so pi never runs on extra usage. pi-claude
# loads it for one command (claude.nix); a machine on the Claude plan loads
# it for every pi (home.nix).
#
# Patched so a used-up plan limit says so plainly: which limit, when it
# resets, and that nothing went to extra usage, where Claude Code says only
# "You've hit your usage limit". And so a question waits on one start of
# Claude Code rather than several, its login is asked once a minute. And so
# a call to a pi tool that isn't loaded is pi's to answer as not found, as
# on every other provider, rather than ending the turn as "relay failure
# Error" each time the model reaches for it.
{ pkgs }:
pkgs.applyPatches {
  name = "pi-claude-code";
  src = pkgs.fetchFromGitHub {
    owner = "earendil-works";
    repo = "pi-claude-code";
    rev = "70abb65186592ca788b29b3db22c3160d629c41f";
    hash = "sha256-9BHTz/IRf8jZczVYoXtypepBDmWJ9+5id6hwZUdeSV0=";
  };
  patches = [
    ./claude-plan-limit.patch
    ./claude-login-once.patch
    ./claude-unloaded-tool.patch
  ];
}
