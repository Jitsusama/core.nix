{
  pkgs,
  ...
}:
{
  home.packages = with pkgs; [
    (writeScriptBin "zdev" (builtins.readFile ./zdev.zsh))
  ];

  programs.zellij = {
    enable = true;
  };

  # Zellij configuration files
  xdg.configFile = {
    "zellij/config.kdl".source = ./config.kdl;
    "zellij/layouts/zdev.kdl".source = ./layouts/zdev.kdl;
    "zellij/layouts/default.kdl".source = ./layouts/default.kdl;
  };
}