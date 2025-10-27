{
  pkgs,
  ...
}:
{
  programs.bat = {
    enable = true;
    config = {
      style = "changes";
    };
    extraPackages = with pkgs.bat-extras; [
      batdiff
      batman
      # batgrep -- broken as of 2025/10/27
      batwatch
    ];
  };

  programs.zsh.shellAliases = {
    cat = "bat";
  };
}
