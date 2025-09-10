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
      batgrep
      batwatch
    ];
  };

  programs.zsh.shellAliases = {
    cat = "bat";
  };
}
