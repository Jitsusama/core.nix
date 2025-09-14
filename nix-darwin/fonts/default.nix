{
  pkgs,
  ...
}:
{
  fonts.packages = with pkgs; [
    nerd-fonts.victor-mono
    nerd-fonts.fira-code
  ];
}

