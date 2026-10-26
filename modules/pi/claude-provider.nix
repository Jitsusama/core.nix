# The claude-code provider, pi's authors' own, which drives Claude Code as a
# Claude plan's terms allow rather than impersonating it. Its billing guard
# stops pi at the first request Anthropic would charge to usage credits
# rather than the plan's limits, so pi never runs on extra usage. pi-claude
# loads it for one command (claude.nix); a machine on the Claude plan loads
# it for every pi (home.nix).
{ pkgs }:
pkgs.fetchFromGitHub {
  owner = "earendil-works";
  repo = "pi-claude-code";
  rev = "70abb65186592ca788b29b3db22c3160d629c41f";
  hash = "sha256-9BHTz/IRf8jZczVYoXtypepBDmWJ9+5id6hwZUdeSV0=";
}
