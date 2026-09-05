{ pkgs, ... }:
{
  # `settings` is intentionally left unset: home-manager renders it as a
  # read-only store symlink, which blocks every runtime write Claude Code
  # makes to this file (permissions, /model, onboarding, sandbox setup),
  # failing with EACCES. Leaving it unset gives Claude Code a normal,
  # writable settings.json.
  programs.claude-code.enable = true;

  home.packages = with pkgs; [
    (writeScriptBin "claude-statusline" (builtins.readFile ./claude-statusline.rb))
  ];

  programs.git.ignores = [
    "CLAUDE.local.md"
  ];
}