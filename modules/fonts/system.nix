# The faces the theme draws with, for every account on the machine, on NixOS
# and a Mac alike.
{ pkgs, ... }:
{
  fonts.packages = [
    pkgs.mona-sans
    pkgs.nerd-fonts.fira-code
    pkgs.nerd-fonts.monaspace
    pkgs.nerd-fonts.victor-mono
  ];
}
