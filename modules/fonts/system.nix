{ pkgs, ... }:
{
  fonts.packages = [
    pkgs.nerd-fonts.fira-code
    pkgs.nerd-fonts.monaspace
    pkgs.nerd-fonts.victor-mono
  ];
}
