{ pkgs, ... }:
{
  programs.bat = {
    enable = true;
    config.style = "changes";
    extraPackages = [
      pkgs.bat-extras.batdiff
      pkgs.bat-extras.batman
      # batgrep has been broken since 2025-10-27.
      pkgs.bat-extras.batwatch
    ];
  };

  programs.zsh.shellAliases.cat = "bat";
}
