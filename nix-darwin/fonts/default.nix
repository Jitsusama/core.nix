{
  pkgs,
  ...
}:
{
  fonts.packages = with pkgs; [
    nerd-fonts.victor-mono
  ];
}

