# dig, for asking DNS servers about a name.
{ pkgs, ... }:
{
  home.packages = [ pkgs.dig ];
}
