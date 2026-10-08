{ pkgs, ... }:
{
  # settings is deliberately unset. home-manager would render it as a
  # read-only store link, and Claude Code writes that file at runtime
  # (permissions, /model, onboarding, sandbox setup), failing with
  # EACCES. Left unset, Claude Code keeps a normal, writable file.
  programs.claude-code.enable = true;
  home.packages = [
    (pkgs.writeScriptBin "claude-statusline" (builtins.readFile ./claude-statusline.rb))
  ];
  programs.git.ignores = [ "CLAUDE.local.md" ];
}
