# nixfmt, the Nix formatter, on the command line.
{ pkgs, ... }:
{
  home.packages = [ pkgs.nixfmt ];
}
