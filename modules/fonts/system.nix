{ pkgs, ... }:
{
  fonts.packages = [
    pkgs.mona-sans
    pkgs.nerd-fonts.fira-code
    pkgs.nerd-fonts.monaspace
    pkgs.nerd-fonts.victor-mono
  ];
}
