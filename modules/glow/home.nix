{ pkgs, ... }:
{
  home.packages = [ pkgs.glow ];
  home.file."Library/Preferences/glow/glow.yml".source = ./glow.yml;
}
