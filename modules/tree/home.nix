# tree, for showing a folder's contents as a tree.
{ pkgs, ... }:
{
  home.packages = [ pkgs.tree ];
}
