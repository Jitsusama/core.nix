# pi-claude, pi on Joel's own Claude plan: claude.sh says how. The provider is
# pi's authors' own, which drives Claude Code as the plan's terms allow
# rather than impersonating it, and is loaded only by this command, so no
# other pi session lists models it can't reach. pi itself is whichever is on
# the PATH, so this follows the pi Joel runs everywhere else.
{ pkgs, claude }:
pkgs.writeShellApplication {
  name = "pi-claude";
  runtimeInputs = [ pkgs.coreutils ];
  runtimeEnv = {
    CLAUDE = pkgs.lib.getExe claude;
    PROVIDER = pkgs.fetchFromGitHub {
      owner = "earendil-works";
      repo = "pi-claude-code";
      rev = "70abb65186592ca788b29b3db22c3160d629c41f";
      hash = "sha256-9BHTz/IRf8jZczVYoXtypepBDmWJ9+5id6hwZUdeSV0=";
    };
  };
  text = builtins.readFile ./claude.sh;
}
